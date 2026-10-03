//! V5: a script a node owns that invokes a script under a sibling is a
//! blocking code edge. Each test pairs the call that must count with one
//! that must not, so a reader that finds every `.sh` -- or none -- fails.

use super::*;
use crate::testrepo::TestRepo;

const DIR: &str = "DIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\n";

/// Nodes `a`, `b` and `c` under the root, each with a SPEC.md, and the
/// scripts given as `(path, body)`.
fn tree(tag: &str, scripts: &[(&str, &str)]) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    for n in ["a", "b", "c"] {
        r.write(&format!("{n}/SPEC.md"), "# SPEC\n")?;
    }
    r.write("b/lib.sh", "#!/bin/sh\n")?;
    r.write("c/x.sh", "#!/bin/sh\n")?;
    for (path, body) in scripts {
        r.write(path, body)?;
    }
    r.commit("tree")?;
    Ok(r)
}

fn needs(root: &Path, node: &str) -> Vec<String> {
    code_deps(root)
        .into_iter()
        .find(|d| d.node == node)
        .map(|d| d.needs)
        .unwrap_or_default()
}

/// The fixture #82 asks for: `a` runs `b`'s script through a variable
/// holding its own dir, so `b` and `c` build first and `a` after.
#[test]
fn a_script_running_a_siblings_script_waits_for_it() -> Result<(), String> {
    let body = format!("{DIR}bash \"$DIR/../b/lib.sh\"\n");
    let r = tree("wave-sh", &[("a/run.sh", &body)])?;
    let s = wave(r.path(), r.path());
    let round = |i: usize| s.rounds.get(i).cloned().unwrap_or_default();
    assert!(
        round(0).contains(&"b".to_string())
            && round(0).contains(&"c".to_string()),
        "{s:?}"
    );
    assert!(!round(0).contains(&"a".to_string()), "{s:?}");
    assert_eq!(round(1), vec!["a".to_string()]);
    Ok(())
}

/// `source`, `.` and direct execution each make the same wait.
#[test]
fn source_dot_and_direct_execution_are_each_an_edge() -> Result<(), String> {
    for (tag, call) in [
        (
            "wave-sh-source",
            "source \"$(dirname \"$0\")/../b/lib.sh\"\n",
        ),
        ("wave-sh-dot", ". b/lib.sh\n"),
        (
            "wave-sh-exec",
            "\"$(git rev-parse --show-toplevel)/b/lib.sh\" --flag\n",
        ),
    ] {
        let r = tree(tag, &[("a/run.sh", call)])?;
        assert_eq!(needs(r.path(), "a"), vec!["b"], "{call}");
    }
    Ok(())
}

/// A mention runs nothing, and a call inside the node itself is no edge.
#[test]
fn a_mention_or_a_call_within_the_node_is_not_an_edge() -> Result<(), String> {
    let body = format!(
        "{DIR}echo \"see b/lib.sh\"\n# bash b/lib.sh\nbash \"$DIR/own.sh\"\n"
    );
    let r = tree(
        "wave-sh-none",
        &[("a/run.sh", &body), ("a/own.sh", "#!/bin/sh\n")],
    )?;
    assert!(
        needs(r.path(), "a").is_empty(),
        "{:?}",
        needs(r.path(), "a")
    );
    assert!(unresolved_calls(r.path(), r.path()).is_empty());
    Ok(())
}

/// A call nothing resolves is reported, and the node keeps its other edge.
#[test]
fn an_unresolvable_call_is_reported_and_other_edges_stay() -> Result<(), String>
{
    let body =
        format!("{DIR}bash \"$UNKNOWN/tool.sh\"\nbash \"$DIR/../c/x.sh\"\n");
    let r = tree("wave-sh-unres", &[("a/run.sh", &body)])?;
    assert_eq!(needs(r.path(), "a"), vec!["c"]);
    let u = unresolved_calls(r.path(), r.path());
    let got: Vec<(&str, &str, &str)> = u
        .iter()
        .map(|x| (x.node.as_str(), x.script.as_str(), x.call.as_str()))
        .collect();
    assert_eq!(got, vec![("a", "a/run.sh", "$UNKNOWN/tool.sh")]);
    Ok(())
}

/// A tree with no scripts has no unresolved calls, and its Rust schedule is
/// the one `code_deps` always gave -- the self-repo test in `wave.rs` pins
/// the rounds themselves.
#[test]
fn a_tree_without_scripts_reports_nothing_unresolved() -> Result<(), String> {
    let r = TestRepo::new("wave-sh-rust")?;
    r.write("a/SPEC.md", "# SPEC\n")?;
    assert!(unresolved_calls(r.path(), r.path()).is_empty());
    assert!(needs(r.path(), "a").is_empty());
    Ok(())
}

/// The forms a script uses to find itself, each resolved: `cd … && pwd`
/// over `BASH_SOURCE`, a variable built on another, a `${VAR:-default}`,
/// and a script fed to `sh` on stdin.
#[test]
fn the_ways_a_script_finds_its_root_all_resolve() -> Result<(), String> {
    let body = "ROOT=\"$(cd \"$(dirname \"${BASH_SOURCE[0]}\")/..\" && pwd)\"\n\
        HERE=\"$(cd \"$ROOT/a\" 2>/dev/null && pwd)\" || HERE=\"\"\n\
        BASE=\"${OVERRIDE:-$(cd \"$HERE/..\" && pwd)}\"\n\
        bash \"$BASE/b/lib.sh\"\n\
        ssh host sh <\"$ROOT/c/x.sh\"\n\
        bash \"$(cd \"$(dirname \"$0\")\" && pwd)/own.sh\"\n";
    let r = tree(
        "wave-sh-forms",
        &[("a/run.sh", body), ("a/own.sh", "#!/bin/sh\n")],
    )?;
    assert_eq!(needs(r.path(), "a"), vec!["b", "c"]);
    let u = unresolved_calls(r.path(), r.path());
    assert!(u.is_empty(), "{u:?}");
    Ok(())
}

/// An assignment whose value ends in `.sh` runs nothing; a name built at
/// run time and a script's own argument are not guessed.
#[test]
fn an_assignment_runs_nothing_and_a_runtime_name_is_unresolved()
-> Result<(), String> {
    let body = "ROOT=\"$1\"\nTOOL=\"$ROOT/b/lib.sh\"\nbash \"$ROOT/b/lib.sh\"\n\
        bash \"$(dirname \"$0\")/../b/lib-${X}.sh\"\n";
    let r = tree("wave-sh-assign", &[("a/run.sh", body)])?;
    assert!(needs(r.path(), "a").is_empty());
    let calls: Vec<String> = unresolved_calls(r.path(), r.path())
        .into_iter()
        .map(|u| u.call)
        .collect();
    assert_eq!(
        calls,
        vec!["$(dirname $0)/../b/lib-${X}.sh", "$ROOT/b/lib.sh"]
    );
    Ok(())
}
