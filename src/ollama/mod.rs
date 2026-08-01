//! Local inference endpoint. **Sole call site for `ureq` and `serde_json`** (V72).
//!
//! Plain HTTP to a LAN box -- no TLS, no cloud, no key. Feature-gated so
//! `--no-default-features` leaves the deterministic core §C requires.

use std::io::{BufRead, BufReader};
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
    let pre = PREFILL_TOK_S.load(Ordering::Relaxed).max(1) as f64;
    let dec = DECODE_TOK_S.load(Ordering::Relaxed).max(1) as f64;
    let gen = if label.is_empty() {
        EXPECT_GEN.load(Ordering::Relaxed)
    } else {
        crate::state::State::load()
            .get_u64("gen", label)
            .unwrap_or_else(|| EXPECT_GEN.load(Ordering::Relaxed))
    };
    Eta { prefill_s: prompt_tokens as f64 / pre, decode_s: gen as f64 / dec, gen_est: gen }
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
    let budget = eta.total();
    // Hard ceiling: 4x the prediction. Also the socket read timeout, so a
    // server that accepts and then says nothing fails here rather than hanging
    // forever -- silence is the failure mode a streaming client must bound.
    let hard = budget * 4;
    let body = serde_json::json!({
        "model": model(),
        "prompt": prompt,
        "stream": true,
        "options": { "num_ctx": num_ctx(), "temperature": 0 },
    });
    let started = Instant::now();
    let agent = ureq::AgentBuilder::new().timeout_read(hard).build();
    let resp = agent
        .post(&format!("{}/api/generate", endpoint()))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("{}: {e}", endpoint()))?;

    let mut text = String::new();
    let mut thinking = String::new();
    let mut warned = 0u8;
    let mut first_chunk: Option<Instant> = None;
    let (mut prompt_tokens, mut eval_tokens) = (0, 0);
    for line in BufReader::new(resp.into_reader()).lines() {
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
        let level = if over >= 4.0 { 4 } else if over >= 3.0 { 3 }
                    else if over >= 2.0 { 2 } else if over >= 1.0 { 1 } else { 0 };
        if level > warned {
            warned = level;
            let el = started.elapsed().as_secs_f64();
            match level {
                1 => eprintln!("\n  [pace] {el:.0}s: past the {:.0}s estimate, still streaming", budget.as_secs_f64()),
                2 => eprintln!("\n  [pace] {el:.0}s: 2x the estimate -- the endpoint is slower than this run assumed"),
                3 => eprintln!("\n  [pace] WARNING {el:.0}s: 3x the estimate. Aborting at {:.0}s.", hard.as_secs_f64()),
                _ => return Err(format!(
                        "aborted after {el:.0}s -- 4x the {:.0}s estimate. \
                         {} output + {} reasoning tokens so far. Endpoint {} may be \
                         overloaded, or BBX_NUM_CTX too large for its memory.",
                        budget.as_secs_f64(), text.len() / 4, thinking.len() / 4, endpoint())),
            }
        }
        // Counts arrive only on the final frame.
        if v["done"].as_bool().unwrap_or(false) {
            prompt_tokens = v["prompt_eval_count"].as_u64().unwrap_or(0);
            eval_tokens = v["eval_count"].as_u64().unwrap_or(0);
        }
    }
    let reply = Reply { text, thinking, prompt_tokens, eval_tokens, ms: started.elapsed().as_millis() };
    // Split the observed time at the first token: everything before it is
    // prefill, everything after is decode. Two rates, learned separately,
    // because they scale differently (R16/R17).
    let pre_ms = first_chunk.map_or(reply.ms, |t| (t - started).as_millis());
    observe(&reply, pre_ms, reply.ms.saturating_sub(pre_ms));
    LAST_CACHED.store(was_cached(reply.prompt_tokens, pre_ms), Ordering::Relaxed);
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
