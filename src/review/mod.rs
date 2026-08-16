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
pub fn unwired(
    crate_src: &str,
    tests_src: &str,
    new_fns: &[String],
) -> Vec<Finding> {
    new_fns.iter().filter_map(|f| {
        // Search the WHOLE crate, not the declaring module: a pub fn called
        // from a sibling node is wired (B1).
        //
        // A CALL is any occurrence outside a declaration line. Counting
        // occurrences and assuming "declaration plus one" fails on generics:
        // `pub fn f<'a>(` does not contain `f(`, so a called function counted
        // once and read as uncalled (B2).
        // Skip only THIS function's declaration -- not every line starting
        // with `fn`, since a one-line body declares and calls on one line.
        let declares_f = |l: &str| {
            l.contains(&format!("fn {f}(")) || l.contains(&format!("fn {f}<"))
        };
        let called = crate_src.lines()
            .any(|l| l.contains(&format!("{f}(")) && !declares_f(l));
        let called_in_tests = tests_src.contains(&format!("{f}("));
        (!called && called_in_tests).then(|| Finding {
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
    new_fns
        .iter()
        .filter_map(|f| {
            if !tests_src.contains(&format!("{f}(")) {
                return None;
            }
            // Any assertion that a result is NON-empty / has items counts as positive.
            let positive = [
                "!.is_empty()",
                "is_empty(), false",
                "assert!(!",
                "len(), 1",
                "len(), 2",
                "len() > 0",
                "> 0",
            ]
            .iter()
            .any(|p| tests_src.contains(p));
            (!positive).then(|| Finding {
                rule: "negative-only",
                detail: format!(
                    "`{f}`'s test never asserts something IS found -- \
                             a function that always finds nothing would pass"
                ),
            })
        })
        .collect()
}

/// A new public fn with an `_`-prefixed parameter.
///
/// `-D warnings` catches an unused parameter, which is how one stub was
/// caught. So the next stub prefixed it with `_` and the warning vanished --
/// the guard was silenced by the code it was guarding (B3). An ignored input
/// on a NEW function is a stub signal, and the compiler cannot say so.
#[must_use]
pub fn ignored_input(impl_src: &str, new_fns: &[String]) -> Vec<Finding> {
    new_fns.iter().filter_map(|f| {
        let sig = impl_src.lines()
            .find(|l| l.contains(&format!("fn {f}(")) || l.contains(&format!("fn {f}<")))?;
        let params = sig.split_once('(')?.1;
        let ignored: Vec<&str> = params.split(',')
            .filter_map(|p| p.trim().split(':').next())
            .filter(|n| n.starts_with('_') && n.len() > 1)
            .collect();
        (!ignored.is_empty()).then(|| Finding {
            rule: "ignored-input",
            detail: format!("`{f}` ignores {} -- an `_` prefix silences the \
                             unused-parameter warning; a new fn that ignores an \
                             input is usually a stub", ignored.join(", ")),
        })
    }).collect()
}

/// Public fn names declared in a source region.
#[must_use]
pub fn public_fns(src: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| {
            let t = l.trim().strip_prefix("pub fn ")?;
            Some(t.split(['(', '<']).next()?.trim().to_string())
        })
        .collect()
}

/// Review one node's module against the checks above.
///
/// # Errors
/// Propagates the read failure -- unreadable is not clean.
pub fn node(path: &Path, added: &[String]) -> std::io::Result<Vec<Finding>> {
    let src = std::fs::read_to_string(path)?;
    let (_, tests_r) = split_module(&src);
    // Every non-test line of the crate, so a caller in a sibling node counts.
    let root = path
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new("src"));
    let mut crate_src = String::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs")
                && let Ok(s) = std::fs::read_to_string(&p)
            {
                crate_src.push_str(split_module(&s).0);
            }
        }
    }
    let (impl_r, _) = split_module(&src);
    let mut out = unwired(&crate_src, tests_r, added);
    out.extend(negative_only(tests_r, added));
    out.extend(ignored_input(impl_r, added));
    Ok(out)
}

/// Public fns ADDED by a commit, per node module it touched.
///
/// Reads the diff rather than the file: a review is about what changed, and
/// the whole file would flag everything that ever landed.
#[must_use]
pub fn added_in_commit(
    root: &Path,
    rev: &str,
) -> Vec<(std::path::PathBuf, Vec<String>)> {
    let out = std::process::Command::new("git")
        .args(["show", "--unified=0", rev])
        .current_dir(root)
        .output();
    let Ok(out) = out else { return Vec::new() };
    let diff = String::from_utf8_lossy(&out.stdout);
    let mut per: std::collections::BTreeMap<std::path::PathBuf, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut file = std::path::PathBuf::new();
    for line in diff.lines() {
        if let Some(p) = line.strip_prefix("+++ b/") {
            file = std::path::PathBuf::from(p);
        } else if let Some(added) = line.strip_prefix('+')
            && let Some(rest) = added.trim().strip_prefix("pub fn ")
            && file.file_name().is_some_and(|f| f == "mod.rs")
            && let Some(name) = rest.split(['(', '<']).next()
        {
            per.entry(file.clone())
                .or_default()
                .push(name.trim().to_string());
        }
    }
    per.into_iter().collect()
}

/// Review one revision: every node module it touched, every fn it added.
///
/// # Errors
/// Propagates a read failure -- unreadable is not clean.
pub fn commit(
    root: &Path,
    rev: &str,
) -> std::io::Result<Vec<(std::path::PathBuf, Finding)>> {
    let mut out = Vec::new();
    for (file, added) in added_in_commit(root, rev) {
        for f in node(&root.join(&file), &added)? {
            out.push((file.clone(), f));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_a_fn_only_tests_call() {
        let f = unwired(
            "pub fn helper(x: u8) -> bool { true }\n",
            "assert!(helper(1));",
            &["helper".into()],
        );
        assert_eq!(f.len(), 1, "is_ignored_dir landed exactly like this");
        assert_eq!(f[0].rule, "unwired");
    }

    #[test]
    fn accepts_a_fn_the_impl_actually_calls() {
        let f = unwired(
            "pub fn helper(x: u8) -> bool { true }\nfn go() { helper(2); }\n",
            "assert!(helper(1));",
            &["helper".into()],
        );
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn a_generic_declaration_is_still_a_declaration() {
        // `pub fn f<'a>(` does not contain `f(`, which made a CALLED function
        // read as uncalled (B2). My earlier test used a non-generic fn, so it
        // never touched this branch.
        let crate_src = "pub fn helper<'a>(x: &'a str) -> bool { true }\nlet v = helper(s);\n";
        assert!(
            unwired(crate_src, "assert!(helper(1));", &["helper".into()])
                .is_empty()
        );
    }

    #[test]
    fn flags_a_generic_fn_nothing_calls() {
        let crate_src = "pub fn helper<'a>(x: &'a str) -> bool { true }\n";
        assert_eq!(
            unwired(crate_src, "assert!(helper(1));", &["helper".into()]).len(),
            1
        );
    }

    #[test]
    fn accepts_a_fn_called_from_a_sibling_node() {
        // find_exhaustive_violations lives in fed and is called from cli.
        // Checking only the declaring module called that unwired (B1).
        let crate_src = "pub fn helper(x: u8) -> bool { true }\n\
                         // ... src/cli/mod.rs ...\nlet v = helper(3);\n";
        assert!(
            unwired(crate_src, "assert!(helper(1));", &["helper".into()])
                .is_empty()
        );
    }

    #[test]
    fn flags_a_detector_tested_only_on_empty() {
        let f = negative_only(
            "let c = detect(&e); assert!(c.is_empty());",
            &["detect".into()],
        );
        assert_eq!(f.len(), 1, "detect_cycles passed exactly like this");
    }

    #[test]
    fn accepts_a_detector_with_a_positive_case() {
        let f = negative_only(
            "let c = detect(&e); assert!(!c.is_empty());",
            &["detect".into()],
        );
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn flags_a_new_fn_that_ignores_an_input() {
        let src = "pub fn hint(root: &Path, _budget: u64) -> Vec<PathBuf> { vec![] }\n";
        let f = ignored_input(src, &["hint".into()]);
        assert_eq!(
            f.len(),
            1,
            "check_split_hint ignored _budget exactly like this"
        );
        assert_eq!(f[0].rule, "ignored-input");
    }

    #[test]
    fn accepts_a_fn_that_uses_every_input() {
        let src = "pub fn hint(root: &Path, budget: u64) -> Vec<PathBuf> { vec![] }\n";
        assert!(ignored_input(src, &["hint".into()]).is_empty());
    }

    #[test]
    fn public_fns_reads_declarations_only() {
        let v = public_fns(
            "pub fn a(x: u8) {}\n// pub fn b() {}\nfn c() {}\npub fn d<T>() {}\n",
        );
        assert_eq!(v, vec!["a".to_string(), "d".to_string()]);
    }
}
