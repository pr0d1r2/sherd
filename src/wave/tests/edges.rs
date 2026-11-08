//! V6 and V7: every edge between two nodes, and the cycles among the blocked
//! ones. Each test pairs what must appear with what must not.

use super::*;
use crate::testrepo::TestRepo;

/// Nodes `a`, `b` and `c` under the root, each with a SPEC.md, and the
/// scripts given as `(path, body)`.
fn tree(tag: &str, scripts: &[(&str, &str)]) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    for n in ["a", "b", "c"] {
        r.write(&format!("{n}/SPEC.md"), "# SPEC\n")?;
    }
    for (path, body) in scripts {
        r.write(path, body)?;
    }
    r.commit("tree")?;
    Ok(r)
}

fn pairs(es: &[Edge]) -> Vec<(&str, &str)> {
    es.iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect()
}

/// The fixture sherd#172 asks for: `a/x.sh` calls `$REPO_ROOT/b/y.sh` with
/// REPO_ROOT handed in as an argument. That is the edge `a -> b`, marked
/// inferred, and it is no longer listed as unresolved. A tail naming no
/// tracked script stays unresolved.
#[test]
fn an_unbound_root_variable_resolves_by_its_tail() -> Result<(), String> {
    let body = "REPO_ROOT=\"$1\"\nbash \"$REPO_ROOT/b/y.sh\"\n\
        bash \"$REPO_ROOT/b/missing.sh\"\n";
    let r = tree(
        "wave-edge-root",
        &[("a/x.sh", body), ("b/y.sh", "#!/bin/sh\n")],
    )?;
    let es = edges(r.path(), r.path());
    assert_eq!(pairs(&es), vec![("a", "b")]);
    assert!(es.iter().all(|e| e.inferred && e.sibling), "{es:?}");
    let u: Vec<String> = unresolved_calls(r.path(), r.path())
        .into_iter()
        .map(|x| x.call)
        .collect();
    assert_eq!(u, vec!["$REPO_ROOT/b/missing.sh"]);
    assert_eq!(wave(r.path(), r.path()).depth(), 2);
    Ok(())
}

/// `${VAR:-.}` whose default resolves to nothing falls back to the tail
/// too, and a lexical call to the same node clears `inferred`.
#[test]
fn a_lexical_call_beats_an_inferred_one() -> Result<(), String> {
    let body = "bash \"${ROOT:-.}/b/y.sh\"\nbash b/y.sh\n";
    let r = tree(
        "wave-edge-merge",
        &[("a/x.sh", body), ("b/y.sh", "#!/bin/sh\n")],
    )?;
    let es = edges(r.path(), r.path());
    assert_eq!(pairs(&es), vec![("a", "b")]);
    assert!(es.iter().all(|e| !e.inferred), "{es:?}");
    Ok(())
}

/// A call into the PARENT node is an edge a consumer needs, and it does not
/// shape the rounds: siblings do (V1).
#[test]
fn a_call_to_the_parent_is_an_edge_but_not_a_wait() -> Result<(), String> {
    let r = TestRepo::new("wave-edge-parent")?;
    r.write("p/SPEC.md", "# SPEC\n")?;
    r.write("p/shared.sh", "#!/bin/sh\n")?;
    r.write("p/k/SPEC.md", "# SPEC\n")?;
    r.write("p/k/run.sh", "bash p/shared.sh\n")?;
    r.commit("tree")?;
    let es = edges(r.path(), r.path());
    assert_eq!(pairs(&es), vec![("p/k", "p")]);
    assert!(es.iter().all(|e| !e.sibling && !e.inferred), "{es:?}");
    assert_eq!(wave(r.path(), r.path()).depth(), 1);
    Ok(())
}

/// A call inside one node is no edge, and neither is one leaving the scope.
#[test]
fn own_calls_and_out_of_scope_calls_are_no_edge() -> Result<(), String> {
    let r = tree(
        "wave-edge-scope",
        &[
            ("a/x.sh", "bash a/own.sh\nbash b/y.sh\n"),
            ("a/own.sh", "#!/bin/sh\n"),
            ("b/y.sh", "#!/bin/sh\n"),
        ],
    )?;
    assert_eq!(pairs(&edges(r.path(), r.path())), vec![("a", "b")]);
    assert!(edges(r.path(), &r.path().join("a")).is_empty());
    Ok(())
}

/// `a` and `b` call each other and `c` waits behind them: all three are
/// blocked, and only `a` and `b` form the cycle.
#[test]
fn cycles_group_the_loop_and_leave_out_who_waits_behind_it()
-> Result<(), String> {
    let r = tree(
        "wave-edge-cycle",
        &[
            ("a/x.sh", "bash b/y.sh\n"),
            ("b/y.sh", "bash a/x.sh\n"),
            ("c/z.sh", "bash a/x.sh\n"),
        ],
    )?;
    let (s, c) = wave_with_cycles(r.path(), r.path());
    assert_eq!(s.blocked, vec!["a", "b", "c"]);
    assert_eq!(c, vec![vec!["a".to_string(), "b".to_string()]]);
    Ok(())
}

/// The pure half: a self-loop is a cycle of one, and a tree with nothing
/// blocked has none.
#[test]
fn cycles_over_a_handed_dag() {
    let dep = |n: &str, needs: &[&str]| CodeDep {
        node: n.to_string(),
        needs: needs.iter().map(|s| (*s).to_string()).collect(),
        seam: Vec::new(),
    };
    let deps = vec![dep("a", &["a"]), dep("b", &[])];
    let s = schedule(&deps);
    assert_eq!(cycles(&deps, &s.blocked), vec![vec!["a".to_string()]]);
    assert!(cycles(&[dep("b", &[])], &[]).is_empty());
}

/// This repository's own Rust edges: `src/cli` reaches into `src/wave`'s
/// behaviour, a blocking sibling edge, never inferred.
#[test]
fn rust_imports_are_sibling_edges() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let es = edges(root, root);
    let e = es
        .iter()
        .find(|e| e.from == "src/cli" && e.to == "src/wave");
    assert!(
        e.is_some_and(|e| e.kind == EdgeKind::Blocking
            && e.sibling
            && !e.inferred),
        "{e:?}"
    );
    assert_eq!(EdgeKind::TypeOnly.as_str(), "type-only");
}
