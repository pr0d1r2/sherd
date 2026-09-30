use super::super::fixtures::*;
use super::*;

/// `lens` at each depth, and the depths must not agree.
///
/// `.:B8` is `--depth rule` selecting NOTHING for the project's whole
/// life while §I documented it as the default. A test that only checked
/// the exit code would have passed throughout.
#[test]
fn lens_runs_at_every_depth_and_the_depths_differ() {
    for d in ["rule", "why", "all"] {
        assert_eq!(
            run_args(argv(&["lens", ".", "--depth", d])),
            ExitCode::SUCCESS,
            "lens --depth {d}"
        );
    }
    let root = repo_root();
    let p = |d| lens::pack(&root, &root, d).map(|p| p.text.len());
    let (Ok(rule), Ok(all)) = (p(lens::Depth::Rule), p(lens::Depth::All))
    else {
        panic!("the root pack must be readable at both depths")
    };
    assert!(rule < all, "B8: `rule` must SELECT, not render everything");
}

/// `budget` on a repo whose chain EXCEEDS its declared ceiling.
///
/// The over-ceiling branch is another detector with no positive case:
/// `.:B7` is `.context-limits` declaring per-node ceilings while `budget`
/// PRINTED the table without comparing against them, so every chain
/// drifted over unseen for the project's life. The comparison now exists
/// and the gate keeps this repo at 0 over, which means the branch that
/// reports a breach can never fire here.
#[test]
fn a_chain_over_its_ceiling_exits_one_and_a_generous_one_exits_zero() {
    assert_eq!(over_ceiling_is_caught(), Ok(()));
}

/// A node whose SPEC costs more than one token -- which is every real one.
const FAT_SPEC: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nsomething long \
         enough to cost more than one token, several times over, so the \
         ceiling below is genuinely exceeded rather than merely equalled\n";

fn over_ceiling_is_caught() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-budget-over")?;
    r.write("SPEC.md", FAT_SPEC)?;
    r.write(".context-limits", "SPEC.md 1\n")?;
    r.commit("a node over its ceiling")?;
    let root = r.path().to_path_buf();
    assert_eq!(
        budget(r.path(), root.clone()),
        ExitCode::from(1),
        "B7: a chain over its ceiling must FAIL, not merely print"
    );
    // The control: raise the ceiling and the same tree passes. Without
    // it, `budget` returning 1 unconditionally would satisfy the test.
    r.write(".context-limits", "SPEC.md 100000\n")?;
    r.commit("raise it")?;
    assert_eq!(budget(r.path(), root), ExitCode::SUCCESS);
    Ok(())
}

/// `B6`: every verb must be run against a repo that has NONE of the thing
/// it looks for. Our own tree has all of them, so ABSENCE is the case only
/// a stranger tests -- and both of these were found on one.
#[test]
fn absence_is_reported_and_is_not_a_failure() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-absence")?;
    r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nnothing here\n")?;
    r.commit("a repo with no §F and no slice registry")?;

    // No `.sherd-slices` is "none required", never a bare OS error.
    assert_eq!(slice_cmd(r.path(), "--list"), ExitCode::SUCCESS);
    assert_eq!(slice_cmd(r.path(), "--check"), ExitCode::SUCCESS);

    // No `§F` table says so rather than printing nothing at all
    // (`.:V48`).
    assert_eq!(fed_cmd(r.path()), ExitCode::SUCCESS);

    // And a dir with no spec at all is still a usage error, not silence.
    assert_eq!(fed_cmd(&r.path().join("nowhere")), ExitCode::from(2));
    Ok(())
}

/// `sherd debt` end to end, over a scripted toolchain: the toolchain is a
/// PARAMETER, which is what makes the verb testable at all -- `tdd::Run`
/// carries `cargo` for the same reason. A verb the gate runs on every
/// commit that no test can reach is a verb nobody has checked.
#[test]
fn the_debt_verb_checks_records_and_refuses() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-debt")?;
    r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\ndebt\n")?;
    r.write("src/a.rs", &"fn f() {}\n".repeat(100))?;
    r.write(
        ".lint-debt",
        "# the reason\ndensity 20.0\nshape 25.0\ncount 0\nexcess 0\nloc 0\n",
    )?;
    r.commit("a tree with a ratchet")?;
    let fake = r.path().join("fake-cargo");
    std::fs::write(
        &fake,
        "#!/bin/sh\necho 'src/a.rs:1:1: warning: too many lines (35/15)' >&2\nexit 0\n",
    )
    .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    let fake = fake.display().to_string();

    // 1 warning over 100 lines is 10.0 per KLoC, and 20 excess lines is
    // 20.0% -- both under the recorded ceilings.
    assert_eq!(
        debt_cmd(r.path(), Some("--check"), &fake),
        ExitCode::SUCCESS
    );
    // `--record` brings it current and keeps the reason.
    assert_eq!(
        debt_cmd(r.path(), Some("--record"), &fake),
        ExitCode::SUCCESS
    );
    let after = std::fs::read_to_string(r.path().join(".lint-debt"))
        .unwrap_or_default();
    assert!(after.contains("density 10.0"), "{after}");
    assert!(after.contains("shape 20.0"), "{after}");
    assert!(after.contains("# the reason"), "the reason survives");
    // Recorded at 10.0, so recording again is a no-op and still passes.
    assert_eq!(
        debt_cmd(r.path(), Some("--check"), &fake),
        ExitCode::SUCCESS
    );
    Ok(())
}

/// A tree with no ratchet is a USAGE error, never a clean bill: absence
/// must not read as "zero allowed" (`sherd/debt:V3`).
#[test]
fn a_tree_with_no_ratchet_is_a_usage_error() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-debt-none")?;
    r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nnone\n")?;
    r.write("src/a.rs", "fn f() {}\n")?;
    r.commit("no ratchet here")?;
    assert_eq!(
        debt_cmd(r.path(), Some("--check"), "true"),
        ExitCode::from(2)
    );
    // A file that EXISTS but carries no ceilings is a different
    // mistake, and only one of the two is the reader's fault (`B7`).
    r.write(".lint-debt", "# a comment and nothing else\n")?;
    assert_eq!(
        debt_cmd(r.path(), Some("--check"), "true"),
        ExitCode::from(2)
    );
    Ok(())
}

/// `sherd coverage` end to end over a scripted toolchain, the same way
/// `debt` is tested: the toolchain is a PARAMETER, so the verb the gate
/// runs on every push is reachable from a test.
#[test]
fn the_coverage_verb_checks_records_and_refuses() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-cov")?;
    r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\ncov\n")?;
    r.write(".coverage", "# the reason\nlines 90.00\n")?;
    r.commit("a tree with a floor")?;
    let fake = r.path().join("fake-cargo");
    std::fs::write(
        &fake,
        "#!/bin/sh\necho 'TOTAL 1 2 3.00% 4 5 6.00% 7 8 92.50% 0 0 -'\nexit 0\n",
    )
    .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }
    let fake = fake.display().to_string();

    // 92.50 is above the recorded 90.00.
    assert_eq!(
        coverage_cmd(r.path(), Some("--check"), &fake),
        ExitCode::SUCCESS
    );
    assert_eq!(
        coverage_cmd(r.path(), Some("--record"), &fake),
        ExitCode::SUCCESS
    );
    let after =
        std::fs::read_to_string(r.path().join(".coverage")).unwrap_or_default();
    assert!(after.contains("lines 92.50"), "{after}");
    assert!(after.contains("# the reason"), "the reason survives");

    // A toolchain that cannot run is an ERROR, not a breach (V26).
    assert_eq!(
        coverage_cmd(r.path(), Some("--check"), "definitely-not-a-cargo"),
        ExitCode::from(2)
    );
    // And a tree with no floor is a usage error, not a clean bill.
    let _ = std::fs::remove_file(r.path().join(".coverage"));
    assert_eq!(
        coverage_cmd(r.path(), Some("--check"), &fake),
        ExitCode::from(2)
    );
    Ok(())
}

/// `src/debt:V4` at the verb: with a `lines-default` row the DEFAULT build
/// is checked too, and a drop there fails even while `lines` holds (`src/debt:B8`).
#[test]
fn the_coverage_verb_checks_the_default_build_when_it_has_a_floor()
-> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-cov-default")?;
    let fake =
        crate::debt::coverage_tests::two_builds(r.path(), "90.89", "94.06")?;

    r.write(".coverage", "lines 90.89\nlines-default 94.10\n")?;
    assert_eq!(
        coverage_cmd(r.path(), Some("--check"), &fake),
        ExitCode::from(1),
        "94.06 under a 94.10 floor fails though `lines` holds"
    );

    r.write(".coverage", "lines 90.00\nlines-default 94.00\n")?;
    assert_eq!(
        coverage_cmd(r.path(), Some("--check"), &fake),
        ExitCode::SUCCESS
    );
    assert_eq!(
        coverage_cmd(r.path(), Some("--record"), &fake),
        ExitCode::SUCCESS
    );
    let after = std::fs::read_to_string(r.path().join(".coverage"))
        .map_err(|e| e.to_string())?;
    assert_eq!(after, "lines 90.89\nlines-default 94.06\n");
    Ok(())
}

#[test]
fn route_reports_a_hit_a_miss_and_an_ambiguity_by_exit_code() {
    let repo = routing_fixture("cli-route");
    let root = repo.path();
    assert_eq!(route_cmd(root, "sprockets"), ExitCode::SUCCESS);
    assert_eq!(route_cmd(root, "wombat"), ExitCode::from(2));
    assert_eq!(route_cmd(root, "widgets gizmos"), ExitCode::from(3));
}
