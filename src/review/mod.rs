//! Mechanical pre-review of what `apply` just committed.
//!
//! Not a substitute for reading the diff -- it cannot tell whether code
//! satisfies an INVARIANT, which is the failure that matters. It catches the
//! subset that is decidable, so the reader spends attention on the rest.
//!
//! Each check exists because it would have caught a real commit on this branch.

use crate::tdd::split_module;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule: &'static str,
    pub detail: String,
}

/// A public fn added but called by nothing outside the tests.
///
/// `is_ignored_dir` landed exactly like this: correct, tested, and never wired
/// in, so the duplication the task existed to remove was merely relocated
/// (`.:fed` B4-class).
#[must_use]
pub fn unwired(impl_src: &str, tests_src: &str, new_fns: &[String]) -> Vec<Finding> {
    new_fns.iter().filter_map(|f| {
        let called_in_impl = impl_src.matches(&format!("{f}(")).count() > 1; // decl + call
        let called_in_tests = tests_src.contains(&format!("{f}("));
        (!called_in_impl && called_in_tests).then(|| Finding {
            rule: "unwired",
            detail: format!("`{f}` is called only from tests -- landed but never wired in"),
        })
    }).collect()
}

/// A detector whose test only asserts the empty case.
///
/// `detect_cycles` returned `Vec::new()` and passed, because the test asserted
/// only that no cycle was found (`.:fed` B6).
#[must_use]
pub fn negative_only(tests_src: &str, new_fns: &[String]) -> Vec<Finding> {
    new_fns.iter().filter_map(|f| {
        if !tests_src.contains(&format!("{f}(")) {
            return None;
        }
        // Any assertion that a result is NON-empty / has items counts as positive.
        let positive = ["!.is_empty()", "is_empty(), false", "assert!(!",
                        "len(), 1", "len(), 2", "len() > 0", "> 0"]
            .iter().any(|p| tests_src.contains(p));
        (!positive).then(|| Finding {
            rule: "negative-only",
            detail: format!("`{f}`'s test never asserts something IS found -- \
                             a function that always finds nothing would pass"),
        })
    }).collect()
}

/// Public fn names declared in a source region.
#[must_use]
pub fn public_fns(src: &str) -> Vec<String> {
    src.lines().filter_map(|l| {
        let t = l.trim().strip_prefix("pub fn ")?;
        Some(t.split(['(', '<']).next()?.trim().to_string())
    }).collect()
}

/// Review one node's module against the checks above.
///
/// # Errors
/// Propagates the read failure -- unreadable is not clean.
pub fn node(path: &Path, added: &[String]) -> std::io::Result<Vec<Finding>> {
    let src = std::fs::read_to_string(path)?;
    let (impl_r, tests_r) = split_module(&src);
    let mut out = unwired(impl_r, tests_r, added);
    out.extend(negative_only(tests_r, added));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_a_fn_only_tests_call() {
        let f = unwired("pub fn helper(x: u8) -> bool { true }\n",
                        "assert!(helper(1));", &["helper".into()]);
        assert_eq!(f.len(), 1, "is_ignored_dir landed exactly like this");
        assert_eq!(f[0].rule, "unwired");
    }

    #[test]
    fn accepts_a_fn_the_impl_actually_calls() {
        let f = unwired("pub fn helper(x: u8) -> bool { true }\nfn go() { helper(2); }\n",
                        "assert!(helper(1));", &["helper".into()]);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn flags_a_detector_tested_only_on_empty() {
        let f = negative_only("let c = detect(&e); assert!(c.is_empty());", &["detect".into()]);
        assert_eq!(f.len(), 1, "detect_cycles passed exactly like this");
    }

    #[test]
    fn accepts_a_detector_with_a_positive_case() {
        let f = negative_only("let c = detect(&e); assert!(!c.is_empty());", &["detect".into()]);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn public_fns_reads_declarations_only() {
        let v = public_fns("pub fn a(x: u8) {}\n// pub fn b() {}\nfn c() {}\npub fn d<T>() {}\n");
        assert_eq!(v, vec!["a".to_string(), "d".to_string()]);
    }
}
