//! Local inference endpoint. **Sole call site for `ureq` and `serde_json`** (V72).
//!
//! Plain HTTP to a LAN box -- no TLS, no cloud, no key. Feature-gated so
//! `--no-default-features` leaves the deterministic core §C requires.

use std::io::BufRead;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use std::time::Instant;

/// Dump full prompts and replies. Off by default -- a prompt is ~1-3k tokens
/// and nobody wants that per step unless they are debugging one.
static VERBOSE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(on: bool) {
    VERBOSE.store(on, Ordering::Relaxed);
}

#[must_use]
pub fn verbose() -> bool {
    VERBOSE.load(Ordering::Relaxed)
}

// ---- pace: predicted cost, learned from what actually came back ----
//
// Seeded from measurements on the 24GB M5 Pro (§R R32): prefill ~940 tok/s at
// 28k, decode ~52 tok/s. A slower box -- the M1 Pro runs 284/24 -- corrects
// these within one call, which is the point of learning rather than pinning.
static PREFILL_TOK_S: AtomicU64 = AtomicU64::new(900);
static DECODE_TOK_S: AtomicU64 = AtomicU64::new(50);
static EXPECT_GEN: AtomicU64 = AtomicU64::new(1200);
/// Whether the last call's prefix was served from cache.
static LAST_CACHED: AtomicBool = AtomicBool::new(false);
/// Model load time on the last call -- non-zero means the endpoint was COLD.
static LAST_LOAD_MS: AtomicU64 = AtomicU64::new(0);

#[must_use]
pub fn last_cached() -> bool {
    LAST_CACHED.load(Ordering::Relaxed)
}

/// Where learned rates persist between processes. Without this, every
/// single-call invocation starts from the seed and learns nothing that lasts.
/// Load rates learned by earlier runs from the unified state. Silent when
/// absent -- a missing cache is a cold start, not an error.
pub fn load_pace() {
    let st = crate::state::State::load();
    for (key, cell) in [
        ("prefill", &PREFILL_TOK_S),
        ("decode", &DECODE_TOK_S),
        ("gen", &EXPECT_GEN),
    ] {
        if let Some(v) = st.get_u64("pace", key) {
            cell.store(v.max(1), Ordering::Relaxed);
        }
    }
}

/// Persist what this process learned, into the one state file every command
/// shares. Best-effort: a read-only tree must not fail a run over a cache.
pub fn save_pace() {
    let mut st = crate::state::State::load();
    save_pace_in(&mut st);
    st.save();
}

/// Persist the learned rates INTO a given store.
pub fn save_pace_in(st: &mut crate::state::State) {
    st.set(
        "pace",
        "prefill",
        PREFILL_TOK_S.load(Ordering::Relaxed).to_string(),
    );
    st.set(
        "pace",
        "decode",
        DECODE_TOK_S.load(Ordering::Relaxed).to_string(),
    );
    st.set(
        "pace",
        "gen",
        EXPECT_GEN.load(Ordering::Relaxed).to_string(),
    );
}

/// Observed prefill rate, and whether it implies the prefix was CACHED.
///
/// A cache hit prefills ~100x faster than cold (R14: 4.60s -> 0.04s), so the
/// two regimes are trivially separable and worth reporting: it tells you
/// whether the stable-prefix ordering (V76/V89) is actually paying.
#[must_use]
pub fn was_cached(prompt_tokens: u64, prefill_ms: u128) -> bool {
    if prompt_tokens == 0 {
        return false;
    }
    if prefill_ms == 0 {
        return true;
    }
    let observed = prompt_tokens as f64 / (prefill_ms as f64 / 1000.0);
    // ABSOLUTE bound, not relative to the learned rate. Measured cold prefill
    // spans 284 tok/s (M1 Pro @ 28k) to 1,519 (M5 Pro @ 7k), so 3,000 is
    // comfortably above any cold run and far below a cache hit (~100x). A
    // threshold relative to the learned rate rises WITH it and stops firing.
    observed > 3_000.0
}

/// A prediction, with its parts, so a wrong total says WHICH term was wrong.
#[derive(Debug, Clone, Copy)]
pub struct Eta {
    pub prefill_s: f64,
    pub decode_s: f64,
    pub gen_est: u64,
}

impl Eta {
    /// Cold: nothing cached, every prompt token prefilled.
    #[must_use]
    pub fn total_s(&self) -> f64 {
        self.prefill_s + self.decode_s
    }

    /// Hot: the prefix is already in the server's cache, so prefill is ~free.
    /// Measured at ~100x faster (R14), which in practice is the COMMON case --
    /// consecutive packs share their chain -- and is why a cold-only estimate
    /// reads 90% high. Both bounds are reported; the guards use the cold one,
    /// because over-estimating is the direction that does not false-alarm.
    #[must_use]
    pub fn cached_s(&self) -> f64 {
        self.decode_s
    }
    #[must_use]
    pub fn total(&self) -> Duration {
        Duration::from_secs_f64(self.total_s().max(1.0))
    }
}

/// Size bucket for a prompt. Prefill rate DEGRADES with length -- measured
/// 1,519 tok/s at 7k, 1,233 at 15k, 941 at 28k (R17) -- so one scalar rate is
/// wrong at both ends. Bucketing keeps the physics without fitting a curve.
#[must_use]
pub fn bucket(prompt_tokens: u64) -> &'static str {
    match prompt_tokens {
        0..=1_999 => "b0",
        2_000..=7_999 => "b2",
        8_000..=31_999 => "b8",
        _ => "b32",
    }
}

/// Append one observation, deduped by content so replaying an identical call
/// cannot double-count it. Capped, oldest dropped, so state stays bounded.
///
/// Retained raw rather than folded away: an average cannot be re-derived into
/// a median, a percentile, or a per-size fit, but samples can become all three.
pub fn record_obs_in(
    st: &mut crate::state::State,
    label: &str,
    row: &Telemetry,
) {
    const KEEP: usize = 200;
    let line = row.line(label);
    let key = crate::state::content_hash(line.as_bytes());
    if st.get("obs", &key).is_some() {
        return; // idempotent: this exact observation is already recorded
    }
    st.set("obs", &key, line);
    st.trim_kind("obs", KEEP);
}

/// One call's raw telemetry.
///
/// A struct because six positional arguments past `clippy.toml`'s limit of
/// four is where `prefill_ms` and `decode_ms` get swapped at a call site and
/// nothing complains -- both are `u128` and both are plausible.
#[derive(Debug, Clone, Copy)]
pub struct Telemetry {
    /// Prompt tokens the server counted.
    pub prompt_tok: u64,
    /// Milliseconds before the first token.
    pub prefill_ms: u128,
    /// Tokens generated.
    pub eval_tok: u64,
    /// Milliseconds generating them.
    pub decode_ms: u128,
    /// The model was loaded from disk for this call.
    pub cold: bool,
}

impl Telemetry {
    /// The retained row, as stored.
    #[must_use]
    pub fn line(&self, label: &str) -> String {
        format!(
            "{label} {} {} {} {} {}",
            self.prompt_tok,
            self.prefill_ms,
            self.eval_tok,
            self.decode_ms,
            u8::from(self.cold)
        )
    }
}

/// Retain one call's telemetry in the ambient store.
pub fn record_obs(label: &str, row: &Telemetry) {
    let mut st = crate::state::State::load();
    record_obs_in(&mut st, label, row);
    st.save();
}

/// Median prefill rate for prompts of this size, from retained samples.
/// `None` until at least two samples exist in the bucket -- one sample is not
/// a rate, and a cold-load call is excluded entirely.
#[must_use]
pub fn derived_prefill(prompt_tokens: u64) -> Option<f64> {
    let want = bucket(prompt_tokens);
    let st = crate::state::State::load();
    let mut rates: Vec<f64> = st
        .all("obs")
        .iter()
        .filter_map(|row| {
            let f: Vec<&str> = row.split(' ').collect();
            if f.len() < 6 || f[5] == "1" {
                return None;
            }
            let (tok, ms) =
                (f[1].parse::<u64>().ok()?, f[2].parse::<f64>().ok()?);
            if bucket(tok) != want || ms <= 0.0 {
                return None;
            }
            let r = tok as f64 / (ms / 1000.0);
            // A cache hit is not evidence about cold prefill (V9).
            if r > 3_000.0 { None } else { Some(r) }
        })
        .collect();
    if rates.len() < 2 {
        return None;
    }
    rates.sort_by(f64::total_cmp);
    Some(rates[rates.len() / 2])
}

/// What this call should cost, given what the endpoint has done so far.
#[must_use]
pub fn predict(prompt_tokens: u64) -> Eta {
    predict_for("", prompt_tokens)
}

/// Same, but using how much THIS KIND of step generates.
///
/// A judge replies in ~300 tokens; a red-test writes ~2,500 including the
/// reasoning gpt-oss hides. One global average under-predicts the big steps
/// badly enough to trip the 4x abort on a healthy run, which is exactly what
/// happened (B5).
#[must_use]
pub fn predict_for(label: &str, prompt_tokens: u64) -> Eta {
    // Derived per-size median first; the EWMA scalar is only the fallback.
    let pre = derived_prefill(prompt_tokens)
        .unwrap_or_else(|| PREFILL_TOK_S.load(Ordering::Relaxed).max(1) as f64);
    let dec = DECODE_TOK_S.load(Ordering::Relaxed).max(1) as f64;
    // `gen` is a reserved keyword from edition 2024; the state KEY stays "gen".
    let gen_tokens = if label.is_empty() {
        EXPECT_GEN.load(Ordering::Relaxed)
    } else {
        crate::state::State::load()
            .get_u64("gen", label)
            .unwrap_or_else(|| EXPECT_GEN.load(Ordering::Relaxed))
    };
    Eta {
        prefill_s: prompt_tokens as f64 / pre,
        decode_s: gen_tokens as f64 / dec,
        gen_est: gen_tokens,
    }
}

/// Raise a step kind's expected generation to at least `seen`.
///
/// Used when a call is killed: the observation is a lower bound, not a mean,
/// so it must not be averaged down with the estimate that was already wrong.
/// Raise the floor for `label` IN a given store.
///
/// The store is a parameter for `V19`: the suite must be HERMETIC, and
/// `.bbx-state` lives in the repo where every test that writes it changes
/// which branch the next test takes. `B8` is that flapping the ratchet and
/// blocking a commit that had changed nothing. `T13` carries the full fix.
pub fn raise_gen_floor_in(
    st: &mut crate::state::State,
    label: &str,
    seen: u64,
) {
    if label.is_empty() || seen == 0 {
        return;
    }
    if st.get_u64("gen", label).unwrap_or(0) < seen {
        st.set("gen", label, seen.to_string());
    }
}

/// A killed call still teaches: record what it managed as a FLOOR, so the
/// next estimate for this kind is not as low.
pub fn raise_gen_floor(label: &str, seen: u64) {
    let mut st = crate::state::State::load();
    raise_gen_floor_in(&mut st, label, seen);
    st.save();
}

/// Was the model cold -- loaded from disk for this call?
///
/// Ollama reports `load_duration`. A cold load is seconds of wall clock that
/// is not generation, so it must not be folded into the learned rates or the
/// endpoint looks permanently slower than it is.
#[must_use]
pub fn last_load_ms() -> u64 {
    LAST_LOAD_MS.load(Ordering::Relaxed)
}

/// Record how much this kind of step actually generated.
/// Fold one observed generation length into the model, IN a given store.
///
/// Exponential moving average at weight 1/4 -- fast enough to track a change
/// of endpoint, slow enough that one odd call does not swing the next
/// prediction. The FIRST observation for a label seeds from itself, so a new
/// kind of step is not dragged toward zero by a default.
pub fn observe_gen_in(
    st: &mut crate::state::State,
    label: &str,
    eval_tokens: u64,
) {
    if label.is_empty() || eval_tokens == 0 {
        return;
    }
    let prev = st.get_u64("gen", label).unwrap_or(eval_tokens);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let next = (prev as f64 * 0.75 + eval_tokens as f64 * 0.25) as u64;
    st.set("gen", label, next.to_string());
}

/// Record how much this kind of step actually generated.
pub fn observe_gen(label: &str, eval_tokens: u64) {
    let mut st = crate::state::State::load();
    observe_gen_in(&mut st, label, eval_tokens);
    st.save();
}

/// Fold one real reply into the model. Exponential moving average, weight 1/4
/// -- fast enough to track a change of endpoint, slow enough that one odd call
/// does not swing the next prediction.
pub fn observe(r: &Reply, prefill_ms: u128, decode_ms: u128) {
    let ewma = |cur: &AtomicU64, sample: f64| {
        if sample.is_finite() && sample > 0.0 {
            let old = cur.load(Ordering::Relaxed) as f64;
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            cur.store((old * 0.75 + sample * 0.25) as u64, Ordering::Relaxed);
        }
    };
    // A cache hit is not evidence about cold prefill speed -- folding it in
    // would collapse the learned rate and make every future eta meaningless.
    if prefill_ms > 0 && !was_cached(r.prompt_tokens, prefill_ms) {
        ewma(
            &PREFILL_TOK_S,
            r.prompt_tokens as f64 / (prefill_ms as f64 / 1000.0),
        );
    }
    if decode_ms > 0 {
        ewma(
            &DECODE_TOK_S,
            r.eval_tokens as f64 / (decode_ms as f64 / 1000.0),
        );
    }
    ewma(&EXPECT_GEN, r.eval_tokens as f64);
}

/// The seam. `generate` posts through this rather than calling `ureq`
/// directly, so a caller can substitute a transport that fails on demand.
///
/// Without it there is nothing to wrap: asked to add retry around real IO
/// with no seam, the model faked the transport instead (B7, V16).
pub trait Transport {
    /// POST `body` and return the response stream.
    ///
    /// # Errors
    /// Any transport-level failure, as a message naming the endpoint.
    fn post(
        &self,
        url: &str,
        body: &str,
        timeout: Duration,
    ) -> Result<Box<dyn BufRead + Send>, String>;
    /// Whether this transport's TIMINGS are real.
    ///
    /// A scripted double answers from memory in microseconds. Folding that
    /// into the pace model teaches it that calls take no time, and every
    /// later ETA and abort ceiling is computed from those rates -- so a
    /// suite full of doubles would leave the model predicting instant
    /// replies and aborting real ones early.
    ///
    /// Defaults to true, because a real endpoint is the normal case and a
    /// double is the thing that has to declare itself. `B9` is what the
    /// absence of this cost: every parser test was also a writer.
    fn timings_are_real(&self) -> bool {
        true
    }
}

/// The real one: `ureq` with TLS compiled in, so `BBX_ENDPOINT` may name an
/// `https://` host.
///
/// TLS is not decoration here. What this posts is the PROMPT -- which for
/// this tool means slices of your spec and your source, since feeding a
/// repository to a model is the entire job. Over `http://` that crosses the
/// network in cleartext, and `ollama` is on by DEFAULT, so it is the default
/// path rather than an opt-in one.
#[derive(Debug, Default, Clone, Copy)]
pub struct Http;

impl Transport for Http {
    fn post(
        &self,
        url: &str,
        body: &str,
        timeout: Duration,
    ) -> Result<Box<dyn BufRead + Send>, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_recv_response(Some(timeout))
            .build()
            .into();
        let resp = agent
            .post(url)
            .content_type("application/json")
            .send(body)
            .map_err(|e| format!("{url}: {e}"))?;
        Ok(Box::new(std::io::BufReader::new(
            resp.into_body().into_reader(),
        )))
    }
}

/// One round-trip, with what it cost. A call without its cost is not evidence.
#[derive(Debug, Clone)]
pub struct Reply {
    pub text: String,
    /// Reasoning the model emitted but did not return as output. Measured at
    /// 64% of frames on gpt-oss:20b -- invisible, but it is where the latency
    /// goes, and `-v` is the only way to see why a judge decided as it did.
    pub thinking: String,
    /// Prompt tokens the SERVER counted -- the real number, not our estimate.
    pub prompt_tokens: u64,
    pub eval_tokens: u64,
    pub ms: u128,
}

fn endpoint() -> String {
    std::env::var("BBX_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:11434".into())
}

fn model() -> String {
    std::env::var("BBX_MODEL").unwrap_or_else(|_| "gpt-oss:20b".into())
}

/// Context window to request. Passed PER REQUEST, never set globally -- a
/// global value makes every model allocate a full cache whether it needs one
/// or not, and we know what we are asking for.
fn num_ctx() -> u64 {
    std::env::var("BBX_NUM_CTX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(131_072)
}

/// How to sample. Carried explicitly because it changes what a call MEANS:
/// at temperature 0 this model is deterministic (V17, 3/3 identical), so two
/// calls with the same prompt are one call run twice.
///
/// A seed is always sent, so a diverse candidate is still REPRODUCIBLE.
/// Diversity without a seed would make a failure impossible to re-examine,
/// and "it was different that time" is the explanation V17 exists to refuse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sampling {
    pub temperature: f64,
    pub seed: u64,
}

impl Sampling {
    /// Temperature 0. Every step of the loop uses this unless asked otherwise.
    pub const DETERMINISTIC: Self = Self {
        temperature: 0.0,
        seed: 0,
    };

    /// The `k`th competing candidate. Candidate 0 IS the deterministic call --
    /// so asking for one candidate is exactly today's behaviour, not a
    /// differently-sampled approximation of it.
    #[must_use]
    pub fn candidate(k: usize) -> Self {
        if k == 0 {
            Self::DETERMINISTIC
        } else {
            Self {
                temperature: 0.6,
                seed: k as u64,
            }
        }
    }
}

/// Generate, STREAMING. Progress is reported as it arrives, because a
/// 90-second silent wait is indistinguishable from a hang -- V48 applied to
/// a running process rather than to a report.
///
/// `on_chunk` receives each token as the server emits it.
///
/// # Errors
/// Transport failure or a response that is not the expected JSON shape. A
/// miss is an error, never an empty success (V20).
pub fn generate_with(
    prompt: &str,
    eta: Eta,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<Reply, String> {
    generate_labelled(prompt, "", eta, on_chunk)
}

/// Escalation ladder, as multiples of the estimate.
///
/// Widened from 1/2/3/4 after a healthy run was killed at 42s: the estimate
/// was 11s because gen_est knew nothing about the 2,614 REASONING tokens
/// gpt-oss emits before its first output token. A ladder tight enough to abort
/// a working call also prevents the measurement that would fix the estimate,
/// so the notices stay early and the kill moves far out (B6).
const NOTICE_1: f64 = 1.0;
const NOTICE_2: f64 = 2.0;
const WARN: f64 = 5.0;
const ABORT: f64 = 10.0;

/// As [`generate_with`], but records what it learned against a step KIND --
/// including on abort, so a killed call still teaches the next estimate.
///
/// # Errors
/// See [`generate_with`].
pub fn generate_labelled(
    prompt: &str,
    label: &str,
    eta: Eta,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<Reply, String> {
    generate_via(&Http, prompt, label, Sampling::DETERMINISTIC, eta, on_chunk)
}

/// As [`generate_labelled`], with explicit sampling.
///
/// # Errors
/// See [`generate_labelled`].
pub fn generate_sampled(
    prompt: &str,
    label: &str,
    sampling: Sampling,
    eta: Eta,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<Reply, String> {
    generate_via(&Http, prompt, label, sampling, eta, on_chunk)
}

/// As [`generate_labelled`], but posting through `transport`.
///
/// # Errors
/// See [`generate_labelled`].
pub fn stream(
    transport: &dyn Transport,
    prompt: &str,
    label: &str,
    sampling: Sampling,
    eta: Eta,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<Streamed, String> {
    let budget = eta.total();
    // Hard ceiling: 4x the prediction. Also the socket read timeout, so a
    // server that accepts and then says nothing fails here rather than hanging
    // forever -- silence is the failure mode a streaming client must bound.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let hard = budget.mul_f64(ABORT);
    let body = serde_json::json!({
        "model": model(),
        "prompt": prompt,
        "stream": true,
        "options": { "num_ctx": num_ctx(), "temperature": sampling.temperature,
                     "seed": sampling.seed },
    });
    let started = Instant::now();
    let stream = transport.post(
        &format!("{}/api/generate", endpoint()),
        &body.to_string(),
        hard,
    )?;

    let mut text = String::new();
    let mut thinking = String::new();
    let mut warned = 0u8;
    let mut first_chunk: Option<Instant> = None;
    let (mut prompt_tokens, mut eval_tokens) = (0, 0);
    for line in stream.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if let Some(th) = v["thinking"].as_str()
            && !th.is_empty()
        {
            thinking.push_str(th);
            if first_chunk.is_none() {
                first_chunk = Some(Instant::now())
            }
            on_chunk("");
        }
        if let Some(chunk) = v["response"].as_str() {
            if first_chunk.is_none() && !chunk.is_empty() {
                first_chunk = Some(Instant::now());
            }
            text.push_str(chunk);
            on_chunk(chunk);
        }
        // Escalate against the prediction: notice, notice again, warn, abort.
        // A prediction nobody checks is a decoration.
        let over = started.elapsed().as_secs_f64() / budget.as_secs_f64();
        let level = if over >= ABORT {
            4
        } else if over >= WARN {
            3
        } else if over >= NOTICE_2 {
            2
        } else if over >= NOTICE_1 {
            1
        } else {
            0
        };
        if level > warned {
            warned = level;
            let el = started.elapsed().as_secs_f64();
            match level {
                1 => eprintln!(
                    "\n  [pace] {el:.0}s: past the {:.0}s estimate, still streaming",
                    budget.as_secs_f64()
                ),
                2 => eprintln!(
                    "\n  [pace] {el:.0}s: 2x the estimate -- the endpoint is slower than this run assumed"
                ),
                3 => eprintln!(
                    "\n  [pace] WARNING {el:.0}s: {WARN:.0}x the estimate. Hard stop at {:.0}s.",
                    hard.as_secs_f64()
                ),
                _ => {
                    // A killed call still teaches: record what it managed as a
                    // FLOOR, so the next estimate for this kind is not as low.
                    let seen = (text.len() + thinking.len()) as u64 / 4;
                    raise_gen_floor(label, seen);
                    return Err(format!(
                        "aborted after {el:.0}s -- 4x the {:.0}s estimate. \
                         {} output + {} reasoning tokens so far (recorded as a floor \
                         for `{label}`). Endpoint {} may be overloaded, or \
                         BBX_NUM_CTX too large for its memory.",
                        budget.as_secs_f64(),
                        text.len() / 4,
                        thinking.len() / 4,
                        endpoint()
                    ));
                }
            }
        }
        // Counts arrive only on the final frame.
        if v["done"].as_bool().unwrap_or(false) {
            prompt_tokens = v["prompt_eval_count"].as_u64().unwrap_or(0);
            eval_tokens = v["eval_count"].as_u64().unwrap_or(0);
            LAST_LOAD_MS.store(
                v["load_duration"].as_u64().unwrap_or(0) / 1_000_000,
                Ordering::Relaxed,
            );
        }
    }
    let reply = Reply {
        text,
        thinking,
        prompt_tokens,
        eval_tokens,
        ms: started.elapsed().as_millis(),
    };
    Ok(Streamed {
        reply,
        started,
        first_chunk,
    })
}

/// One reply, with the timing needed to learn from it.
pub struct Streamed {
    /// What the model said.
    pub reply: Reply,
    /// When the request went out.
    pub started: Instant,
    /// When the first token came back, if one did.
    pub first_chunk: Option<Instant>,
}

/// Load the model BEFORE the first measured call.
///
/// The first call after an endpoint wakes includes a disk load -- measured at
/// ~11s for `gpt-oss:20b`'s 12.7GB -- while the eta is learned from WARM
/// calls. The abort ceiling is 4x that eta, so the first step of a run is
/// killed before it can finish: T13's first re-measure died at 96s for
/// exactly this, and the second did too because the box had unloaded again
/// between runs.
///
/// `V15` already keeps a cold call out of rate LEARNING. This keeps it out of
/// the CEILING as well, which is the half that discards work.
///
/// One tiny call, discarded, through `stream` so it persists nothing (`V20`).
/// Skipped for a double, whose timings are fiction and which loads nothing.
pub fn prewarm(t: &dyn Transport) {
    if !t.timings_are_real() {
        return;
    }
    let eta = Eta {
        prefill_s: 60.0,
        decode_s: 60.0,
        gen_est: 1,
    };
    // An empty label: this call is not a step and must not key the pace model.
    let _ = stream(t, "ok", "", Sampling::DETERMINISTIC, eta, &mut |_| {});
    let ms = last_load_ms();
    if ms > 500 {
        eprintln!(
            "  warm: model loaded from disk in {ms}ms -- not charged to the run"
        );
    }
}

/// Stream a reply AND fold it into the pace model.
///
/// The production path: [`stream`] measures, [`learn_from`] persists, and
/// only this does both. A test that wants the parser calls `stream` and
/// writes nothing (`V20`, and `B9` is what it cost not to have the split).
///
/// # Errors
/// Transport failure, a malformed frame, or an abort past the ceiling.
pub fn generate_via(
    transport: &dyn Transport,
    prompt: &str,
    label: &str,
    sampling: Sampling,
    eta: Eta,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<Reply, String> {
    let s = stream(transport, prompt, label, sampling, eta, on_chunk)?;
    if transport.timings_are_real() {
        learn_from(&s.reply, label, s.started, s.first_chunk);
    }
    Ok(s.reply)
}

/// Fold one real reply into the pace model, and PERSIST it.
///
/// Separate from [`generate_via`] because a function that ends by writing
/// what it just observed cannot be exercised without mutating shared state:
/// every test of the STREAM PARSER became a writer of the pace model, and
/// `B9` is that measured -- `cargo test --lib ollama::` alone moved
/// `pace gen` from 5 to 517. `V20` is the rule, and it is what makes `V19`'s
/// hermetic suite enforceable rather than aspirational.
///
/// Production calls this immediately after `generate_via`; a test that only
/// wants the parser calls neither.
pub fn learn_from(
    reply: &Reply,
    label: &str,
    started: Instant,
    first_chunk: Option<Instant>,
) {
    let mut st = crate::state::State::load();
    learn_from_in(&mut st, reply, label, timing(reply, started, first_chunk));
    st.save();
}

/// Split the observed time at the first token: everything before it is
/// prefill, everything after is decode. Two rates, learned separately,
/// because they scale differently (R16/R17). Model load is subtracted -- it
/// is disk time, not prefill.
#[must_use]
pub fn timing(
    reply: &Reply,
    started: Instant,
    first_chunk: Option<Instant>,
) -> Telemetry {
    let load_ms = u128::from(LAST_LOAD_MS.load(Ordering::Relaxed));
    let pre_ms = first_chunk
        .map_or(reply.ms, |t| (t - started).as_millis())
        .saturating_sub(load_ms);
    Telemetry {
        prompt_tok: reply.prompt_tokens,
        prefill_ms: pre_ms,
        eval_tok: reply.eval_tokens,
        decode_ms: reply.ms.saturating_sub(pre_ms),
        cold: load_ms > 500,
    }
}

/// Fold one reply into the pace model IN a given store.
///
/// `V20`: measuring and learning are separate calls, and the store is a
/// parameter so learning can be exercised without writing the ambient file
/// (`V19`, `B9`).
pub fn learn_from_in(
    st: &mut crate::state::State,
    reply: &Reply,
    label: &str,
    t: Telemetry,
) {
    observe(reply, t.prefill_ms, t.decode_ms);
    LAST_CACHED.store(
        was_cached(reply.prompt_tokens, t.prefill_ms),
        Ordering::Relaxed,
    );
    record_obs_in(st, label, &t);
    save_pace_in(st);
}

/// Generate with no progress reporting.
///
/// # Errors
/// See [`generate_with`].
pub fn generate(prompt: &str) -> Result<Reply, String> {
    generate_with(prompt, predict(0), &mut |_| {})
}

/// Pull code out of whatever prose the model wrapped it in. Longest fenced
/// block wins; unfenced text is taken whole.
#[must_use]
pub fn rust_block(text: &str) -> String {
    let mut best = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("```") {
        let after = &rest[open + 3..];
        let start = after.find('\n').map_or(0, |i| i + 1);
        let Some(close) = after[start..].find("```") else {
            break;
        };
        let block = &after[start..start + close];
        if block.len() > best.len() {
            best = block.to_string();
        }
        rest = &after[start + close..];
    }
    if best.is_empty() {
        text.trim().to_string()
    } else {
        best.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A store of its own -- `V19`, the suite must be HERMETIC.
    fn store(tag: &str) -> crate::state::State {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        crate::state::State::at(
            std::env::temp_dir()
                .join(format!("bbx-pace-{tag}-{}-{n}", std::process::id())),
        )
    }

    #[test]
    fn timing_splits_prefill_from_decode_at_the_first_token() {
        // Two rates learned separately because they scale differently
        // (R16/R17). Folding them into one would make a fat prompt look like
        // slow generation and every later eta would be wrong in both halves.
        let r = Reply {
            text: String::new(),
            thinking: String::new(),
            prompt_tokens: 900,
            eval_tokens: 100,
            ms: 3_000,
        };
        let started = Instant::now();
        let first = started.checked_add(Duration::from_millis(1_000));
        let t = timing(&r, started, first);
        assert_eq!(t.prefill_ms, 1_000, "everything before the first token");
        assert_eq!(t.decode_ms, 2_000, "and everything after it");
        assert_eq!(t.prompt_tok, 900);
        assert_eq!(t.eval_tok, 100);
    }

    #[test]
    fn a_reply_with_no_token_at_all_is_all_prefill() {
        // No first chunk means nothing ever came back, so the whole elapsed
        // time was spent waiting -- calling any of it `decode` would teach
        // the model a generation rate from a call that generated nothing.
        let r = Reply {
            text: String::new(),
            thinking: String::new(),
            prompt_tokens: 10,
            eval_tokens: 0,
            ms: 500,
        };
        let t = timing(&r, Instant::now(), None);
        assert_eq!(t.prefill_ms, 500);
        assert_eq!(t.decode_ms, 0);
    }

    #[test]
    fn a_telemetry_row_round_trips_its_fields_in_order() {
        // The row is positional and six wide. `prefill_ms` and `decode_ms`
        // are both `u128` and both plausible in either slot, which is why
        // they travel in a struct now rather than as bare arguments.
        let t = Telemetry {
            prompt_tok: 1,
            prefill_ms: 2,
            eval_tok: 3,
            decode_ms: 4,
            cold: true,
        };
        assert_eq!(t.line("lbl"), "lbl 1 2 3 4 1");
    }

    #[test]
    fn learning_from_a_reply_retains_it_and_persists_the_rates() {
        // The production path's persisting half, exercised in a store of its
        // own. Before the split this could only run by writing the ambient
        // `.bbx-state`, which is `B9` -- so the only test of it was every
        // other test, by accident.
        let mut st = store("learn");
        let r = Reply {
            text: "x".into(),
            thinking: String::new(),
            prompt_tokens: 900,
            eval_tokens: 100,
            ms: 2_000,
        };
        let t = Telemetry {
            prompt_tok: 900,
            prefill_ms: 1_000,
            eval_tok: 100,
            decode_ms: 1_000,
            cold: false,
        };
        learn_from_in(&mut st, &r, "1 test", t);
        assert!(!st.all("obs").is_empty(), "the observation is retained");
        assert!(st.get("pace", "prefill").is_some(), "rates persisted");
    }

    #[test]
    fn the_same_observation_twice_is_retained_once() {
        // Idempotent by content hash. A retry that re-reports the same call
        // must not double-weight it in the median.
        let mut st = store("idem");
        let t = Telemetry {
            prompt_tok: 7,
            prefill_ms: 3,
            eval_tok: 2,
            decode_ms: 4,
            cold: false,
        };
        record_obs_in(&mut st, "1 test", &t);
        let after_one = st.all("obs").len();
        record_obs_in(&mut st, "1 test", &t);
        assert_eq!(st.all("obs").len(), after_one, "recorded once, not twice");
    }

    #[test]
    fn a_scripted_transport_declares_its_timings_fake() {
        // The whole guard rests on this: a double answers from memory in
        // microseconds, and folding that into the model would teach it that
        // calls take no time. `Http` says true by default; a double says no.
        assert!(Http.timings_are_real(), "a real endpoint is real");
        assert!(
            !Canned(String::new()).timings_are_real(),
            "a double must declare itself -- B9 is the absence of this"
        );
    }

    /// Claims REAL timings, so `prewarm` does not skip it, and counts the
    /// calls it is asked to make.
    ///
    /// The only double here that does NOT override `timings_are_real`: the
    /// point is to get PAST the guard and exercise the body.
    struct WarmSpy {
        calls: std::cell::Cell<usize>,
    }

    impl Transport for WarmSpy {
        fn post(
            &self,
            _u: &str,
            _b: &str,
            _t: Duration,
        ) -> Result<Box<dyn BufRead + Send>, String> {
            self.calls.set(self.calls.get().saturating_add(1));
            Ok(Box::new(std::io::Cursor::new(
                "{\"response\":\"ok\",\"done\":true}\n".as_bytes().to_vec(),
            )))
        }
    }

    #[test]
    fn prewarming_makes_exactly_one_discarded_call() {
        // ONE: zero would warm nothing and leave the load inside step 1,
        // which is what V21 exists to stop; two would pay for a second
        // round-trip on every run for no further benefit.
        let t = WarmSpy {
            calls: std::cell::Cell::new(0),
        };
        prewarm(&t);
        assert_eq!(t.calls.get(), 1, "one call, and only one");
    }

    #[test]
    fn prewarming_learns_nothing_from_the_call_it_makes() {
        // V19/V20: a warm-up is not a measurement. It goes through `stream`,
        // so no `gen` row is keyed and no pace row is written -- otherwise
        // the first run after a cold start would teach the model that a
        // one-token reply is what a step costs.
        let (_f, p) = scratch_state("warm");
        let before = crate::state::State::at(&p).all("gen").len();
        prewarm(&WarmSpy {
            calls: std::cell::Cell::new(0),
        });
        assert_eq!(
            crate::state::State::at(&p).all("gen").len(),
            before,
            "a warm-up records nothing"
        );
    }

    /// A state file of its own (`src/review:V6`).
    fn scratch_state(tag: &str) -> ((), std::path::PathBuf) {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        (
            (),
            std::env::temp_dir()
                .join(format!("bbx-warm-{tag}-{}-{n}", std::process::id())),
        )
    }

    #[test]
    fn prewarming_a_double_costs_nothing_and_asks_it_nothing() {
        // A scripted transport loads no model and its timings are fiction
        // (V20), so warming one would burn a scripted reply and shift every
        // later call in the script by one -- the run would then be driven by
        // the wrong answers.
        let t = Canned("{\"response\":\"x\",\"done\":true}\n".into());
        prewarm(&t);
        assert!(drain(&t).is_ok(), "the double's first reply is still there");
    }

    #[test]
    fn a_floor_only_ever_rises() {
        // A killed call records what it MANAGED as a floor, so the next
        // estimate for that kind is not as low. Letting it fall would mean a
        // short abort teaching the model that this step is small, and the
        // next run would abort even sooner -- a ratchet running backwards.
        let mut st = store("floor");
        raise_gen_floor_in(&mut st, "4 repair-1", 500);
        assert_eq!(st.get_u64("gen", "4 repair-1"), Some(500));
        raise_gen_floor_in(&mut st, "4 repair-1", 200);
        assert_eq!(
            st.get_u64("gen", "4 repair-1"),
            Some(500),
            "a LOWER observation must not lower the floor"
        );
        raise_gen_floor_in(&mut st, "4 repair-1", 900);
        assert_eq!(st.get_u64("gen", "4 repair-1"), Some(900), "higher wins");
    }

    #[test]
    fn an_empty_label_or_a_zero_count_records_nothing() {
        // Both are "no information". Writing them would seed the model with
        // a zero it then averages against every real observation.
        let mut st = store("guard");
        raise_gen_floor_in(&mut st, "", 500);
        raise_gen_floor_in(&mut st, "label", 0);
        observe_gen_in(&mut st, "", 500);
        observe_gen_in(&mut st, "label", 0);
        assert_eq!(st.get_u64("gen", ""), None);
        assert_eq!(st.get_u64("gen", "label"), None);
    }

    #[test]
    fn the_first_observation_seeds_from_itself_and_later_ones_average() {
        // Seeding from a default would drag a brand-new kind of step toward
        // that default; seeding from itself means one call is believed
        // exactly once, and the weight-1/4 average takes over after that.
        let mut st = store("ewma");
        observe_gen_in(&mut st, "1 test", 400);
        assert_eq!(
            st.get_u64("gen", "1 test"),
            Some(400),
            "the first observation is the estimate"
        );
        // 400 * 0.75 + 800 * 0.25 = 500.
        observe_gen_in(&mut st, "1 test", 800);
        assert_eq!(st.get_u64("gen", "1 test"), Some(500));
        // A single odd call must MOVE the estimate without owning it.
        assert!(
            st.get_u64("gen", "1 test").is_some_and(|v| v < 800),
            "weight 1/4: one call does not swing the next prediction"
        );
    }

    /// Fails `fail_times`, then answers. The thing B7 had no way to build.
    struct Flaky {
        fail_times: std::cell::Cell<u32>,
        body: String,
    }

    impl Transport for Flaky {
        fn timings_are_real(&self) -> bool {
            false
        }

        fn post(
            &self,
            url: &str,
            _b: &str,
            _t: Duration,
        ) -> Result<Box<dyn BufRead + Send>, String> {
            let left = self.fail_times.get();
            if left > 0 {
                self.fail_times.set(left - 1);
                return Err(format!("{url}: simulated transport failure"));
            }
            Ok(Box::new(std::io::Cursor::new(
                self.body.clone().into_bytes(),
            )))
        }
    }

    /// A transport that answers with a canned NDJSON stream.
    struct Canned(String);

    impl Transport for Canned {
        fn timings_are_real(&self) -> bool {
            false
        }

        fn post(
            &self,
            _u: &str,
            _b: &str,
            _t: Duration,
        ) -> Result<Box<dyn BufRead + Send>, String> {
            Ok(Box::new(std::io::Cursor::new(self.0.clone().into_bytes())))
        }
    }

    /// Generous, so the pace escalation never fires and the test measures
    /// parsing rather than wall clock.
    fn slow_eta() -> Eta {
        Eta {
            prefill_s: 600.0,
            decode_s: 600.0,
            gen_est: 100,
        }
    }

    /// Drive the PARSER only. `stream` measures and persists nothing, so a
    /// test of the stream format is not also a writer of the pace model
    /// (`V20`, `B9`).
    fn drain(t: &dyn Transport) -> Result<Reply, String> {
        stream(
            t,
            "p",
            "test",
            Sampling::candidate(0),
            slow_eta(),
            &mut |_| {},
        )
        .map(|s| s.reply)
    }

    #[test]
    fn a_streamed_reply_is_assembled_from_its_chunks() {
        // The counts arrive ONLY on the final frame, so a parser that stopped
        // at the first `done` field or ignored the tail would report zero
        // tokens for a real call -- and `was_cached` and every pace estimate
        // are computed from them.
        let body = concat!(
            "{\"response\":\"pub fn \",\"done\":false}\n",
            "\n",
            "{\"response\":\"x() {}\",\"done\":false}\n",
            "{\"response\":\"\",\"done\":true,\"prompt_eval_count\":1234,\
             \"eval_count\":56,\"load_duration\":0}\n"
        );
        let r = drain(&Canned(body.into()));
        let Ok(r) = r else {
            unreachable!("canned stream must parse")
        };
        assert_eq!(r.text, "pub fn x() {}", "chunks concatenate in order");
        assert_eq!(r.prompt_tokens, 1234, "counts come off the final frame");
        assert_eq!(r.eval_tokens, 56);
    }

    #[test]
    fn reasoning_is_kept_apart_from_the_answer() {
        // `thinking` must never leak into `text`: the answer is fenced code
        // that gets compiled, and reasoning prose in it would not build.
        let body = concat!(
            "{\"thinking\":\"let me think\",\"done\":false}\n",
            "{\"response\":\"fn a(){}\",\"done\":false}\n",
            "{\"response\":\"\",\"done\":true,\"eval_count\":2}\n"
        );
        let Ok(r) = drain(&Canned(body.into())) else {
            unreachable!("canned stream must parse")
        };
        assert_eq!(r.text, "fn a(){}");
        assert_eq!(r.thinking, "let me think");
    }

    #[test]
    fn a_transport_failure_is_an_error_and_never_an_empty_reply() {
        // `assay:V1`: a call that did not RUN says nothing. An empty `Reply`
        // here would be graded as a wrong answer and read as the model
        // failing, which is `assay:B1` costing forty minutes.
        let t = Flaky {
            fail_times: std::cell::Cell::new(1),
            body: String::new(),
        };
        assert!(drain(&t).is_err(), "a dead transport is an ERROR");
    }

    #[test]
    fn a_malformed_frame_is_an_error_and_not_a_silent_skip() {
        // A line that is not JSON means the stream is not what this client
        // thinks it is. Skipping it would silently truncate the answer.
        let body = "{\"response\":\"a\",\"done\":false}\nnot json at all\n";
        assert!(drain(&Canned(body.into())).is_err());
    }

    #[test]
    fn the_longest_fenced_block_wins_and_a_bare_reply_survives() {
        // The model prefixes prose and sometimes emits two blocks -- a short
        // example and the real answer. Taking the FIRST would compile the
        // example. Taking none when unfenced would discard a correct reply.
        assert_eq!(rust_block("no fence here"), "no fence here");
        assert_eq!(rust_block("pre\n```rust\nfn a(){}\n```\npost"), "fn a(){}");
        let two =
            "```\nfn a(){}\n```\ntext\n```rust\nfn long(){ let x = 1; }\n```";
        assert_eq!(rust_block(two), "fn long(){ let x = 1; }");
        // An unterminated fence is not a block: fall back to the whole text
        // rather than returning nothing.
        assert_eq!(rust_block("```rust\nfn a(){}"), "```rust\nfn a(){}");
    }

    #[test]
    fn a_transport_can_be_substituted_and_made_to_fail() {
        let t = Flaky {
            fail_times: std::cell::Cell::new(1),
            body: String::new(),
        };
        assert!(
            t.post("u", "b", Duration::from_secs(1)).is_err(),
            "first call fails"
        );
        assert!(
            t.post("u", "b", Duration::from_secs(1)).is_ok(),
            "then succeeds"
        );
    }

    #[test]
    fn generate_via_reads_a_substituted_stream() {
        let body = "{\"response\":\"hi\",\"done\":false}\n\
                    {\"response\":\"\",\"done\":true,\"prompt_eval_count\":7,\"eval_count\":2}\n";
        let t = Flaky {
            fail_times: std::cell::Cell::new(0),
            body: body.into(),
        };
        let r = generate_via(
            &t,
            "p",
            "test",
            Sampling::DETERMINISTIC,
            predict(10),
            &mut |_| {},
        )
        .unwrap();
        assert_eq!(r.text, "hi");
        assert_eq!((r.prompt_tokens, r.eval_tokens), (7, 2));
    }

    /// Records what was actually POSTed. The seam exists so a claim about the
    /// request can be checked without an endpoint.
    struct Spy(std::cell::RefCell<String>);
    impl Transport for Spy {
        fn timings_are_real(&self) -> bool {
            false
        }

        fn post(
            &self,
            _u: &str,
            b: &str,
            _t: Duration,
        ) -> Result<Box<dyn BufRead + Send>, String> {
            self.0.replace(b.to_string());
            Ok(Box::new(std::io::Cursor::new(
                "{\"response\":\"x\",\"done\":true}\n".as_bytes().to_vec(),
            )))
        }
    }

    fn posted(s: Sampling) -> serde_json::Value {
        let spy = Spy(std::cell::RefCell::new(String::new()));
        generate_via(&spy, "p", "test", s, predict(10), &mut |_| {}).unwrap();
        let body = spy.0.borrow().clone();
        serde_json::from_str(&body).unwrap()
    }

    #[test]
    fn sampling_reaches_the_request_and_candidates_differ() {
        let zero = posted(Sampling::candidate(0));
        assert_eq!(
            zero["options"]["temperature"], 0.0,
            "candidate 0 must be today's deterministic call, unchanged"
        );

        let two = posted(Sampling::candidate(2));
        assert!(
            two["options"]["temperature"].as_f64().unwrap() > 0.0,
            "a competing candidate at temperature 0 is the same call twice (V17)"
        );
        assert_eq!(
            two["options"]["seed"], 2,
            "diverse but REPRODUCIBLE -- an unseeded failure cannot be re-examined"
        );
        assert_ne!(zero["options"]["seed"], two["options"]["seed"]);
    }

    #[test]
    fn takes_the_longest_fenced_block() {
        let t = "blah\n```rust\nfn a() {}\n```\nmore\n```rust\nfn bb() { todo!() }\n```\n";
        assert_eq!(rust_block(t), "fn bb() { todo!() }");
    }

    #[test]
    fn unfenced_text_survives_whole() {
        assert_eq!(rust_block("  fn a() {}  "), "fn a() {}");
    }

    #[test]
    fn unterminated_fence_does_not_hang_or_panic() {
        assert_eq!(rust_block("```rust\nfn a() {}"), "```rust\nfn a() {}");
    }
}

#[cfg(test)]
mod pace_tests {
    use super::*;

    #[test]
    fn a_bucket_is_a_size_band_not_a_scalar() {
        // `.:R17` -- prefill rate falls with size (1,519 tok/s at 7k, 941 at
        // 28k), so one learned scalar under-predicts the big packs badly
        // enough to trip the abort on a healthy run.
        const BANDS: [(u64, &str); 7] = [
            (0, "b0"),
            (1_999, "b0"),
            (2_000, "b2"),
            (7_999, "b2"),
            (8_000, "b8"),
            (31_999, "b8"),
            (32_000, "b32"),
        ];
        for (n, want) in BANDS {
            assert_eq!(bucket(n), want, "{n} belongs in {want}");
        }
    }

    #[test]
    fn a_cache_hit_is_an_absolute_rate_not_a_multiple_of_the_learned_one() {
        // The threshold is 3,000 tok/s flat. Relative to the LEARNED rate it
        // would rise as the rate rose and stop catching hits -- which is what
        // B4 in this node records.
        assert!(was_cached(10_000, 1_000), "10k tok/s is a hit");
        assert!(!was_cached(1_000, 1_000), "1k tok/s is a cold run");
        assert!(
            !was_cached(3_000, 1_000),
            "exactly 3k is NOT over the bound"
        );
        // Degenerate inputs must not read as evidence either way.
        assert!(!was_cached(0, 500), "no prompt is not a cache hit");
        assert!(was_cached(500, 0), "no measurable prefill is a hit");
    }

    #[test]
    fn verbose_is_a_mode_that_can_be_turned_off_again() {
        let before = verbose();
        set_verbose(true);
        assert!(verbose());
        set_verbose(false);
        assert!(!verbose());
        set_verbose(before);
    }

    #[test]
    fn an_eta_is_prefill_plus_decode_and_says_what_it_expects() {
        // A prediction with no gen estimate cannot bound anything, and the
        // abort ladder is a multiple of this number.
        let e = predict(10_000);
        assert!(e.prefill_s > 0.0, "prefill must be predicted: {e:?}");
        assert!(e.decode_s > 0.0, "decode must be predicted: {e:?}");
        assert!(e.total() > std::time::Duration::ZERO);
        assert!(e.gen_est > 0, "an eta of zero tokens bounds nothing");
    }

    #[test]
    fn a_bigger_prompt_never_predicts_a_shorter_prefill() {
        // Monotonicity is the one property an eta must have: R17 measured the
        // rate FALLING with size, so a bigger pack predicting less time would
        // be worse than no prediction.
        let small = predict(1_000);
        let big = predict(30_000);
        assert!(
            big.prefill_s >= small.prefill_s,
            "{big:?} must not be quicker than {small:?}"
        );
    }
}
