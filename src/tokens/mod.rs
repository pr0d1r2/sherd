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
    Count {
        tokens: itok::bpe::count(text),
        method: "o200k",
    }
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

/// Per-path token ceilings, in `itok`'s `.context-limits` format.
///
/// Reused rather than reinvented (§C): `<path><whitespace><limit>`, `#`
/// comments, blank lines skipped. itok gates itself with this file, and a
/// second format for the same job would be two readings of one rule.
#[derive(Debug, Default, Clone)]
pub struct Ceilings {
    rows: Vec<(String, u64)>,
    default: u64,
}

/// Node budget when the file names no ceiling for a path.
///
/// 2,000 tokens, from `.:V6`. A default is required because an unlisted path
/// must not read as "no limit" -- itok's B7 is precisely a row silently
/// skipped and a gate that then checked nothing.
pub const DEFAULT_NODE: u64 = 2_000;

impl Ceilings {
    /// Parse the format. An unparseable limit is an ERROR, never a skipped
    /// row: itok's B7 skipped `SPEC.md 20.5k`, reported "checked: 1 of 2",
    /// and exited 0 while gating nothing.
    ///
    /// # Errors
    /// Returns the offending line, numbered.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut rows = Vec::new();
        for (n, line) in text.lines().enumerate() {
            let l = line.trim();
            if l.is_empty() || l.starts_with('#') {
                continue;
            }
            let mut it = l.split_whitespace();
            match (it.next(), it.next()) {
                (Some(path), Some(limit)) => {
                    let v: u64 = limit.parse()
                        .map_err(|_| format!(".context-limits:{}: `{limit}` is not a token count", n + 1))?;
                    rows.push((path.to_string(), v));
                }
                _ => {
                    return Err(format!(
                        ".context-limits:{}: expected `<path> <limit>`",
                        n + 1
                    ));
                }
            }
        }
        Ok(Self {
            rows,
            default: DEFAULT_NODE,
        })
    }

    /// Load from `root`, or defaults when absent. A missing file is a cold
    /// start; a malformed one is not.
    ///
    /// # Errors
    /// Propagates a parse failure.
    pub fn load(root: &std::path::Path) -> Result<Self, String> {
        match std::fs::read_to_string(root.join(".context-limits")) {
            Ok(t) => Self::parse(&t),
            Err(_) => Ok(Self::default_only()),
        }
    }

    #[must_use]
    pub fn default_only() -> Self {
        Self {
            rows: Vec::new(),
            default: DEFAULT_NODE,
        }
    }

    /// The ceiling for a path: longest matching prefix wins, else the default.
    #[must_use]
    pub fn for_path(&self, path: &str) -> u64 {
        self.rows
            .iter()
            .filter(|(p, _)| path.starts_with(p.as_str()))
            .max_by_key(|(p, _)| p.len())
            .map_or(self.default, |(_, v)| *v)
    }
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
    fn ceilings_parse_the_itok_format() {
        let c =
            Ceilings::parse("# comment\n\nSPEC.md    31000\nsrc/fed 4000\n")
                .unwrap();
        assert_eq!(c.for_path("SPEC.md"), 31_000);
        assert_eq!(c.for_path("src/fed/mod.rs"), 4_000);
        assert_eq!(
            c.for_path("src/lens/mod.rs"),
            DEFAULT_NODE,
            "unlisted -> default"
        );
    }

    #[test]
    fn longest_prefix_wins() {
        let c = Ceilings::parse("src 1000\nsrc/fed 4000\n").unwrap();
        assert_eq!(c.for_path("src/fed/mod.rs"), 4_000);
        assert_eq!(c.for_path("src/lens/mod.rs"), 1_000);
    }

    #[test]
    fn an_unparseable_limit_is_an_error_not_a_skip() {
        // itok B7: a skipped row made a gate check nothing while exiting 0.
        let e = Ceilings::parse("SPEC.md 20.5k\n").unwrap_err();
        assert!(e.contains("not a token count"), "{e}");
        assert!(e.contains(":1:"), "must name the line: {e}");
    }

    #[test]
    fn a_missing_file_is_a_cold_start() {
        let c = Ceilings::load(std::path::Path::new("/nonexistent")).unwrap();
        assert_eq!(c.for_path("anything"), DEFAULT_NODE);
    }

    #[test]
    fn working_saturates_when_window_smaller_than_entry() {
        assert_eq!(working(16_384), 0);
        assert_eq!(working(131_072), 131_072 - ENTRY_COST);
    }
}
