//! `check` and `review` -- the verbs that JUDGE a tree and exit 1 on a finding.
//!
//! Each check family COLLECTS [`Finding`]s; the text form prints their lines
//! and the json form (`plumb`) serialises the same values, so the two forms
//! cannot disagree about what was found (V19).

use super::*;

pub(super) fn verdict(bad: usize) -> ExitCode {
    if bad == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

pub(super) fn microlith_findings(path: &Path, text: &str) -> Vec<Finding> {
    // `v` prints itself already namespaced -- the caller's coordinates are
    // all sherd owns here.
    spec::check(text)
        .into_iter()
        .map(|v| {
            Finding::new(path, &format!("microlith/{}", v.rule), v.msg)
                .at(v.line)
        })
        .collect()
}

/// Citations that point at no node or no row (`src/spec:V7`).
///
/// A citation is a LINK, and a link nothing resolves is a comment. `.:V41` was
/// cited from four files since the first commit and never written at root,
/// which is what `src/spec:B2` found once anything looked.
fn dangling(root: &Path, text: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    for c in spec::citations(text) {
        let target = if c.owner == "." {
            root.join("SPEC.md")
        } else {
            root.join(&c.owner).join("SPEC.md")
        };
        let Ok(owner_spec) = std::fs::read_to_string(&target) else {
            out.push((
                c.line,
                format!(
                    "`{}:{}` names no node -- \
                     a citation is a path from the root, `.` for root itself",
                    c.owner, c.id
                ),
            ));
            continue;
        };
        if !spec::declares(&owner_spec, &c.id) {
            out.push((
                c.line,
                format!("`{}:{}` resolves to no row", c.owner, c.id),
            ));
        }
    }
    out
}

/// `.:V50` -- the per-file code and test ceilings, measured at last.
///
/// Reports as kind `judgment`, which `§I` defines as the finding whose call
/// belongs to the reader: a file over the limit is a design question, not a
/// defect, and the same wording that made `review` advisory applies. It is
/// NOT fatal, and `.:B23` records why -- eleven of fourteen nodes sit over
/// the test ceiling, which says 2,000 was set before the suite reached this
/// size. Re-derive that number before this refuses a commit.
///
/// Counted SEPARATELY per `§V50`, never as one ceiling over both.
///
/// Over every REGION, not from the first `#[cfg(test)]` onward: production
/// code written below a test module is code, and measuring it as test weight
/// is what `.:B29` records. `code::split_regions` owns that reading -- the
/// cut point `src/tdd` and `src/review` edit against is a different question
/// and keeps its own function.
pub(super) fn file_findings(root: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    for f in fed::rust_files(root) {
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        let rel_path = f.strip_prefix(root).unwrap_or(&f);
        let (impl_r, tests_r) = halves(rel_path, src);
        for (half, text, ceiling) in [
            ("code", impl_r, crate::debt::CEILING_FILE),
            ("tests", tests_r, crate::debt::CEILING_TEST),
        ] {
            let n = tokens::count(&text).tokens;
            if n > ceiling {
                out.push(
                    Finding::new(
                        rel_path,
                        "sherd/V50",
                        format!(
                            "{half} {n} tok over {ceiling} -- the node \
                             carries more than one worker can hold (judgment)"
                        ),
                    )
                    .advisory(),
                );
            }
        }
    }
    out
}

/// A file's `(code, tests)` halves for V50. A file under a `tests/` tree is
/// test code in FULL: it carries no `#[cfg(test)]` to find, because the
/// `mod tests;` that includes it does (`src/cli:V18`). Read by region it
/// would be all code, against a ceiling twice the test one.
fn halves(rel: &Path, src: String) -> (String, String) {
    if rel.components().any(|c| c.as_os_str() == "tests") {
        (String::new(), src)
    } else {
        crate::code::split_regions(&src)
    }
}

/// Structural checks over ONE node's spec, fatal and advisory, in the order
/// the text form prints them.
///
/// Split from [`check`], which had grown to five independent check families
/// in one loop. Each is one question about one file, and the advisory ones
/// say so in their own text rather than by where they sit.
pub(super) fn node_findings(
    root: &Path,
    node: &Path,
    path: &Path,
    text: &str,
) -> Vec<Finding> {
    let mut out = microlith_findings(path, text);
    // A bug with no invariant will recur (spec V4). Advisory -- some bugs
    // genuinely warrant no new rule, and forcing one would manufacture
    // invariants to silence a gate.
    for (id, cause) in spec::unreflected_bugs(text) {
        out.push(
            Finding::new(
                path,
                "sherd/spec:V4",
                format!(
                    "{id} names no invariant -- `{cause}` will recur (advisory)"
                ),
            )
            .advisory(),
        );
    }
    // A citation is a LINK: it names a node path and a row that exists
    // there (`src/spec:V7`). Nothing resolved them until `src/spec:B2`.
    for (line, msg) in dangling(root, text) {
        out.push(Finding::new(path, "sherd/spec:V7", msg).at(line));
    }
    // A finished `§T` row is history and every chain pays for it on every
    // turn (`sherd/fed:V9`). Advisory -- some carry a MEASURED result that
    // belongs in `§R` before the row goes.
    for (id, task) in spec::completed_tasks(text) {
        out.push(
            Finding::new(
                path,
                "sherd/fed:V9",
                format!(
                    "{id} is done -- `{task}` is history, and §T states \
                     remaining work (advisory)"
                ),
            )
            .advisory(),
        );
    }
    out.extend(federation_findings(node, path, text));
    out.extend(why_findings(node, text));
    out
}

/// `.:V44`: where a node keeps a `SPEC.why.md`, every `§V` id has a row in
/// it -- `-` counts -- and no row names a rule that is gone. Absent file,
/// nothing to check (`src/cli:V12`).
fn why_findings(node: &Path, text: &str) -> Vec<Finding> {
    let path = node.join("SPEC.why.md");
    let Ok(why) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let (missing, orphan) = spec::why_gaps(text, &why);
    let missing = missing.iter().map(|id| {
        Finding::new(
            &path,
            "sherd/V44",
            format!(
                "{id} has no row -- rationale is kept by reference, and `-` \
                 is an answer"
            ),
        )
    });
    let orphan = orphan.iter().map(|id| {
        Finding::new(
            &path,
            "sherd/V44",
            format!("{id} answers no rule in SPEC.md"),
        )
    });
    missing.chain(orphan).collect()
}

/// `§F` structure: duplicate rows (fed V12) and child dirs with no row (fed
/// V11). A missing row is often a dir that is simply not a node yet, so it
/// reports rather than fails.
fn federation_findings(node: &Path, path: &Path, text: &str) -> Vec<Finding> {
    let edges = fed::edges(text);
    let (dupes, missing) = fed::find_exhaustive_violations(&edges, node);
    let dupes = dupes.into_iter().map(|e| {
        Finding::new(
            path,
            "sherd/fed:V12",
            format!("`{}` named twice in §F -- descent is ambiguous", e.dir),
        )
    });
    let missing = missing.into_iter().map(|m| {
        let name = m.file_name().unwrap_or_default().to_string_lossy();
        Finding::new(
            path,
            "sherd/fed:V11",
            format!(
                "`{name}/` exists on disk with no §F row -- unreachable by \
                 descent (advisory)"
            ),
        )
        .advisory()
    });
    dupes.chain(missing).collect()
}

/// A discovered node whose `SPEC.md` could not be read. A FAIL, never a
/// skip (`.:V48`): skipping it reported a clean tree that was never read
/// (`.:B31`).
pub(super) fn unread(path: &Path, e: &std::io::Error) -> Finding {
    Finding::new(
        path,
        "sherd/V48",
        format!(
            "cannot read -- {e}. a node discovered & not read is a failure, \
             not a node with nothing wrong"
        ),
    )
}

/// What `check` collected: per-node findings, then the `.:V50` file
/// ceilings, which are reported once for the whole tree.
pub(super) struct CheckReport {
    pub nodes: usize,
    pub findings: Vec<Finding>,
    pub files: Vec<Finding>,
    pub rs_files: usize,
}

pub(super) fn check_report(root: &Path) -> CheckReport {
    let nodes = fed::discover(root);
    let mut findings = Vec::new();
    for node in &nodes {
        let path = node.join("SPEC.md");
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                findings.extend(node_findings(root, node, &path, &text))
            }
            Err(e) => findings.push(unread(&path, &e)),
        }
    }
    // `.:V50`, built at last. Reported once for the whole tree rather
    // than per node: the ceiling is per FILE (`.:V119`), and a file belongs
    // to exactly one node, so walking nodes would visit each twice.
    CheckReport {
        nodes: nodes.len(),
        findings,
        files: file_findings(root),
        rs_files: fed::rust_files(root).len(),
    }
}

/// The text form, as dispatch called it before `--format` -- kept for the
/// tests that drive it, and compiled as one (see `repo_root`).
#[cfg(test)]
pub(super) fn check(root: &Path) -> ExitCode {
    check_as(root, false)
}

pub(super) fn check_as(root: &Path, json: bool) -> ExitCode {
    let c = check_report(root);
    let bad = fatal(&c.findings);
    if json {
        println!("{}", check_json(root, &c));
        return verdict(bad);
    }
    print_findings(&c.findings);
    print_findings(&c.files);
    // `.:V48`: state what was EXAMINED, not only what failed.
    println!(
        "\n  {} .rs files measured against V50 · {} over ceiling (advisory)",
        c.rs_files,
        c.files.len()
    );
    println!("\n  {} nodes examined · {bad} violations", c.nodes);
    verdict(bad)
}

pub(super) fn review_cmd(root: &Path, rev: &str) -> ExitCode {
    match crate::review::commit(root, rev) {
        Ok(fs) if fs.is_empty() => {
            // V4: say what was CHECKED. "clean" on two rules is not "clean".
            println!(
                "{rev}: no findings (checked: {})",
                crate::review::RULES.join(", ")
            );
            ExitCode::SUCCESS
        }
        Ok(fs) => {
            for (file, f) in &fs {
                println!(
                    "{}: sherd/review:{}: {}",
                    file.display(),
                    f.rule,
                    f.detail
                );
            }
            println!("\n  {} finding(s) -- ADVISORY. Read the diff.", fs.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sherd: {e}");
            ExitCode::from(2)
        }
    }
}

/// [`dangling`] as the text form prints it after the path. Kept for the
/// tests that read it, and compiled as one (see `repo_root`).
#[cfg(test)]
pub(super) fn dangling_citations(root: &Path, text: &str) -> Vec<String> {
    dangling(root, text)
        .into_iter()
        .map(|(line, msg)| format!("{line}: sherd/spec:V7: {msg}"))
        .collect()
}

/// [`file_findings`] as the text form prints them, for the same tests.
#[cfg(test)]
pub(super) fn file_ceilings(root: &Path) -> Vec<String> {
    file_findings(root).iter().map(Finding::text).collect()
}

#[cfg(test)]
#[path = "tests/check.rs"]
mod tests;
