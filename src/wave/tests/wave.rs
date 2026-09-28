use super::*;

fn dep(node: &str, needs: &[&str]) -> CodeDep {
    seamed(node, needs, &[])
}

/// A node with BLOCKING deps and type-only ones. `schedule` counts the
/// second kind and never waits for it (V4).
fn seamed(node: &str, needs: &[&str], seam: &[&str]) -> CodeDep {
    let own = |xs: &[&str]| -> Vec<String> {
        xs.iter().map(|n| (*n).to_string()).collect()
    };
    CodeDep {
        node: node.to_string(),
        needs: own(needs),
        seam: own(seam),
    }
}

/// Round 1 is what waits for nothing, and a dependent node waits for the
/// round that builds what it names.
#[test]
fn a_node_with_no_sibling_deps_is_ready_and_a_dependent_waits() {
    let s = schedule(&[
        dep("src/cli", &["src/fed"]),
        dep("src/fed", &[]),
        dep("src/tokens", &[]),
    ]);
    assert_eq!(
        s.rounds,
        vec![
            vec!["src/fed".to_string(), "src/tokens".to_string()],
            vec!["src/cli".to_string()],
        ]
    );
    assert!(s.blocked.is_empty(), "{s:?}");
}

/// The two numbers that make the case. A chain of three is three DEEP and
/// one WIDE however many nodes sit beside it, which is what `.:R57`
/// measured about scheduling a DAG by dependency.
#[test]
fn depth_and_width_are_pinned_by_a_known_fixture() {
    let s = schedule(&[
        dep("a", &[]),
        dep("b", &["a"]),
        dep("c", &["b"]),
        dep("d", &[]),
    ]);
    assert_eq!(s.depth(), 3, "a -> b -> c is the critical path: {s:?}");
    assert_eq!(s.width(), 2, "round 1 holds `a` and `d`");
    assert_eq!(schedule(&[]).width(), 0, "nothing to schedule is 0 wide");
    assert_eq!(schedule(&[]).depth(), 0);
}

/// A CYCLE is NAMED rather than looped over, and the rounds computed
/// before it are kept -- a report that threw them away would hide the
/// work that IS schedulable.
#[test]
fn a_cycle_is_reported_rather_than_looped() {
    let s = schedule(&[
        dep("src/a", &["src/b"]),
        dep("src/b", &["src/a"]),
        dep("src/free", &[]),
    ]);
    assert_eq!(s.rounds, vec![vec!["src/free".to_string()]]);
    assert_eq!(
        s.blocked,
        vec!["src/a".to_string(), "src/b".to_string()],
        "both members are named, not just the one reached first"
    );
}

/// A node importing ITSELF never becomes ready either. A loop tested only
/// on pairs spins on this one.
#[test]
fn a_self_edge_is_a_cycle_of_one() {
    let s = schedule(&[dep("src/a", &["src/a"])]);
    assert!(s.rounds.is_empty(), "{s:?}");
    assert_eq!(s.blocked, vec!["src/a".to_string()]);
}

/// A tree the test is HANDED (`src/cli:V6`): two sibling nodes where
/// `src/b` imports `src/a`, and no `§F` table says so.
fn code_dag_fixture() -> Result<crate::testrepo::TestRepo, String> {
    let r = crate::testrepo::TestRepo::new("wave")?;
    let kids = ["a".to_string(), "b".to_string()];
    r.write("SPEC.md", &crate::spec::scaffold(".", &["src".to_string()]))?;
    r.write("src/SPEC.md", &crate::spec::scaffold("src", &kids))?;
    r.write("src/a/SPEC.md", &crate::spec::scaffold("src/a", &[]))?;
    r.write("src/b/SPEC.md", &crate::spec::scaffold("src/b", &[]))?;
    r.write("src/a/mod.rs", "pub fn a() {}\n")?;
    r.write("src/b/mod.rs", "use crate::{a};\npub fn b() {}\n")?;
    Ok(r)
}

/// The schedule follows the CODE dag: `b` imports `a`, so it cannot be in
/// round 1 whatever the federation says.
#[test]
fn the_schedule_follows_the_use_crate_edges() -> Result<(), String> {
    let r = code_dag_fixture()?;
    let s = wave(r.path(), r.path());
    assert_eq!(s.depth(), 2, "b waits for a: {s:?}");
    assert!(
        s.rounds
            .first()
            .is_some_and(|f| f.contains(&"src/a".into())),
        "a waits for nothing: {s:?}"
    );
    assert_eq!(s.rounds.get(1), Some(&vec!["src/b".to_string()]));
    assert!(s.blocked.is_empty());
    Ok(())
}

/// V4 (`B1`). `.:R57` recorded a ten-node repository that `wave` called
/// five rounds deep and that seven workers in fact built in ONE, because
/// a seam commit had declared every node's public types first. This is
/// that tree in miniature: `b` names a TYPE of `a` and nothing else, so
/// it does not wait for `a`'s logic -- and the edge is still reported,
/// because it is still an edge.
#[test]
fn an_import_naming_only_a_type_is_an_edge_and_not_a_wait() -> Result<(), String>
{
    let r = code_dag_fixture()?;
    r.write("src/a/mod.rs", "pub struct Level;\npub fn a() {}\n")?;
    r.write("src/b/mod.rs", "use crate::a::Level;\npub fn b() {}\n")?;

    let s = wave(r.path(), r.path());
    assert_eq!(s.depth(), 1, "one round, behind a seam: {s:?}");
    assert_eq!(s.width(), 4, "every node of the fixture at once: {s:?}");
    assert_eq!((s.edges, s.blocking), (1, 0), "an edge, and not a wait");

    // The SAME tree, reaching for behaviour instead: back to two rounds.
    r.write("src/b/mod.rs", "use crate::a::a;\npub fn b() {}\n")?;
    let s = wave(r.path(), r.path());
    assert_eq!(s.depth(), 2, "a function is a wait: {s:?}");
    assert_eq!((s.edges, s.blocking), (1, 1));

    // And one behavioural reach among type reaches keeps the wait: a
    // sibling you must wait for is not made safe by also naming a type.
    r.write(
        "src/b/mod.rs",
        "use crate::a::Level;\nuse crate::a::a;\npub fn b() {}\n",
    )?;
    let s = wave(r.path(), r.path());
    assert_eq!(s.depth(), 2, "{s:?}");
    assert_eq!((s.edges, s.blocking), (1, 1), "counted ONCE, blocking");
    Ok(())
}

/// `schedule` never waits for a type-only edge, and still counts it. The
/// two numbers are what `B1` was missing: reporting only the total made
/// `depth` read as a fact rather than an upper bound.
#[test]
fn a_type_only_edge_is_counted_and_never_scheduled_against() {
    let s = schedule(&[
        seamed("src/rules", &[], &["src/lint"]),
        dep("src/lint", &[]),
    ]);
    assert_eq!(s.depth(), 1, "{s:?}");
    assert_eq!((s.edges, s.blocking), (1, 0));
}

/// `V1`, from the other side: `§F` makes `a` and `b` CO-CHILDREN with no
/// edge between them, so a scheduler reading the federation would start
/// both in round 1 and hand two workers a pair of files one of which
/// needs the other. Same tree, two graphs, two answers.
#[test]
fn the_federation_table_would_have_scheduled_them_together()
-> Result<(), String> {
    let r = code_dag_fixture()?;
    let path = r.path().join("src/SPEC.md");
    let src = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let children: Vec<String> =
        fed::edges(&src).iter().map(|e| e.dir.clone()).collect();
    assert_eq!(
        children,
        vec!["a".to_string(), "b".to_string()],
        "the §F table declares them as co-children, with no edge between"
    );
    assert_eq!(wave(r.path(), r.path()).depth(), 2);
    Ok(())
}

/// `[dir]` scopes the wave, and an edge LEAVING the scope is dropped
/// rather than left unsatisfiable.
#[test]
fn a_scoped_wave_drops_an_edge_that_leaves_the_scope() -> Result<(), String> {
    let r = code_dag_fixture()?;
    let s = wave(r.path(), &r.path().join("src/b"));
    assert_eq!(s.rounds, vec![vec!["src/b".to_string()]], "{s:?}");
    assert!(s.blocked.is_empty(), "the dropped edge is not a cycle");
    Ok(())
}
