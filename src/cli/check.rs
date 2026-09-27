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
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
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
        let (impl_r, tests_r) = crate::code::split_regions(&src);
        let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
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

pub(super) fn check(root: &Path) -> ExitCode {
    let nodes = fed::discover(root);
    let mut bad: usize = 0;
    for node in &nodes {
        let path = node.join("SPEC.md");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
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
mod tests {
    use super::super::fixtures::*;
    use super::*;

    /// `check` on a repo that IS broken.
    ///
    /// Every reporting branch in `check` only runs when something is wrong,
    /// so a clean tree exercises none of them -- and this repo is kept clean
    /// by the gate. A detector tested only on the negative case is satisfied
    /// by finding nothing, which is `src/fed:B6` and the reason `src/fed:V10`
    /// exists. These are five such detectors with no positive case.
    #[test]
    fn check_reports_the_violations_it_finds_and_exits_one() {
        assert_eq!(broken_repo_is_caught(), Ok(()));
    }

    /// A §F row pointing at a directory that is not there (fed V1), the same
    /// dir named twice (fed V12), a child on disk with no row (fed V11), and
    /// a §B row naming no invariant (spec V4).
    const BROKEN: &str = "# SPEC\n\n## \u{a7}V INVARIANTS\n\nV1: out of order\n\n## \u{a7}G GOAL\n\nbroken on purpose\n\n## \u{a7}T TASKS\n\nid|status|task|cites\nT1|?|a status that is not x, ~ or .|-\n\n\
         ## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n\
         ghost|nothing real|-|-\n\
         twice|a|-|-\n\
         twice|b|-|-\n\n\
         ## \u{a7}B BUGS\n\nid|date|cause|fix\n\
         B1|2026-08-21|something broke|no invariant named here\n";

    fn broken_repo_is_caught() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-check-broken")?;
        r.write("SPEC.md", BROKEN)?;
        // A real child dir with no §F row -- fed V11's advisory.
        r.write("orphan/SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nx\n")?;
        r.commit("a deliberately broken tree")?;
        assert_eq!(
            check(r.path()),
            ExitCode::from(1),
            "a tree with violations must exit 1, never 0"
        );
        Ok(())
    }

    /// The control, and it is what makes the test above mean anything: the
    /// same function on a clean tree exits 0. Without this, `check` returning
    /// 1 unconditionally would pass.
    #[test]
    fn check_on_a_clean_tree_exits_zero() {
        assert_eq!(check_clean_tree(), Ok(()));
    }

    fn check_clean_tree() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-check-clean")?;
        r.write(
            "SPEC.md",
            "# SPEC\n\n## \u{a7}G GOAL\n\nclean\n\n## \u{a7}V INVARIANTS\n\n\
             V1: something ! hold\n",
        )?;
        r.commit("a clean tree")?;
        assert_eq!(check(r.path()), ExitCode::SUCCESS);
        Ok(())
    }

    /// `.:V50`: the two halves are counted SEPARATELY, never as one ceiling
    /// over both. A tiny implementation with a large suite is a different
    /// thing from the reverse, and one number over the pair cannot tell them
    /// apart -- so this fixture is exactly that shape.
    #[test]
    fn the_code_and_test_halves_have_their_own_ceilings() -> Result<(), String>
    {
        let r = crate::testrepo::TestRepo::new("cli-v50")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nceilings\n")?;
        let big = "    assert_eq!(one_plus_one(), 2, \"a wordy message\");\n"
            .repeat(400);
        r.write(
            "src/small.rs",
            &format!(
                "pub fn one_plus_one() -> u32 {{ 2 }}\n\
                 #[cfg(test)]\nmod t {{\n use super::*;\n #[test]\n \
                 fn a() {{\n{big}}}\n}}\n"
            ),
        )?;
        r.commit("a small impl and a large suite")?;
        let found = file_ceilings(r.path());
        assert_eq!(found.len(), 1, "one half over, not both: {found:?}");
        let Some(one) = found.first() else {
            unreachable!("just asserted a length of one")
        };
        assert!(
            one.contains("tests"),
            "the TEST half is the one over: {one}"
        );
        assert!(
            one.contains("(judgment)"),
            "kind is judgment -- the call is the reader's: {one}"
        );
        Ok(())
    }

    /// A file inside both ceilings reports nothing at all -- the check must
    /// not fire on every file merely for existing.
    #[test]
    fn a_file_within_both_ceilings_is_silent() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-v50-quiet")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nquiet\n")?;
        r.write("src/tiny.rs", "pub fn f() -> u8 { 1 }\n")?;
        r.commit("one small file")?;
        assert!(file_ceilings(r.path()).is_empty());
        Ok(())
    }

    /// `B8`: a COLD START is not a breach. With no `.context-limits` at all
    /// the default is a suggestion nobody wrote, and failing a stranger
    /// against it is a claim about a rule that does not exist.
    ///
    /// An unlisted PATH inside an EXISTING file is a different thing and
    /// still takes the default -- `.:V6` requires that, or a row silently
    /// skipped gates nothing.
    #[test]
    fn a_default_ceiling_nobody_set_is_advisory() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-cold")?;
        let big = "V1: a rule long enough to blow past two thousand tokens. "
            .repeat(400);
        r.write(
            "SPEC.md",
            &format!("# SPEC\n\n## \u{a7}V INVARIANTS\n\n{big}\n"),
        )?;
        r.commit("a chain over the default, with no ceilings file")?;

        // Cold: reported, not failed.
        assert_eq!(budget(r.path(), r.path().to_path_buf()), ExitCode::SUCCESS);

        // Warm: the SAME tree with a ceilings file fails, because now the
        // number is one somebody wrote.
        r.write(".context-limits", "SPEC.md 100\n")?;
        assert_eq!(budget(r.path(), r.path().to_path_buf()), ExitCode::from(1));

        // And a path unlisted in an existing file still takes the default.
        r.write(".context-limits", "src/nowhere 999999\n")?;
        assert_eq!(budget(r.path(), r.path().to_path_buf()), ExitCode::from(1));

        // `validate` gives ONE verdict over the same rule: cold counts zero,
        // warm counts the breach.
        let nodes = crate::fed::discover(r.path());
        assert_eq!(validate_ceilings(r.path(), &nodes), 1, "warm counts it");
        std::fs::remove_file(r.path().join(".context-limits"))
            .map_err(|e| e.to_string())?;
        assert_eq!(validate_ceilings(r.path(), &nodes), 0, "cold does not");
        Ok(())
    }

    /// `src/spec:B2` and `B3`: a citation is a LINK, and both ways it can
    /// fail are distinct mistakes. This tree reports neither, which is why
    /// the branches need a repo that does.
    #[test]
    fn a_citation_fails_on_a_missing_node_and_on_a_missing_row()
    -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-citations")?;
        r.write(
            "SPEC.md",
            "# SPEC\n\n## \u{a7}G GOAL\n\nlinks\n\n## \u{a7}V INVARIANTS\n\n\
             V1: see `nowhere:V3`\nV2: see `.:V99`\nV3: see `.:V1`\n",
        )?;
        r.commit("two dead links and one live one")?;
        let Ok(text) = std::fs::read_to_string(r.path().join("SPEC.md")) else {
            unreachable!("just written")
        };
        let found = dangling_citations(r.path(), &text);
        assert_eq!(found.len(), 2, "V3 cites a row that exists: {found:?}");
        assert!(
            found.iter().any(|f| f.contains("names no node")),
            "`nowhere` is not a node: {found:?}"
        );
        assert!(
            found.iter().any(|f| f.contains("resolves to no row")),
            "root has no V99: {found:?}"
        );
        Ok(())
    }

    /// `review` on a commit that adds a STUB, so the findings loop runs.
    ///
    /// The printing branch only executes when there is something to print,
    /// and this repo's own commits are reviewed clean -- so `review_cmd`'s
    /// findings path had never run. A stub is a new `pub fn` called only from
    /// its own test, which is exactly what `unwired` flags (`src/fed:B6`).
    #[test]
    fn review_prints_the_findings_it_has_and_stays_advisory() {
        assert_eq!(review_a_stub_commit(), Ok(()));
    }

    fn review_a_stub_commit() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-review-stub")?;
        r.write(
            "src/n/mod.rs",
            "pub fn stub() -> bool { false }\n\n#[cfg(test)]\nmod t {\n \
             use super::*;\n #[test]\n fn a() { assert!(!stub()); }\n}\n",
        )?;
        r.commit("add a stub called only by its own test")?;
        assert_eq!(
            review_cmd(r.path(), "HEAD"),
            ExitCode::SUCCESS,
            "V3: a finding is ADVISORY -- review reports, the reader judges, \
             and auto-failing would trade a false negative for a false positive"
        );
        Ok(())
    }

    /// `review` against a revision that does not exist.
    #[test]
    fn review_of_an_unknown_revision_is_an_error_not_a_clean_bill() {
        // `src/review`'s own rule: an unreadable module is an error, never a
        // clean review. A missing rev reporting "no findings" would be the
        // most dangerous possible output.
        assert_ne!(
            run_args(argv(&["review", "definitely-not-a-rev"])),
            ExitCode::SUCCESS,
            "a rev that does not exist cannot be clean"
        );
    }

    /// `review` of a real revision runs and reports.
    #[test]
    fn review_of_a_real_revision_reports_and_succeeds() {
        // ADVISORY by design -- findings do not fail the command -- so the
        // assertion is that it runs and classifies, not that it is silent.
        //
        // Handed a FIXTURE repository rather than run against whatever tree
        // the runner sits in (V6/B1): `run_args` would resolve the root from
        // the CWD, which is this checkout here and is not a repository at
        // all inside a crate tarball or a nix sandbox.
        let Ok(repo) = crate::testrepo::TestRepo::new("cli-review") else {
            unreachable!("a fixture repository is buildable")
        };
        assert_eq!(review_cmd(repo.path(), "HEAD"), ExitCode::SUCCESS);
    }

    /// The edge half. A `§F` row naming a grandchild skips a level, which is
    /// the one structural rule `validate` checks that `check` does not.
    #[test]
    fn an_edge_that_skips_a_level_is_a_finding() {
        let repo = routing_fixture("cli-edges");
        assert_eq!(validate_edges(repo.path()), 0);

        write_spec(
            repo.path(),
            "alpha",
            "widgets\n\n## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\ndeep/deeper|a|b|-",
        );
        assert_eq!(validate_edges(repo.path()), 1);
    }

    /// A node whose `SPEC.md` cannot be READ is skipped rather than counted
    /// as a violation: `validate` reports what it examined, and an
    /// unreadable file was not examined (`.:V48`).
    #[test]
    fn a_node_whose_spec_cannot_be_read_is_skipped() {
        let repo = routing_fixture("cli-unreadable");
        let spec = repo.path().join("alpha").join("SPEC.md");
        let Ok(()) = std::fs::remove_file(&spec) else {
            unreachable!("the fixture spec exists")
        };
        let Ok(()) = std::fs::create_dir_all(&spec) else {
            unreachable!("a directory can take its place")
        };
        assert_eq!(validate_specs(&[repo.path().join("alpha")]), 0);
    }

    /// The drift half, which the other two `validate` tests never reach: a
    /// tree with no slice registry counts ZERO, and a registry that cannot
    /// be read still counts one (`.:src/cli:B3`, `V12`).
    #[test]
    fn a_missing_slice_registry_is_not_drift() {
        let repo = routing_fixture("cli-drift");
        assert_eq!(validate_drift(repo.path()), 0);

        let Ok(()) = std::fs::create_dir_all(repo.path().join(".sherd-slices"))
        else {
            unreachable!("a registry dir is creatable")
        };
        // A directory where a registry file belongs: present, unreadable.
        assert_eq!(validate_drift(repo.path()), 1);
    }

    /// `validate` REPORTS what it examined, and a planted structural
    /// violation has to move its verdict -- a validator that cannot fail is
    /// the vacuous pass every gate here is written against.
    #[test]
    fn validate_passes_a_clean_tree_and_fails_a_broken_one() {
        let repo = routing_fixture("cli-validate");
        assert_eq!(validate(repo.path()), ExitCode::SUCCESS);

        // A task citing a rule that does not exist: dangling, and structural.
        write_spec(
            repo.path(),
            "alpha",
            "widgets\n\n## \u{a7}T TASKS\n\nid|status|task|cites\nT1|.|do it|V99",
        );
        assert_eq!(validate(repo.path()), ExitCode::from(1));
    }
}
