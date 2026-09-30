//! Verbs `run_args` never reached in a test: each is driven through dispatch
//! against a fixture it cannot harm, and asserted on the exit code `§I`
//! documents. The verbs' own logic is tested where it lives; what these hold
//! is the ROUTE -- argv to the right function, with the root taken from argv.

use super::fixtures::*;
use super::*;
use crate::testrepo::TestRepo;

fn at(verb: &[&str], repo: &TestRepo) -> Vec<String> {
    let mut a = argv(verb);
    a.push(repo.path().display().to_string());
    a
}

/// `coverage` and `debt` route to the ratchets with the root from argv.
/// Neither fixture carries a ratchet file, so both are USAGE (2): nothing
/// to compare against is a bad argument, never a clean bill (`.:V48`).
#[test]
fn the_ratchet_verbs_refuse_a_tree_with_no_ratchet() -> Result<(), String> {
    let r = TestRepo::new("route-ratchets")?;
    assert_eq!(
        run_args(at(&["coverage", "--check"], &r)),
        ExitCode::from(2)
    );
    assert_eq!(run_args(at(&["debt", "--check"], &r)), ExitCode::from(2));
    Ok(())
}

/// `land` from `main` refuses before any gate runs, and says so with the
/// violation code: the request was well-formed, the state is wrong.
#[test]
fn land_on_main_is_refused_as_a_violation() -> Result<(), String> {
    let r = TestRepo::new("route-land")?;
    assert_eq!(run_args(at(&["land"], &r)), ExitCode::from(1));
    assert_eq!(run_args(at(&["land", "--push"], &r)), ExitCode::from(1));
    Ok(())
}

/// A `--depth` that is neither `rule` nor `why` is usage, not a pack.
#[test]
fn lens_with_an_unknown_depth_is_usage() {
    assert_eq!(
        run_args(argv(&["lens", ".", "--depth", "deep"])),
        ExitCode::from(2)
    );
}

/// `route` without a query is usage; with one it resolves against the root
/// named in argv, not the one the test runs in.
#[test]
fn route_needs_a_query_and_reads_the_named_root() {
    let r = routing_fixture("route-query");
    assert_eq!(run_args(argv(&["route"])), ExitCode::from(2));
    assert_eq!(run_args(at(&["route", "sprockets"], &r)), ExitCode::SUCCESS);
    assert_eq!(run_args(at(&["route", "wombat"], &r)), ExitCode::from(2));
}

/// `sync --check` on this repo is clean: the gate's `nav` step holds the
/// same claim on every commit.
#[test]
fn sync_check_is_clean_on_this_repo() {
    assert_eq!(run_args(argv(&["sync", "--check"])), ExitCode::SUCCESS);
}

/// `split` on a node with no declared modules reports rather than guesses
/// (`src/split:V1`), and on a dir with no SPEC.md it is usage.
#[test]
fn split_routes_the_named_dir_and_refuses_a_non_node() -> Result<(), String> {
    let r = routing_fixture("route-split");
    assert_eq!(run_args(at(&["split"], &r)), ExitCode::SUCCESS);
    assert_eq!(run_args(argv(&["split", "no-such-dir"])), ExitCode::from(2));
    Ok(())
}

/// `plan --milestone` plans a milestone some node declares, and refuses one
/// nobody does (a typo is not a plan).
#[test]
fn plan_milestone_accepts_a_declared_id_only() -> Result<(), String> {
    let r = TestRepo::new("route-milestone")?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\ntoy\n\n## \u{a7}T TASKS\n\n\
         | id | scope | tasks | done-when |\n|----|-------|-------|-----------|\n\
         | M1 | first | T1 | shipped |\n\nid|status|task|cites\nT1|.|one|-\n",
    )?;
    assert_eq!(
        run_args(at(&["plan", "--milestone", "M1"], &r)),
        ExitCode::SUCCESS
    );
    assert_eq!(
        run_args(at(&["plan", "--milestone", "M9"], &r)),
        ExitCode::from(2)
    );
    Ok(())
}

/// Without the `ollama` feature the model verbs are NOT COMPILED IN, and
/// that is usage with its own message, not "unknown command".
#[cfg(not(feature = "ollama"))]
#[test]
fn a_model_verb_in_the_default_build_is_usage() {
    for verb in ["apply", "ask", "oneshot", "tdd"] {
        assert_eq!(run_args(argv(&[verb])), ExitCode::from(2), "{verb}");
    }
}
