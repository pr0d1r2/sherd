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
fn the_code_and_test_halves_have_their_own_ceilings() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-v50")?;
    r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nceilings\n")?;
    let big =
        "    assert_eq!(one_plus_one(), 2, \"a wordy message\");\n".repeat(400);
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

/// A file in a `tests/` tree is test code in FULL, though it carries no
/// `#[cfg(test)]` -- it is compiled only under one, from the `mod tests;`
/// that points at it. Read by region it would be all CODE, measured against
/// the code ceiling, twice the test one: moving a suite into its own file
/// would then make a test-ceiling breach vanish without a token shrinking.
#[test]
fn a_file_in_a_tests_tree_is_measured_as_tests() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-v50-tree")?;
    r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\ntree\n")?;
    let big = "    assert_eq!(f(), 2, \"a wordy message\");\n".repeat(400);
    let suite = format!("#[test]\nfn a() {{\n{big}}}\n");
    r.write("src/small/tests/small.rs", &suite)?;
    r.commit("a suite in its own file")?;
    let found = file_ceilings(r.path());
    let as_tests = found.iter().filter(|f| f.contains(": tests ")).count();
    assert_eq!(
        (found.len(), as_tests),
        (1, 1),
        "ONE finding, as TESTS: {found:?}"
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
    let big =
        "V1: a rule long enough to blow past two thousand tokens. ".repeat(400);
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

/// A node whose `SPEC.md` cannot be READ -- here a directory stands where
/// the file belongs -- is a VIOLATION: V48 reads "node discovered & ⊥
/// parsed = FAIL, ⊥ skip". This test asserted the skip (0) from 2026-08-23
/// on, citing V48 for the opposite of what V48 said (`.:B31`).
#[test]
fn a_node_whose_spec_cannot_be_read_is_a_violation() {
    let repo = routing_fixture("cli-unreadable");
    let spec = repo.path().join("alpha").join("SPEC.md");
    let Ok(()) = std::fs::remove_file(&spec) else {
        unreachable!("the fixture spec exists")
    };
    let Ok(()) = std::fs::create_dir_all(&spec) else {
        unreachable!("a directory can take its place")
    };
    assert_eq!(validate_specs(&[repo.path().join("alpha")]), 1);
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

/// `.:V44` is enforced, not only stated: a `SPEC.why.md` that leaves a `§V`
/// id unanswered fails `check`. `src/review` carried 1 why row for 10 rules
/// for as long as the rule existed, because nothing read the file but `lens`.
/// An orphan row -- rationale for a rule that is gone -- fails it too.
#[test]
fn a_why_file_missing_a_rule_fails_check() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-v44")?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nwhy\n\n## \u{a7}V INVARIANTS\n\nV1: a ! hold\nV2: b ! hold\n",
    )?;
    r.write("SPEC.why.md", "V1|because\n")?;
    r.commit("a why file one row short")?;
    assert_eq!(check(r.path()), ExitCode::from(1));
    r.write("SPEC.why.md", "V1|because\nV2|-\nV3|a rule that left\n")?;
    assert_eq!(check(r.path()), ExitCode::from(1));
    r.write("SPEC.why.md", "V1|because\nV2|-\n")?;
    assert_eq!(check(r.path()), ExitCode::SUCCESS);
    Ok(())
}
