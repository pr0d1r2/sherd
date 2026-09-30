//! `check`, `validate` and `review` -- the verbs that JUDGE a tree and exit 1 on a finding.

use super::*;

pub(super) fn validate(root: &Path) -> ExitCode {
    let nodes = fed::discover(root);
    let structural = validate_specs(&nodes);
    let edges = validate_edges(root);
    let over = validate_ceilings(root, &nodes);
    let drift = validate_drift(root);
    println!(
        "\n  {} nodes · {structural} structural · {edges} edge · \
         {over} over ceiling · {drift} drifted",
        nodes.len()
    );
    if structural + edges + over + drift == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// The structural check, on every node's spec.
pub(super) fn validate_specs(nodes: &[PathBuf]) -> usize {
    let mut bad: usize = 0;
    for node in nodes {
        let path = node.join("SPEC.md");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                unread(&path, &e);
                bad = bad.saturating_add(1);
                continue;
            }
        };
        for v in spec::check(&text) {
            println!("{}:{}: {v}", path.display(), v.line);
            bad = bad.saturating_add(1);
        }
    }
    bad
}

/// Distilled slices against their sources. An unreadable tree counts as one
/// failure rather than zero: "could not look" and "nothing wrong" are the
/// same output otherwise, which is the vacuous pass `.:V48` forbids.
pub(super) fn validate_drift(root: &Path) -> usize {
    // A repository with NO slice registry has nothing to drift from, and
    // absence is legal: `sherd init` then `sherd validate` has to be able to
    // pass, or the two verbs contradict each other. Distinguished from a
    // registry that cannot be READ, which stays a failure.
    if !root.join(".sherd-slices").exists() {
        println!("slice: no registry (none required)");
        return 0;
    }
    match slice::drifted(root) {
        Ok(drifted) => report_drift(&drifted),
        Err(e) => {
            println!("slice: {e}");
            1
        }
    }
}

pub(super) fn report_drift(drifted: &[PathBuf]) -> usize {
    for p in drifted {
        println!("{}: slice drifted from its source", p.display());
    }
    drifted.len()
}

/// Depth and ownership rules over the `§F` edges of every node.
pub(super) fn validate_edges(root: &Path) -> usize {
    let mut bad: usize = 0;
    for node in fed::discover(root) {
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
            continue;
        };
        let edges = fed::edges(&text);
        for e in fed::depth_violations(&edges) {
            println!("{}: edge to `{}` skips a level", node.display(), e.dir);
            bad = bad.saturating_add(1);
        }
    }
    bad
}

/// Every chain against the ceiling it inherits.
pub(super) fn validate_ceilings(root: &Path, nodes: &[PathBuf]) -> usize {
    let over = nodes.iter().filter(|node| over_ceiling(root, node)).count();
    // A COLD START is not a breach: with no `.context-limits` the default is
    // a suggestion nobody wrote, and one verdict that fails on it is a claim
    // about a rule that does not exist (`B8`). Still REPORTED -- `.:V48` --
    // just not counted against the verdict.
    if over > 0 && tokens::Ceilings::load(root).is_ok_and(|c| c.is_cold()) {
        println!(
            "  ({over} over the {} tok default; set .context-limits to gate it)",
            tokens::DEFAULT_NODE
        );
        return 0;
    }
    over
}

/// One chain against the ceiling it inherits. A node whose pack or ceiling
/// cannot be read is not over -- it is unmeasured, and `budget` is the verb
/// that reports that.
pub(super) fn over_ceiling(root: &Path, node: &Path) -> bool {
    let (Ok(pack), Ok(ceiling)) = (
        lens::pack(root, node, lens::Depth::Rule),
        lens::ceiling_for(root, node),
    ) else {
        return false;
    };
    if pack.cost.tokens > ceiling {
        println!(
            "{}: chain {} tok over its ceiling of {ceiling}",
            node.display(),
            pack.cost.tokens
        );
        return true;
    }
    false
}

/// Citations that point at no node or no row (`src/spec:V7`).
///
/// A citation is a LINK, and a link nothing resolves is a comment. `.:V41` was
/// cited from four files since the first commit and never written at root,
/// which is what `src/spec:B2` found once anything looked.
pub(super) fn dangling_citations(root: &Path, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for c in spec::citations(text) {
        let target = if c.owner == "." {
            root.join("SPEC.md")
        } else {
            root.join(&c.owner).join("SPEC.md")
        };
        let Ok(owner_spec) = std::fs::read_to_string(&target) else {
            out.push(format!(
                "{}: sherd/spec:V7: `{}:{}` names no node -- \
                 a citation is a path from the root, `.` for root itself",
                c.line, c.owner, c.id
            ));
            continue;
        };
        if !spec::declares(&owner_spec, &c.id) {
            out.push(format!(
                "{}: sherd/spec:V7: `{}:{}` resolves to no row",
                c.line, c.owner, c.id
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
pub(super) fn file_ceilings(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for f in fed::rust_files(root) {
        let Ok(src) = std::fs::read_to_string(&f) else {
            continue;
        };
        let rel_path = f.strip_prefix(root).unwrap_or(&f);
        let (impl_r, tests_r) = halves(rel_path, src);
        let rel = rel_path.display().to_string();
        for (half, text, ceiling) in [
            ("code", impl_r, crate::debt::CEILING_FILE),
            ("tests", tests_r, crate::debt::CEILING_TEST),
        ] {
            let n = tokens::count(&text).tokens;
            if n > ceiling {
                out.push(format!(
                    "{rel}: sherd/V50: {half} {n} tok over {ceiling} -- \
                     the node carries more than one worker can hold (judgment)"
                ));
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

/// Structural checks over ONE node's spec. Returns how many were FATAL.
///
/// Split from [`check`], which had grown to five independent check families
/// in one loop. Each is one question about one file, and the advisory ones
/// say so in their own text rather than by where they sit.
pub(super) fn check_node(
    root: &Path,
    node: &Path,
    path: &Path,
    text: &str,
) -> usize {
    let mut bad = 0usize;
    for v in spec::check(text) {
        // `v` prints itself already namespaced -- these are the caller's
        // coordinates prefixed to it, which is all sherd owns here.
        println!("{}:{}: {v}", path.display(), v.line);
        bad = bad.saturating_add(1);
    }
    // A bug with no invariant will recur (spec V4). Advisory -- some bugs
    // genuinely warrant no new rule, and forcing one would manufacture
    // invariants to silence a gate.
    for (id, cause) in spec::unreflected_bugs(text) {
        println!(
            "{}: sherd/spec:V4: {id} names no invariant -- `{cause}` \
             will recur (advisory)",
            path.display()
        );
    }
    // A citation is a LINK: it names a node path and a row that exists
    // there (`src/spec:V7`). Nothing resolved them until `src/spec:B2`.
    for d in dangling_citations(root, text) {
        println!("{}:{d}", path.display());
        bad = bad.saturating_add(1);
    }
    // A finished `§T` row is history and every chain pays for it on every
    // turn (`sherd/fed:V9`). Advisory -- some carry a MEASURED result that
    // belongs in `§R` before the row goes.
    for (id, task) in spec::completed_tasks(text) {
        println!(
            "{}: sherd/fed:V9: {id} is done -- `{task}` is history, and §T \
             states remaining work (advisory)",
            path.display()
        );
    }
    bad.saturating_add(check_federation(node, path, text))
        .saturating_add(check_why(node, text))
}

/// `.:V44`: where a node keeps a `SPEC.why.md`, every `§V` id has a row in
/// it -- `-` counts -- and no row names a rule that is gone. Absent file,
/// nothing to check (`src/cli:V12`).
fn check_why(node: &Path, text: &str) -> usize {
    let path = node.join("SPEC.why.md");
    let Ok(why) = std::fs::read_to_string(&path) else {
        return 0;
    };
    let (missing, orphan) = spec::why_gaps(text, &why);
    for id in &missing {
        println!(
            "{}: sherd/V44: {id} has no row -- rationale is kept by \
             reference, and `-` is an answer",
            path.display()
        );
    }
    for id in &orphan {
        println!(
            "{}: sherd/V44: {id} answers no rule in SPEC.md",
            path.display()
        );
    }
    missing.len().saturating_add(orphan.len())
}

/// `§F` structure: duplicate rows (fed V12) and child dirs with no row (fed
/// V11). A missing row is often a dir that is simply not a node yet, so it
/// reports rather than fails.
pub(super) fn check_federation(node: &Path, path: &Path, text: &str) -> usize {
    let edges = fed::edges(text);
    let (dupes, missing) = fed::find_exhaustive_violations(&edges, node);
    let mut bad = 0usize;
    for e in dupes {
        println!(
            "{}: sherd/fed:V12: `{}` named twice in §F -- descent is ambiguous",
            path.display(),
            e.dir
        );
        bad = bad.saturating_add(1);
    }
    for m in missing {
        let name = m.file_name().unwrap_or_default().to_string_lossy();
        println!(
            "{}: sherd/fed:V11: `{name}/` exists on disk with no §F row -- \
             unreachable by descent (advisory)",
            path.display()
        );
    }
    bad
}

/// A discovered node whose `SPEC.md` could not be read. A FAIL, never a
/// skip (`.:V48`): skipping it reported a clean tree that was never read
/// (`.:B31`).
pub(super) fn unread(path: &Path, e: &std::io::Error) {
    println!(
        "{}: sherd/V48: cannot read -- {e}. a node discovered & not read \
         is a failure, not a node with nothing wrong",
        path.display()
    );
}

pub(super) fn check(root: &Path) -> ExitCode {
    let nodes = fed::discover(root);
    let mut bad: usize = 0;
    for node in &nodes {
        let path = node.join("SPEC.md");
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                unread(&path, &e);
                bad = bad.saturating_add(1);
                continue;
            }
        };
        bad = bad.saturating_add(check_node(root, node, &path, &text));
    }
    // `.:V50`, built at last. Reported once for the whole tree rather
    // than per node: the ceiling is per FILE (`.:V119`), and a file belongs
    // to exactly one node, so walking nodes would visit each twice.
    let over = file_ceilings(root);
    for v in &over {
        println!("{v}");
    }
    // `.:V48`: state what was EXAMINED, not only what failed.
    println!(
        "\n  {} .rs files measured against V50 · {} over ceiling (advisory)",
        fed::rust_files(root).len(),
        over.len()
    );
    println!("\n  {} nodes examined · {bad} violations", nodes.len());
    if bad > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
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

#[cfg(test)]
#[path = "tests/check.rs"]
mod tests;
