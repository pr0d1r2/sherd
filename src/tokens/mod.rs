//! Token counting. **Sole call site for `itok`** (V72) -- no other module in
//! this crate may name `itok::`, and a planted violation must fail to compile.

use std::path::Path;

/// A count that carries how it was reached (V24: never a bare int).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Count {
    pub tokens: u64,
    /// V17: gates run on `bpe` or better. `dummy` (bytes/4) is never a gate --
    /// measured 48% low on this repo's own caveman-encoded SPEC.md.
    pub method: &'static str,
}

impl std::fmt::Display for Count {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} tok ({})", self.tokens, self.method)
    }
}

/// Real tokenizer count. o200k, offline, deterministic.
#[must_use]
pub fn count(text: &str) -> Count {
    Count { tokens: itok::bpe::count(text), method: "o200k" }
}

/// # Errors
/// Propagates any read failure -- V48: unreadable is a failure, never a
/// silently skipped zero.
pub fn count_file(path: &Path) -> std::io::Result<Count> {
    Ok(count(&std::fs::read_to_string(path)?))
}

/// Measured harness overhead: system prompt + tool schemas, resident before
/// any file loads and re-billed every turn. Budgets subtract it (V46).
pub const ENTRY_COST: u64 = 28_543;

/// Working tokens left on a window after entry cost. Saturates at 0 rather
/// than wrapping -- a negative budget is "does not fit", not a huge one.
#[must_use]
pub const fn working(window: u64) -> u64 {
    window.saturating_sub(ENTRY_COST)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_carries_its_method() {
        let c = count("hello world");
        assert!(c.tokens > 0);
        assert_eq!(c.method, "o200k");
    }

    #[test]
    fn working_saturates_when_window_smaller_than_entry() {
        assert_eq!(working(16_384), 0);
        assert_eq!(working(131_072), 131_072 - ENTRY_COST);
    }
}
