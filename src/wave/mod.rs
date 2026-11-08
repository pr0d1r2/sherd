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

mod cycle;
mod shell;

pub use cycle::cycles;

/// A script invocation that resolves to no tracked script (V5): reported,
/// never guessed, because a missed edge schedules a node before the one it
/// calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unresolved {
    /// The node owning the script, e.g. `scripts`.
    pub node: String,
    /// The script, relative to the root.
    pub script: String,
    /// The invoked path as written, e.g. `$TOOLS/x.sh`.
    pub call: String,
}

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
    let scripts = fed::script_files(root);
    nodes
        .iter()
        .map(|n| {
            let (mut needs, mut seam) = edges_of(root, n, &nodes);
            needs.extend(script_edges(root, n, &nodes, &scripts));
            dedup(&mut needs);
            seam.retain(|l| !needs.contains(l));
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

/// The siblings a node's SCRIPTS invoke (V5) -- always blocking: shell has
/// no type-only reach. A call inside the node, or to a script under no
/// sibling, is no sibling edge.
fn script_edges(
    root: &Path,
    node: &Path,
    nodes: &[PathBuf],
    all: &[PathBuf],
) -> Vec<String> {
    let sibs: Vec<PathBuf> = nodes
        .iter()
        .filter(|n| n.as_path() != node && n.parent() == node.parent())
        .cloned()
        .collect();
    let mut out = Vec::new();
    for (_, target) in script_calls(root, node, nodes, all) {
        if let shell::Target::Script(p) | shell::Target::Inferred(p) = target
            && let Some(s) = sibs.iter().find(|s| p.starts_with(s))
        {
            out.push(fed::node_label(root, s));
        }
    }
    out
}

/// Every invocation in every script a node owns, with the script it is in.
fn script_calls(
    root: &Path,
    node: &Path,
    nodes: &[PathBuf],
    all: &[PathBuf],
) -> Vec<(PathBuf, shell::Target)> {
    let mut out = Vec::new();
    for script in fed::owned_script_files(node, nodes) {
        let Ok(text) = std::fs::read_to_string(&script) else {
            continue;
        };
        let cx = shell::Context {
            script: &script,
            root,
            all,
        };
        for t in shell::calls(&text, &cx) {
            out.push((script.clone(), t));
        }
    }
    out
}

/// The script invocations under `dir` that resolve to no tracked script
/// (V5), sorted by script then call so a report is stable.
#[must_use]
pub fn unresolved_calls(root: &Path, dir: &Path) -> Vec<Unresolved> {
    let nodes = fed::discover(root);
    let all = fed::script_files(root);
    let mut out: Vec<Unresolved> = Vec::new();
    for node in nodes.iter().filter(|n| n.starts_with(dir)) {
        for (script, target) in script_calls(root, node, &nodes, &all) {
            if let shell::Target::Unresolved(call) = target {
                out.push(Unresolved {
                    node: fed::node_label(root, node),
                    script: fed::node_label(root, &script),
                    call,
                });
            }
        }
    }
    out.sort_by(|a, b| {
        a.script.cmp(&b.script).then_with(|| a.call.cmp(&b.call))
    });
    out.dedup();
    out
}

/// How one node reaches another (V4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeKind {
    /// The caller waits for the callee's behaviour.
    Blocking,
    /// The caller names only the callee's public types.
    TypeOnly,
}

impl EdgeKind {
    /// The word a report spells it with.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blocking => "blocking",
            Self::TypeOnly => "type-only",
        }
    }
}

/// One edge between two DIFFERENT nodes: `from` calls `to` (V8).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Edge {
    /// The caller's label.
    pub from: String,
    /// The callee's label: the node that OWNS the called code.
    pub to: String,
    /// Blocking, or type-only (V4).
    pub kind: EdgeKind,
    /// Whether `to` is, or lies under, a co-child of `from`'s parent: the
    /// edges that shape the rounds (V1). The rest are calls a consumer still
    /// needs.
    pub sibling: bool,
    /// Whether every call behind it was INFERRED from a root-relative tail
    /// after an unbound variable (V6, V7), rather than resolved lexically.
    pub inferred: bool,
}

/// Every edge between two different nodes in scope, sorted (V3, V8).
///
/// Rust edges are the sibling ones `code_deps` reads. Shell edges go to the
/// node that OWNS the called script (`src/fed:V15`) -- sibling, parent,
/// cousin or child alike -- because a consumer asking "who can break if
/// this node changes" needs every caller, not only the ones that order a
/// wave. A pair reached both ways is one BLOCKING edge, and it is inferred
/// only when no call behind it resolved lexically.
#[must_use]
pub fn edges(root: &Path, dir: &Path) -> Vec<Edge> {
    let nodes = fed::discover(root);
    let all = fed::script_files(root);
    let mut out: Vec<Edge> = Vec::new();
    for node in nodes.iter().filter(|n| n.starts_with(dir)) {
        for e in rust_edges(root, node, &nodes)
            .into_iter()
            .chain(shell_edges(root, dir, node, &nodes, &all))
        {
            merge(&mut out, e);
        }
    }
    let scope: Vec<String> = nodes
        .iter()
        .filter(|n| n.starts_with(dir))
        .map(|n| fed::node_label(root, n))
        .collect();
    out.retain(|e| scope.contains(&e.to));
    out.sort();
    out
}

/// One node's `use crate::` edges: siblings only, never inferred (V4).
fn rust_edges(root: &Path, node: &Path, nodes: &[PathBuf]) -> Vec<Edge> {
    let from = fed::node_label(root, node);
    let (needs, seam) = edges_of(root, node, nodes);
    let edge = |to: String, kind| Edge {
        from: from.clone(),
        to,
        kind,
        sibling: true,
        inferred: false,
    };
    needs
        .into_iter()
        .map(|t| edge(t, EdgeKind::Blocking))
        .chain(seam.into_iter().map(|t| edge(t, EdgeKind::TypeOnly)))
        .collect()
}

/// One node's script calls into OTHER nodes under `dir`, each to the node
/// owning the called script (V8).
fn shell_edges(
    root: &Path,
    dir: &Path,
    node: &Path,
    nodes: &[PathBuf],
    all: &[PathBuf],
) -> Vec<Edge> {
    let from = fed::node_label(root, node);
    let mut out = Vec::new();
    for (_, target) in script_calls(root, node, nodes, all) {
        let (p, inferred) = match target {
            shell::Target::Script(p) => (p, false),
            shell::Target::Inferred(p) => (p, true),
            shell::Target::Unresolved(_) => continue,
        };
        let Some(owner) = owner_of(&p, nodes) else {
            continue;
        };
        if owner == node || !owner.starts_with(dir) {
            continue;
        }
        out.push(Edge {
            from: from.clone(),
            to: fed::node_label(root, owner),
            kind: EdgeKind::Blocking,
            sibling: under_sibling(node, owner, nodes),
            inferred,
        });
    }
    out
}

/// What `script_edges` counts: the owner is, or lies under, a sibling of
/// `node` -- a call that orders the rounds (V1).
fn under_sibling(node: &Path, owner: &Path, nodes: &[PathBuf]) -> bool {
    nodes.iter().any(|s| {
        s.as_path() != node
            && s.parent() == node.parent()
            && owner.starts_with(s)
    })
}

/// Fold one edge into the list: blocking beats type-only, and a lexical
/// call clears `inferred` (V7).
fn merge(out: &mut Vec<Edge>, e: Edge) {
    if let Some(have) =
        out.iter_mut().find(|h| h.from == e.from && h.to == e.to)
    {
        have.kind = have.kind.min(e.kind);
        have.inferred &= e.inferred;
        return;
    }
    out.push(e);
}

/// The deepest node whose directory holds a path (`src/fed:V15`).
fn owner_of<'a>(p: &Path, nodes: &'a [PathBuf]) -> Option<&'a Path> {
    nodes
        .iter()
        .filter(|n| p.starts_with(n))
        .max_by_key(|n| n.components().count())
        .map(PathBuf::as_path)
}

/// The schedule and its cycles, over the same scoped DAG `wave` uses.
#[must_use]
pub fn wave_with_cycles(
    root: &Path,
    dir: &Path,
) -> (Schedule, Vec<Vec<String>>) {
    let deps = scoped_deps(root, dir);
    let s = schedule(&deps);
    let c = cycles(&deps, &s.blocked);
    (s, c)
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
    schedule(&scoped_deps(root, dir))
}

/// The code DAG restricted to the nodes under `dir` (V2).
fn scoped_deps(root: &Path, dir: &Path) -> Vec<CodeDep> {
    let deps = code_deps(root);
    let keep: Vec<String> = deps
        .iter()
        .map(|d| d.node.clone())
        .filter(|l| node_path(root, l).starts_with(dir))
        .collect();
    scoped(&deps, &keep)
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
#[path = "tests/wave.rs"]
mod wave_tests;

#[cfg(test)]
#[path = "tests/shell.rs"]
mod shell_tests;

#[cfg(test)]
#[path = "tests/shell_var.rs"]
mod shell_var_tests;

#[cfg(test)]
#[path = "tests/edges.rs"]
mod edges_tests;
