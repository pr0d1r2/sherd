//! Local inference endpoint. **Sole call site for `ureq` and `serde_json`** (V72).
//!
//! Plain HTTP to a LAN box -- no TLS, no cloud, no key. Feature-gated so
//! `--no-default-features` leaves the deterministic core §C requires.

use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
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

/// One round-trip, with what it cost. A call without its cost is not evidence.
#[derive(Debug, Clone)]
pub struct Reply {
    pub text: String,
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
pub fn generate_with(prompt: &str, on_chunk: &mut dyn FnMut(&str)) -> Result<Reply, String> {
    let body = serde_json::json!({
        "model": model(),
        "prompt": prompt,
        "stream": true,
        "options": { "num_ctx": num_ctx(), "temperature": 0 },
    });
    let started = Instant::now();
    let resp = ureq::post(&format!("{}/api/generate", endpoint()))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("{}: {e}", endpoint()))?;

    let mut text = String::new();
    let (mut prompt_tokens, mut eval_tokens) = (0, 0);
    for line in BufReader::new(resp.into_reader()).lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if let Some(chunk) = v["response"].as_str() {
            text.push_str(chunk);
            on_chunk(chunk);
        }
        // Counts arrive only on the final frame.
        if v["done"].as_bool().unwrap_or(false) {
            prompt_tokens = v["prompt_eval_count"].as_u64().unwrap_or(0);
            eval_tokens = v["eval_count"].as_u64().unwrap_or(0);
        }
    }
    Ok(Reply { text, prompt_tokens, eval_tokens, ms: started.elapsed().as_millis() })
}

/// Generate with no progress reporting.
///
/// # Errors
/// See [`generate_with`].
pub fn generate(prompt: &str) -> Result<Reply, String> {
    generate_with(prompt, &mut |_| {})
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
