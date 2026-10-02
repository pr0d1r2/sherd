//! `--format json` over REAL trees: the collectors measure what the text
//! form measured, and the exit code does not move with the format.

use super::super::fixtures::argv;
use super::super::unread::unreadable_node;
use super::*;
use crate::testrepo::TestRepo;

const ROOT: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nthe root\n\n## \u{a7}F FEDERATION\n\n\
     dir|owns|\u{22a5}owns|tokens\na|alpha|-|-\n";
const CHILD: &str =
    "# SPEC\n\n## \u{a7}G GOAL\n\nalpha, long enough to cost tokens\n";

/// A root and a child `a` whose ceiling is far below its chain.
fn warm_tree(tag: &str) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    r.write("SPEC.md", ROOT)?;
    r.write("a/SPEC.md", CHILD)?;
    r.write(".context-limits", "SPEC.md 100000\na 5\n")?;
    Ok(r)
}

/// `own` is the node's spec and `chain` adds its ancestors; the json
/// carries both, and the over-ceiling child fails in either format.
#[test]
fn budget_measures_own_and_chain_and_fails_over_in_both_formats()
-> Result<(), String> {
    let r = warm_tree("plumb-budget")?;
    let b = measure_budget(r.path(), r.path());
    let [root, a] = b.rows.as_slice() else {
        return Err(format!("two rows, got {}", b.rows.len()));
    };
    assert_eq!((root.chain_nodes, a.chain_nodes), (1, 2));
    assert_eq!(
        root.own_tokens, root.chain_tokens,
        "the root inherits nothing"
    );
    assert!(a.own_tokens > 0 && a.own_tokens < a.chain_tokens);
    assert_eq!(a.over_by, Some(a.chain_tokens - 5));
    assert!(budget_json(r.path(), &b).contains(r#""node":"a","#));
    assert_eq!(budget_as(r.path(), r.path(), true), ExitCode::from(1));
    assert_eq!(budget_as(r.path(), r.path(), false), ExitCode::from(1));
    Ok(())
}

/// A node that cannot be read is NAMED in `unmeasured`, never an empty
/// list that reads as clean (`.:V48`); a dir naming no node is still 2.
#[test]
fn budget_json_names_what_it_could_not_measure() -> Result<(), String> {
    let r = unreadable_node("plumb-budget-unread")?;
    let b = measure_budget(r.path(), r.path());
    let json = budget_json(r.path(), &b);
    assert!(
        json.contains(r#""unmeasured":[{"node":"a","error":"#),
        "{json}"
    );
    assert!(json.contains(r#""ok":false"#), "{json}");
    assert_eq!(budget_as(r.path(), r.path(), true), ExitCode::from(1));
    let nope = r.path().join("nope");
    assert_eq!(budget_as(r.path(), &nope, true), ExitCode::from(2));
    Ok(())
}

/// `check` and `validate` in json fail the same tree the text form fails,
/// name the unread node, and pass it once it is readable.
#[test]
fn check_and_validate_json_keep_the_exit_code() -> Result<(), String> {
    let r = unreadable_node("plumb-check")?;
    let json = check_json(r.path(), &check_report(r.path()));
    assert!(
        json.contains(r#"{"file":"a/SPEC.md","line":null,"rule":"sherd/V48","#),
        "{json}"
    );
    assert!(json.contains(r#""ok":false,"nodes_examined":2,"#), "{json}");
    assert_eq!(check_as(r.path(), true), ExitCode::from(1));
    assert_eq!(validate_json_cmd(r.path()), ExitCode::from(1));
    r.write("a/SPEC.md", CHILD)?;
    assert_eq!(check_as(r.path(), true), ExitCode::SUCCESS);
    assert_eq!(validate_json_cmd(r.path()), ExitCode::SUCCESS);
    Ok(())
}

/// An unknown `--format` is usage on every verb that takes one, never a
/// quiet fall back to text a parser then chokes on.
#[test]
fn an_unknown_format_is_usage() {
    for verb in ["budget", "check", "validate"] {
        assert_eq!(
            run_args(argv(&[verb, "--format", "yaml"])),
            ExitCode::from(2),
            "{verb}"
        );
    }
}
