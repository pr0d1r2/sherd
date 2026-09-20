//! The SCHEDULE a parallel build follows: the code DAG, the ready set per
//! round, and the two numbers that make the case -- depth and width.
//!
//! Split out of `src/plan` (`src/plan:T14`, `.:B29`). That node's subject is
//! what to attempt NEXT and why it might not survive contact; scheduling a
//! parallel build over the code DAG is a different question, with its own
//! graph, its own rule (V1) and its own future occupant -- the executor half
//! frozen until rung 0.7 needs somewhere to live that is not the planner.

use crate::{code, fed};
use std::path::{Path, PathBuf};

/// One node of the CODE dag: what it is called, and what it waits for.
///
/// The CODE dag is NOT the federation dag (`V1`). These edges come from
/// `use crate::` imports between SIBLING nodes; the federation's come from
/// the `§F` tables and run parent to child. Two graphs over the same
/// directories, and conflating them ships a schedule nobody can run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeDep {
    /// The node's path relative to the repository root -- `src/fed`, or `.`.
    pub node: String,
    /// The siblings it must WAIT for: it reaches into their behaviour.
    pub needs: Vec<String>,
    /// The siblings it names only by TYPE, which a seam commit can satisfy
    /// before either node is written (V4). These are edges, and they are not
    /// blocking -- `schedule` never waits for one.
    pub seam: Vec<String>,
}

/// The rounds a wave would run, and what no round can reach.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Schedule {
    /// Round 1 first. Every node in a round may be built AT ONCE, because
    /// nothing in it waits for anything else in it.
    pub rounds: Vec<Vec<String>>,
    /// Nodes no round can reach: a CYCLE in the code DAG. Named rather than
    /// looped over, and not a defect (`V1`).
    pub blocked: Vec<String>,
    /// Every sibling edge the DAG carries -- blocking and type-only both.
    pub edges: usize,
    /// The edges that decided these rounds. The gap between this and
    /// `edges` is what a seam commit buys, and reporting only one of the two
    /// is what made `depth` read as a fact rather than an upper bound (V4,
    /// B1).
    pub blocking: usize,
}

impl Schedule {
    /// DEPTH -- the critical path, the rounds a wave cannot avoid.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.rounds.len()
    }

    /// WIDTH -- the most workers the wave can ever keep busy at once.
    #[must_use]
    pub fn width(&self) -> usize {
        self.rounds.iter().map(Vec::len).max().unwrap_or(0)
    }
}

/// The CODE dag of a repository: per node, the siblings it waits for, and
/// the siblings it names only by type (V4).
#[must_use]
pub fn code_deps(root: &Path) -> Vec<CodeDep> {
    let nodes = fed::discover(root);
    nodes
        .iter()
        .map(|n| {
            let (needs, seam) = edges_of(root, n, &nodes);
            CodeDep {
                node: fed::node_label(root, n),
                needs,
                seam,
            }
        })
        .collect()
}

/// One node's sibling edges, split into BLOCKING and type-only (V4).
///
/// Resolved against SIBLINGS: a crate module path is one segment, and the
/// nodes that can carry that segment are the co-children of the same parent
/// -- `use crate::fed` inside `src/cli` is `src/fed`. A name matching no
/// sibling resolves to nothing and is dropped, which is what happens to
/// `std`, to a module that is not a node, and to a declaration inside a
/// string literal (`src/code:§C` states that trade).
///
/// An edge is type-only when EVERY item the line names is a public type the
/// sibling declares. One behavioural reach makes the whole edge blocking, and
/// a node reached both ways appears only in `needs`: a sibling you must wait
/// for is not made safe by also naming one of its types.
fn edges_of(
    root: &Path,
    node: &Path,
    nodes: &[PathBuf],
) -> (Vec<String>, Vec<String>) {
    let sibs = siblings_of(root, node, nodes);
    let mut needs: Vec<String> = Vec::new();
    let mut seam: Vec<String> = Vec::new();
    for src in owned_sources(node, nodes) {
        for import in code::crate_imports(&src) {
            let Some(label) = sibling_label(&sibs, &import.module) else {
                continue;
            };
            let target = node_path(root, &label);
            if type_only(&import, &declared_types(&target, nodes)) {
                seam.push(label);
            } else {
                needs.push(label);
            }
        }
    }
    dedup(&mut needs);
    dedup(&mut seam);
    seam.retain(|l| !needs.contains(l));
    (needs, seam)
}

/// Does this import name ONLY types the target declares?
///
/// An import of the module itself (`use crate::lint;`) names no item and is
/// never type-only: what the importer does with it is a call, and the line
/// does not say which.
fn type_only(import: &code::Import, types: &[String]) -> bool {
    !import.items.is_empty() && import.items.iter().all(|i| types.contains(i))
}

/// The names of the public types a node declares.
fn declared_types(node: &Path, nodes: &[PathBuf]) -> Vec<String> {
    code::types_in(&owned_sources(node, nodes))
        .into_iter()
        .map(|t| t.name)
        .collect()
}

fn dedup(v: &mut Vec<String>) {
    v.sort();
    v.dedup();
}

/// The label of the sibling a crate module name resolves to, if any.
fn sibling_label(sibs: &[(String, String)], name: &str) -> Option<String> {
    sibs.iter()
        .find(|(n, _)| n == name)
        .map(|(_, label)| label.clone())
}

/// The co-children of a node's parent, as `(directory name, label)`.
fn siblings_of(
    root: &Path,
    node: &Path,
    nodes: &[PathBuf],
) -> Vec<(String, String)> {
    let parent = node.parent();
    nodes
        .iter()
        .filter(|n| n.as_path() != node && n.parent() == parent)
        .filter_map(|n| {
            let name = n.file_name()?.to_string_lossy().to_string();
            Some((name, fed::node_label(root, n)))
        })
        .collect()
}

/// The text of every `.rs` file a node OWNS (`src/fed:V15`).
///
/// A file that cannot be read is skipped: this REPORTS, and a file nobody
/// can read is not an edge it may claim.
fn owned_sources(node: &Path, nodes: &[PathBuf]) -> Vec<String> {
    fed::owned_rust_files(node, nodes)
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .collect()
}

/// The rounds a wave would run, over a code DAG it is HANDED (`src/cli:V6`).
///
/// Round 1 is every node that waits for nothing; round 2 is what becomes
/// ready once round 1 lands, and so on -- the READY SET, recomputed after
/// each round. A node that never becomes ready is in a CYCLE, and it is
/// NAMED rather than looped over: `use crate::` cycles are ordinary, legal
/// Rust, so they bound how parallel a build can be rather than breaking a
/// rule (`V1`).
#[must_use]
pub fn schedule(deps: &[CodeDep]) -> Schedule {
    let mut out = Schedule {
        blocking: deps.iter().map(|d| d.needs.len()).sum(),
        edges: deps
            .iter()
            .map(|d| d.needs.len().saturating_add(d.seam.len()))
            .sum(),
        ..Schedule::default()
    };
    let mut built: Vec<String> = Vec::new();
    let mut rest: Vec<&CodeDep> = deps.iter().collect();
    while !rest.is_empty() {
        let (ready, waiting) = split_ready(rest, &built);
        if ready.is_empty() {
            out.blocked = labels(&waiting);
            break;
        }
        built.extend(labels(&ready));
        out.rounds.push(labels(&ready));
        rest = waiting;
    }
    out
}

/// The nodes whose every dependency is already built, and the rest.
fn split_ready<'a>(
    rest: Vec<&'a CodeDep>,
    built: &[String],
) -> (Vec<&'a CodeDep>, Vec<&'a CodeDep>) {
    rest.into_iter()
        .partition(|d| d.needs.iter().all(|n| built.contains(n)))
}

/// Node labels, sorted, so a schedule is stable between runs.
fn labels(ds: &[&CodeDep]) -> Vec<String> {
    let mut out: Vec<String> = ds.iter().map(|d| d.node.clone()).collect();
    out.sort();
    out
}

/// The schedule a wave over `dir` would follow.
///
/// Deterministic and offline, which is the half of `.:V123` sherd owns: the
/// DAG, the ready set, the rounds. WHO writes the code is a named, swappable
/// executor and is not built here (`.:V117`).
#[must_use]
pub fn wave(root: &Path, dir: &Path) -> Schedule {
    let deps = code_deps(root);
    let keep: Vec<String> = deps
        .iter()
        .map(|d| d.node.clone())
        .filter(|l| node_path(root, l).starts_with(dir))
        .collect();
    schedule(&scoped(&deps, &keep))
}

/// The directory a node label names.
fn node_path(root: &Path, label: &str) -> PathBuf {
    if label == "." {
        root.to_path_buf()
    } else {
        root.join(label)
    }
}

/// The DAG restricted to a set of nodes.
///
/// A dependency LEAVING the scope is dropped rather than left unsatisfiable:
/// it is not part of this wave, so it is not something this wave waits for.
/// Keeping it would report every scoped node blocked -- a cycle report for a
/// tree that has no cycle.
/// The labels of a list that are inside the scope.
fn within(labels: &[String], keep: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter(|l| keep.contains(l))
        .cloned()
        .collect()
}

fn scoped(deps: &[CodeDep], keep: &[String]) -> Vec<CodeDep> {
    deps.iter()
        .filter(|d| keep.contains(&d.node))
        .map(|d| CodeDep {
            node: d.node.clone(),
            needs: within(&d.needs, keep),
            seam: within(&d.seam, keep),
        })
        .collect()
}

#[cfg(test)]
mod wave_tests {
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
    fn an_import_naming_only_a_type_is_an_edge_and_not_a_wait()
    -> Result<(), String> {
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
    fn a_scoped_wave_drops_an_edge_that_leaves_the_scope() -> Result<(), String>
    {
        let r = code_dag_fixture()?;
        let s = wave(r.path(), &r.path().join("src/b"));
        assert_eq!(s.rounds, vec![vec!["src/b".to_string()]], "{s:?}");
        assert!(s.blocked.is_empty(), "the dropped edge is not a cycle");
        Ok(())
    }
}
