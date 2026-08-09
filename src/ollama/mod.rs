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
    for (key, cell) in [("prefill", &PREFILL_TOK_S), ("decode", &DECODE_TOK_S), ("gen", &EXPECT_GEN)] {
        if let Some(v) = st.get_u64("pace", key) {
            cell.store(v.max(1), Ordering::Relaxed);
        }
    }
}

/// Persist what this process learned, into the one state file every command
/// shares. Best-effort: a read-only tree must not fail a run over a cache.
pub fn save_pace() {
    let mut st = crate::state::State::load();
    st.set("pace", "prefill", PREFILL_TOK_S.load(Ordering::Relaxed).to_string());
    st.set("pace", "decode", DECODE_TOK_S.load(Ordering::Relaxed).to_string());
    st.set("pace", "gen", EXPECT_GEN.load(Ordering::Relaxed).to_string());
    st.save();
}

/// Observed prefill rate, and whether it implies the prefix was CACHED.
///
/// A cache hit prefills ~100x faster than cold (R14: 4.60s -> 0.04s), so the
/// two regimes are trivially separable and worth reporting: it tells you
/// whether the stable-prefix ordering (V76/V89) is actually paying.
#[must_use]
pub fn was_cached(prompt_tokens: u64, prefill_ms: u128) -> bool {
    if prompt_tokens == 0 { return false }
    if prefill_ms == 0 { return true }
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
pub fn record_obs(label: &str, prompt_tok: u64, prefill_ms: u128, eval_tok: u64,
                  decode_ms: u128, cold: bool) {
    const KEEP: usize = 200;
    let row = format!("{label} {prompt_tok} {prefill_ms} {eval_tok} {decode_ms} {}",
                      u8::from(cold));
    let mut st = crate::state::State::load();
    let key = crate::state::content_hash(row.as_bytes());
    if st.get("obs", &key).is_some() {
        return; // idempotent: this exact observation is already recorded
    }
    st.set("obs", &key, row);
    st.trim_kind("obs", KEEP);
    st.save();
}

/// Median prefill rate for prompts of this size, from retained samples.
/// `None` until at least two samples exist in the bucket -- one sample is not
/// a rate, and a cold-load call is excluded entirely.
#[must_use]
pub fn derived_prefill(prompt_tokens: u64) -> Option<f64> {
    let want = bucket(prompt_tokens);
    let st = crate::state::State::load();
    let mut rates: Vec<f64> = st.all("obs").iter().filter_map(|row| {
        let f: Vec<&str> = row.split(' ').collect();
        if f.len() < 6 || f[5] == "1" { return None }
        let (tok, ms) = (f[1].parse::<u64>().ok()?, f[2].parse::<f64>().ok()?);
        if bucket(tok) != want || ms <= 0.0 { return None }
        let r = tok as f64 / (ms / 1000.0);
        // A cache hit is not evidence about cold prefill (V9).
        if r > 3_000.0 { None } else { Some(r) }
    }).collect();
    if rates.len() < 2 { return None }
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
pub fn raise_gen_floor(label: &str, seen: u64) {
    if label.is_empty() || seen == 0 {
        return;
    }
    let mut st = crate::state::State::load();
    if st.get_u64("gen", label).unwrap_or(0) < seen {
        st.set("gen", label, seen.to_string());
        st.save();
    }
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
pub fn observe_gen(label: &str, eval_tokens: u64) {
    if label.is_empty() || eval_tokens == 0 {
        return;
    }
    let mut st = crate::state::State::load();
    let prev = st.get_u64("gen", label).unwrap_or(eval_tokens);
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let next = (prev as f64 * 0.75 + eval_tokens as f64 * 0.25) as u64;
    st.set("gen", label, next.to_string());
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
        ewma(&PREFILL_TOK_S, r.prompt_tokens as f64 / (prefill_ms as f64 / 1000.0));
    }
    if decode_ms > 0 {
        ewma(&DECODE_TOK_S, r.eval_tokens as f64 / (decode_ms as f64 / 1000.0));
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
    fn post(&self, url: &str, body: &str, timeout: Duration)
        -> Result<Box<dyn BufRead + Send>, String>;
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
    fn post(&self, url: &str, body: &str, timeout: Duration)
        -> Result<Box<dyn BufRead + Send>, String>
    {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_recv_response(Some(timeout))
            .build()
            .into();
        let resp = agent.post(url)
            .content_type("application/json")
            .send(body)
            .map_err(|e| format!("{url}: {e}"))?;
        Ok(Box::new(std::io::BufReader::new(resp.into_body().into_reader())))
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
    std::env::var("BBX_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434".into())
}

fn model() -> String {
    std::env::var("BBX_MODEL").unwrap_or_else(|_| "gpt-oss:20b".into())
}

/// Context window to request. Passed PER REQUEST, never set globally -- a
/// global value makes every model allocate a full cache whether it needs one
/// or not, and we know what we are asking for.
fn num_ctx() -> u64 {
    std::env::var("BBX_NUM_CTX").ok().and_then(|v| v.parse().ok()).unwrap_or(131_072)
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
    pub const DETERMINISTIC: Self = Self { temperature: 0.0, seed: 0 };

    /// The `k`th competing candidate. Candidate 0 IS the deterministic call --
    /// so asking for one candidate is exactly today's behaviour, not a
    /// differently-sampled approximation of it.
    #[must_use]
    pub fn candidate(k: usize) -> Self {
        if k == 0 { Self::DETERMINISTIC } else { Self { temperature: 0.6, seed: k as u64 } }
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
pub fn generate_via(
    transport: &dyn Transport,
    prompt: &str,
    label: &str,
    sampling: Sampling,
    eta: Eta,
    on_chunk: &mut dyn FnMut(&str),
) -> Result<Reply, String> {
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
    let stream = transport.post(&format!("{}/api/generate", endpoint()),
                                &body.to_string(), hard)?;

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
        let v: serde_json::Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if let Some(th) = v["thinking"].as_str() {
            if !th.is_empty() {
                thinking.push_str(th);
                if first_chunk.is_none() { first_chunk = Some(Instant::now()) }
                on_chunk("");
            }
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
        let level = if over >= ABORT { 4 } else if over >= WARN { 3 }
                    else if over >= NOTICE_2 { 2 } else if over >= NOTICE_1 { 1 } else { 0 };
        if level > warned {
            warned = level;
            let el = started.elapsed().as_secs_f64();
            match level {
                1 => eprintln!("\n  [pace] {el:.0}s: past the {:.0}s estimate, still streaming", budget.as_secs_f64()),
                2 => eprintln!("\n  [pace] {el:.0}s: 2x the estimate -- the endpoint is slower than this run assumed"),
                3 => eprintln!("\n  [pace] WARNING {el:.0}s: {WARN:.0}x the estimate. Hard stop at {:.0}s.", hard.as_secs_f64()),
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
                        budget.as_secs_f64(), text.len() / 4, thinking.len() / 4, endpoint()));
                }
            }
        }
        // Counts arrive only on the final frame.
        if v["done"].as_bool().unwrap_or(false) {
            prompt_tokens = v["prompt_eval_count"].as_u64().unwrap_or(0);
            eval_tokens = v["eval_count"].as_u64().unwrap_or(0);
            LAST_LOAD_MS.store(v["load_duration"].as_u64().unwrap_or(0) / 1_000_000,
                               Ordering::Relaxed);
        }
    }
    let reply = Reply { text, thinking, prompt_tokens, eval_tokens, ms: started.elapsed().as_millis() };
    // Split the observed time at the first token: everything before it is
    // prefill, everything after is decode. Two rates, learned separately,
    // because they scale differently (R16/R17).
    // Subtract model load: it is disk time, not prefill.
    let load_ms = u128::from(LAST_LOAD_MS.load(Ordering::Relaxed));
    let pre_ms = first_chunk.map_or(reply.ms, |t| (t - started).as_millis()).saturating_sub(load_ms);
    observe(&reply, pre_ms, reply.ms.saturating_sub(pre_ms));
    LAST_CACHED.store(was_cached(reply.prompt_tokens, pre_ms), Ordering::Relaxed);
    record_obs(label, reply.prompt_tokens, pre_ms, reply.eval_tokens,
               reply.ms.saturating_sub(pre_ms), load_ms > 500);
    save_pace();
    Ok(reply)
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
        let Some(close) = after[start..].find("```") else { break };
        let block = &after[start..start + close];
        if block.len() > best.len() {
            best = block.to_string();
        }
        rest = &after[start + close..];
    }
    if best.is_empty() { text.trim().to_string() } else { best.trim().to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fails `fail_times`, then answers. The thing B7 had no way to build.
    struct Flaky {
        fail_times: std::cell::Cell<u32>,
        body: String,
    }

    impl Transport for Flaky {
        fn post(&self, url: &str, _b: &str, _t: Duration)
            -> Result<Box<dyn BufRead + Send>, String>
        {
            let left = self.fail_times.get();
            if left > 0 {
                self.fail_times.set(left - 1);
                return Err(format!("{url}: simulated transport failure"));
            }
            Ok(Box::new(std::io::Cursor::new(self.body.clone().into_bytes())))
        }
    }

    #[test]
    fn a_transport_can_be_substituted_and_made_to_fail() {
        let t = Flaky { fail_times: std::cell::Cell::new(1), body: String::new() };
        assert!(t.post("u", "b", Duration::from_secs(1)).is_err(), "first call fails");
        assert!(t.post("u", "b", Duration::from_secs(1)).is_ok(), "then succeeds");
    }

    #[test]
    fn generate_via_reads_a_substituted_stream() {
        let body = "{\"response\":\"hi\",\"done\":false}\n\
                    {\"response\":\"\",\"done\":true,\"prompt_eval_count\":7,\"eval_count\":2}\n";
        let t = Flaky { fail_times: std::cell::Cell::new(0), body: body.into() };
        let r = generate_via(&t, "p", "test", Sampling::DETERMINISTIC, predict(10),
                             &mut |_| {}).unwrap();
        assert_eq!(r.text, "hi");
        assert_eq!((r.prompt_tokens, r.eval_tokens), (7, 2));
    }

    /// Records what was actually POSTed. The seam exists so a claim about the
    /// request can be checked without an endpoint.
    struct Spy(std::cell::RefCell<String>);
    impl Transport for Spy {
        fn post(&self, _u: &str, b: &str, _t: Duration)
            -> Result<Box<dyn BufRead + Send>, String> {
            self.0.replace(b.to_string());
            Ok(Box::new(std::io::Cursor::new(
                "{\"response\":\"x\",\"done\":true}\n".as_bytes().to_vec())))
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
        assert_eq!(zero["options"]["temperature"], 0.0,
                   "candidate 0 must be today's deterministic call, unchanged");

        let two = posted(Sampling::candidate(2));
        assert!(two["options"]["temperature"].as_f64().unwrap() > 0.0,
                "a competing candidate at temperature 0 is the same call twice (V17)");
        assert_eq!(two["options"]["seed"], 2,
                   "diverse but REPRODUCIBLE -- an unseeded failure cannot be re-examined");
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
