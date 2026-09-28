//! Mechanical pre-review of what `apply` just committed.
//!
//! Not a substitute for reading the diff -- it cannot tell whether code
//! satisfies an INVARIANT, which is the failure that matters. It catches the
//! subset that is decidable, so the reader spends attention on the rest.
//!
//! Each check exists because it would have caught a real commit on this branch.

// Source reading lives in `crate::code` -- one owner for both this node and
// `src/tdd` (`.:B13`). `unwired`'s call detection went with it as `is_called`.
use crate::code::{fn_body, fn_names, is_called, markers, split_module};
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub struct Finding {
    pub rule: &'static str,
    pub detail: String,
}

/// The mechanical rules `review` runs, named for the report.
///
/// `V5` says report what was CHECKED rather than claim clean, and the list
/// lived in `src/cli`'s format string, where it went stale: `undocumented`
/// shipped and the report never mentioned it. Here it sits beside the
/// functions it names, so adding a rule and forgetting the report is one
/// edit away rather than two files apart.
pub const RULES: [&str; 8] = [
    "unwired",
    "negative-only",
    "ignored-input",
    "undocumented",
    "duplication",
    "positional-index",
    "byte-scanner",
    "status-unread",
];

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
        let called = is_called(crate_src, f);
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
pub fn negative_only(
    impl_src: &str,
    tests_src: &str,
    new_fns: &[String],
) -> Vec<Finding> {
    new_fns
        .iter()
        .filter(|f| is_detector(impl_src, f))
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

/// Does `f` RETURN something that can be empty?
///
/// The rule is about a DETECTOR -- `src/fed:B6` is `detect_cycles(edges) ->
/// Vec::new()` shipped under a doc comment reading "stub ... satisfies the
/// current test suite". "Found nothing" is only a failure mode where nothing
/// is expressible, so `Vec` and `Option` are the shapes, and the seven
/// positive markers it looks for are all collection-shaped too.
///
/// Applied to every new `pub fn`, it flagged `double(n) -> u8` and every
/// other scalar the loop writes, which V23 then made FATAL (B6 here).
fn is_detector(impl_src: &str, f: &str) -> bool {
    impl_src.lines().any(|l| {
        let s = l.trim();
        s.starts_with("pub fn ")
            && s.contains(&format!("fn {f}("))
            && s.split("->").nth(1).is_some_and(|r| {
                r.contains("Vec<")
                    || r.contains("Option<")
                    || r.contains("Map<")
            })
    })
}

/// A new public fn with no doc comment.
///
/// Doc comments are not decoration here: `code::signatures` emits the `///`
/// lines into the worker's surface, and `src/code:B4` is a judge that could
/// not tell whether `not_owns` held a path or prose without them. So an
/// undocumented `pub fn` degrades every prompt built from that node
/// afterwards -- the loop's own next run included.
///
/// Scoped to NEW functions, like every rule here. The 69 undocumented public
/// items already in the tree are a separate debt, and flagging them would
/// make this rule fire on work nobody just did.
#[must_use]
pub fn undocumented(impl_src: &str, new_fns: &[String]) -> Vec<Finding> {
    new_fns
        .iter()
        .filter(|f| declared_without_doc(impl_src, f))
        .map(|f| Finding {
            rule: "undocumented",
            detail: format!(
                "`{f}` is a new `pub fn` with no doc comment -- \
                 `signatures` puts those lines in the next prompt"
            ),
        })
        .collect()
}

/// Is `f` declared with no `///` line above it?
///
/// Scans BACK past attributes: `#[must_use]` sits between the doc and the fn
/// all over this crate, so reading only the immediately preceding line would
/// flag the house style as undocumented.
fn declared_without_doc(impl_src: &str, f: &str) -> bool {
    let lines: Vec<&str> = impl_src.lines().collect();
    let decl = format!("fn {f}(");
    let Some(i) = lines.iter().position(|l| {
        let s = l.trim();
        s.starts_with("pub fn ") && s.contains(&decl)
    }) else {
        return false;
    };
    lines.get(..i).is_none_or(|before| {
        !before
            .iter()
            .rev()
            .find(|p| !p.trim().starts_with("#["))
            .is_some_and(|p| p.trim().starts_with("///"))
    })
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
    // ONE walker. This had its own, which never skipped `target/` and
    // concatenated in `read_dir` order, so the same tree could produce a
    // different `crate_src` between runs (`src/debt:§C`).
    let crate_src: String = crate::fed::rust_files(root)
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .map(|s| split_module(&s).0.to_string())
        .collect();
    let (impl_r, _) = split_module(&src);
    let mut out = unwired(&crate_src, tests_r, added);
    out.extend(negative_only(impl_r, tests_r, added));
    out.extend(ignored_input(impl_r, added));
    out.extend(undocumented(impl_r, added));
    // Whole-crate like `unwired`: the case worth catching is cross-node.
    out.extend(duplication(&crate_src, added));
    out.extend(fix_shapes(&crate_src, added));
    Ok(out)
}

/// Public fns ADDED by a commit, per node module it touched.
///
/// Every `pub fn` a revision ADDED, by the file that holds it.
pub type AddedFns = Vec<(std::path::PathBuf, Vec<String>)>;

/// The raw diff of one revision.
///
/// # Errors
/// The revision could not be read. `Command::output()` returns `Ok` for a
/// process that RAN and FAILED, so the exit STATUS is what separates a
/// revision with no changes from one that does not exist -- checking only
/// the spawn result made `git show <unknown rev>` look like an empty diff,
/// and the review then printed a clean bill (B5, V7).
fn diff_of(root: &Path, rev: &str) -> std::io::Result<String> {
    let out = crate::git::at(root, &["show", "--unified=0", rev]).output()?;
    if !out.status.success() {
        return Err(std::io::Error::other(format!(
            "cannot read revision `{rev}`: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The name a `+` line declares, if it declares a `pub fn` at all.
fn added_pub_fn(line: &str) -> Option<&str> {
    let rest = line.strip_prefix('+')?.trim().strip_prefix("pub fn ")?;
    Some(rest.split(['(', '<']).next()?.trim())
}

/// The `pub fn`s a diff adds, per `mod.rs` it touches. Pure over the text.
#[must_use]
pub fn added_fns(diff: &str) -> AddedFns {
    let mut per: std::collections::BTreeMap<std::path::PathBuf, Vec<String>> =
        std::collections::BTreeMap::new();
    let mut file = std::path::PathBuf::new();
    for line in diff.lines() {
        if let Some(p) = line.strip_prefix("+++ b/") {
            file = std::path::PathBuf::from(p);
        } else if file.file_name().is_some_and(|f| f == "mod.rs")
            && let Some(name) = added_pub_fn(line)
        {
            per.entry(file.clone()).or_default().push(name.to_string());
        }
    }
    per.into_iter().collect()
}

/// Reads the diff rather than the file: a review is about what changed, and
/// the whole file would flag everything that ever landed.
///
/// # Errors
/// The revision could not be read -- unreadable is never clean (V7).
pub fn added_in_commit(root: &Path, rev: &str) -> std::io::Result<AddedFns> {
    Ok(added_fns(&diff_of(root, rev)?))
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
    for (file, added) in added_in_commit(root, rev)? {
        for f in node(&root.join(&file), &added)? {
            out.push((file.clone(), f));
        }
    }
    Ok(out)
}

/// A new function that recognises the same markers as an existing one.
///
/// `.:B13` is the defect: "parse Rust source" shipped TWICE across nodes --
/// `src/tdd` held `signatures`, `expected_calls` and `split_module` while
/// `src/review` held `public_fns` and call-detection. Nothing said so; a
/// human noticed months later.
///
/// The signal is the string literals a body MATCHES ON. Two functions that
/// recognise `## §T` are two readings of one row format, wherever they live.
/// Searched over the WHOLE crate like `unwired`, because the case worth
/// catching is cross-node and a per-file check cannot see it (V1).
///
/// TWO shared markers, not one: measured against this crate, a threshold of
/// one fires on every pair that mentions `SPEC.md`, and two leaves seven
/// pairs of which three are real. ADVISORY (V3) -- an encode/decode pair
/// legitimately shares its keys, and only the reader can tell that from a
/// duplicated parse.
#[must_use]
pub fn duplication(crate_src: &str, new_fns: &[String]) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in new_fns {
        let Some(mine) = fn_body(crate_src, f).map(markers) else {
            continue;
        };
        if mine.len() < 2 {
            continue;
        }
        for other in fn_names(crate_src) {
            if &other == f {
                continue;
            }
            let shared = shared_markers(crate_src, &mine, &other);
            if shared.len() < 2 || calls(crate_src, f, &other) {
                continue;
            }
            out.push(Finding {
                rule: "duplication",
                detail: format!(
                    "`{f}` recognises {} that `{other}` already does -- \
                     two readings of one parse (`.:B13`)",
                    shared.join(", ")
                ),
            });
        }
    }
    out
}

/// Markers `other` recognises that are also in `mine`, quoted for a report.
fn shared_markers(src: &str, mine: &[String], other: &str) -> Vec<String> {
    let Some(theirs) = fn_body(src, other).map(markers) else {
        return Vec::new();
    };
    mine.iter()
        .filter(|m| theirs.contains(m))
        .map(|m| format!("`{m}`"))
        .collect()
}

/// Does `f`'s body call `other`? Then it REUSES rather than re-parses.
fn calls(src: &str, f: &str, other: &str) -> bool {
    fn_body(src, f).is_some_and(|b| b.contains(&format!("{other}(")))
}

/// A transform this repository has already applied, with the evidence.
///
/// Named, never applied: `V3` says review reports and the reader judges, and
/// `.:B24` records why that is not timidity -- every one of these rewrites
/// needed a TYPE to get right, and two of the first attempts were refused by
/// the compiler or the gate.
pub struct FixShape {
    /// The rule name, as it appears in a report.
    pub rule: &'static str,
    /// What to write instead.
    pub shape: &'static str,
}

/// Positional indexes into a binding whose length was just checked.
///
/// MEASURED: applied 34 times across two commits, and the code got shorter
/// every time -- the length check and the indexes state one fact in two
/// places that can disagree, and the pattern states it once.
const SLICE_PATTERN: FixShape = FixShape {
    rule: "positional-index",
    shape: "let [a, b] = xs.as_slice() else { ... } -- the pattern carries \
            the arity, so the length check and the indexes stop repeating \
            each other",
};

/// A long function scanning bytes by index.
///
/// MEASURED on `code::expected_calls`: 75 lines and 11 byte indexes became
/// four named helpers over slices, and 24 `indexing_slicing` warnings went
/// with them. A helper that takes a slice has a BOUNDARY to state; a loop
/// carries its bounds in the author's head.
const NAMED_BOUNDARY: FixShape = FixShape {
    rule: "byte-scanner",
    shape: "cut it into helpers that each take a slice and answer one \
            question -- `get` and `saturating_*` become natural where `b[i]` \
            was, and the indexing goes with the length",
};

/// A subprocess whose EXIT STATUS is never read.
///
/// MEASURED: `Command::output()` returns `Ok` for a process that RAN and
/// FAILED, so `let Ok(out) = .. else` catches only "the binary is not on
/// PATH". `review::added_in_commit` read `out.stdout` and nothing else, and
/// `git show <unknown rev>` therefore looked like an empty diff and printed a
/// clean bill (`V7`, `B5`). The same shape is `.:B18` and `.:B20` on the
/// ratchets: a count taken from a tool that did not run.
const READ_THE_STATUS: FixShape = FixShape {
    rule: "status-unread",
    shape: "read `out.status.success()` before `out.stdout` -- `Ok` means \
            the process RAN, not that it worked, and a failed run's empty \
            output reads exactly like a clean one",
};

/// Transforms worth naming in what a commit ADDED.
///
/// Reports the SHAPE, never the edit. `cargo clippy --fix` already applies
/// every rewrite that is machine-applicable, and measured against this tree
/// it changes nothing: all 252 warnings we carry are the ones upstream marks
/// as needing judgement (`.:B24`).
#[must_use]
pub fn fix_shapes(crate_src: &str, new_fns: &[String]) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in new_fns {
        let Some(body) = fn_body(crate_src, f) else {
            continue;
        };
        if let Some(bind) = repeated_index(body) {
            out.push(shape_finding(f, &SLICE_PATTERN, &format!("`{bind}`")));
        }
        if body.contains(".output()") && !body.contains("status") {
            out.push(shape_finding(f, &READ_THE_STATUS, "a subprocess"));
        }
        let scans = body.matches("b[").count();
        if body.lines().count() > 15 && scans >= 3 {
            out.push(shape_finding(
                f,
                &NAMED_BOUNDARY,
                &format!(
                    "{scans} byte indexes over {} lines",
                    body.lines().count()
                ),
            ));
        }
    }
    out
}

fn shape_finding(f: &str, s: &FixShape, what: &str) -> Finding {
    Finding {
        rule: s.rule,
        detail: format!("`{f}`: {what}. Try: {}", s.shape),
    }
}

/// A binding indexed by a literal twice or more, whose length is ALSO
/// checked -- both halves, so a single `xs[0]` after a `match` that already
/// proved the arity is not a finding.
fn repeated_index(body: &str) -> Option<String> {
    crate::code::literal_indexes(body)
        .into_iter()
        .find(|(n, c)| *c >= 2 && body.contains(&format!("{n}.len()")))
        .map(|(n, _)| n)
}

#[cfg(test)]
#[path = "tests/review.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/git.rs"]
mod git_tests;
