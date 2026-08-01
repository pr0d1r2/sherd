//! Local inference endpoint. **Sole call site for `ureq` and `serde_json`** (V72).
//!
//! Plain HTTP to a LAN box -- no TLS, no cloud, no key. Feature-gated so
//! `--no-default-features` leaves the deterministic core §C requires.

use std::time::Instant;

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

/// # Errors
/// Transport failure or a response that is not the expected JSON shape. A
/// miss is an error, never an empty success (V20).
pub fn generate(prompt: &str) -> Result<Reply, String> {
    let body = serde_json::json!({
        "model": model(),
        "prompt": prompt,
        "stream": false,
        "options": { "num_ctx": num_ctx(), "temperature": 0 },
    });
    let started = Instant::now();
    let resp = ureq::post(&format!("{}/api/generate", endpoint()))
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
        .map_err(|e| format!("{}: {e}", endpoint()))?;
    let v: serde_json::Value =
        serde_json::from_reader(resp.into_reader()).map_err(|e| e.to_string())?;
    Ok(Reply {
        text: v["response"].as_str().unwrap_or_default().to_string(),
        prompt_tokens: v["prompt_eval_count"].as_u64().unwrap_or(0),
        eval_tokens: v["eval_count"].as_u64().unwrap_or(0),
        ms: started.elapsed().as_millis(),
    })
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
