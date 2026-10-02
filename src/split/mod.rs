//! `split` -- propose a federation: which rows and modules would come off a
//! node's chain, and where an unmanaged row belongs.
//!
//! Out of `src/plan` (`src/plan:T14`): planning the next step and proposing
//! a structure are two subjects, and they answer to different rules. Every
//! function here PROPOSES and writes nothing -- where law belongs is a
//! judgement over prose, and a tool moving it on its own rewrites law it
//! cannot read.

use std::path::{Path, PathBuf};

mod shell;

/// One proposal with the spec weight that ranks it.
#[derive(Debug, Clone)]
pub struct Ranked<'a> {
    pub node: &'a Proposed,
    /// Spec rows in the parent naming this module.
    pub rows: usize,
    /// What those rows cost.
    pub tokens: u64,
}

/// Proposals HEAVIEST first.
///
/// The evidence grade discriminates in ONE of six repositories measured --
/// `itok`. In the other five every module carries the same grade, which
/// leaves the spec rows as the only signal present, and alphabetical order
/// threw it away: `metope` spans 0 to 58 rows and led with its 58-row node
/// by luck of the letter (`src/split:B5`).
///
/// Grade breaks ties, then name, so the order is stable between runs.
#[must_use]
pub fn rank<'a>(proposed: &'a [Proposed], spec: &str) -> Vec<Ranked<'a>> {
    let ranked = proposed
        .iter()
        .map(|node| {
            let (rows, tokens) = row_weight(spec, &node.name);
            Ranked { node, rows, tokens }
        })
        .collect();
    heaviest_first(ranked)
}

/// [`rank`] for a tree that may hold scripts (`src/split:V7`): a Rust
/// module is weighed by its name, as `rank` does, and any candidate also by
/// the rows citing a script it would own -- each row once.
///
/// Every script under `dir` is read, not only the candidates', because a
/// basename is ambiguous against a script no candidate owns as much as
/// against one it does.
#[must_use]
pub fn rank_in<'a>(
    dir: &Path,
    proposed: &'a [Proposed],
    spec: &str,
) -> Vec<Ranked<'a>> {
    let all = crate::fed::script_files(dir);
    let modules: Vec<String> =
        modules(dir).into_iter().map(|m| m.name).collect();
    let ranked = proposed
        .iter()
        .map(|node| {
            let owned = scripts_of(dir, node, &all);
            let by_name = modules.contains(&node.name);
            let (rows, tokens) = weight(spec, node, by_name, &owned, &all);
            Ranked { node, rows, tokens }
        })
        .collect();
    heaviest_first(ranked)
}

/// Heaviest first; grade breaks ties, then name, so the order is stable.
fn heaviest_first(mut out: Vec<Ranked<'_>>) -> Vec<Ranked<'_>> {
    out.sort_by(|a, b| {
        b.rows
            .cmp(&a.rows)
            .then_with(|| a.node.evidence.cmp(&b.node.evidence))
            .then_with(|| a.node.name.cmp(&b.node.name))
    });
    out
}

/// Do ALL proposals carry the same grade? Then it ranks nothing, and a reader
/// who takes the order for a verdict is reading spec rows, not structure.
#[must_use]
pub fn uniform_evidence(proposed: &[Proposed]) -> bool {
    let mut grades = proposed.iter().map(|p| p.evidence);
    let Some(first) = grades.next() else {
        return false;
    };
    proposed.len() > 1 && grades.all(|g| g == first)
}

#[cfg(test)]
#[path = "tests/propose.rs"]
mod tests;

// ---- home: where does an unmanaged row belong? ----

/// Node keywords. A row naming exactly one node's vocabulary probably belongs
/// to that node. ADVISORY -- prose classification is wrong-by-default (`src/plan:V4`,
/// `src/plan:B1`), so this proposes and a reader decides.
const VOCAB: [(&str, &[&str]); 9] = [
    (
        "fed",
        &[
            "§f",
            "§n",
            "edge",
            "dag",
            "cycle",
            "chain",
            "discover",
            "graph",
            "orphan",
            "promotion",
        ],
    ),
    ("lens", &["lens", "pack", "budget", "depth", "why", "facet"]),
    ("tokens", &["token", "tier", "itok", "count", "ceiling"]),
    ("spec", &["microlith", "section", "record", "format", "fmt"]),
    ("ollama", &["ollama", "endpoint", "retry", "model"]),
    ("tdd", &["tdd", "judge", "red", "green", "repair"]),
    ("plan", &["plan", "apply", "horizon", "needs", "actionable"]),
    ("review", &["review", "unwired", "negative"]),
    ("state", &["state", "cache", "idempot"]),
];

/// What triage proposes for one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proposal {
    /// Work lands in exactly one node: move the row there.
    Move(&'static str),
    /// Names several nodes' work: split into one row per node.
    Decompose(Vec<&'static str>),
    /// No node vocabulary -- root, CLI, or research. Read it.
    Keep,
}

/// Propose where a row belongs, from the vocabulary it uses.
#[must_use]
pub fn propose(text: &str) -> Proposal {
    let t = text.to_lowercase();
    let hits: Vec<&'static str> = VOCAB
        .iter()
        .filter(|(_, ks)| ks.iter().any(|k| t.contains(k)))
        .map(|(n, _)| *n)
        .collect();
    // The pattern carries the arity: exactly one hit MOVES, and anything
    // else is Keep or Decompose. A `len()` match plus an index said it twice.
    match hits.as_slice() {
        [one] => Proposal::Move(one),
        [] => Proposal::Keep,
        _ => Proposal::Decompose(hits),
    }
}

// ---- split: what would come off this node's chain? ----

/// Spec lines naming one module, whole-word and case-sensitive, outside the
/// STRUCTURAL sections.
///
/// `§F` and `§N` describe the federation rather than state law about it, and
/// `§N` is GENERATED by `sync` into every node, where it names every sibling.
/// Counting them made this function read its own generator's output: split,
/// `sync` writes more `§N`, the weights rise, split proposes more (`src/split:V3`).
fn naming_rows(spec: &str, name: &str) -> Vec<String> {
    law_lines(spec)
        .filter(|line| names_word(line, name))
        .map(str::to_string)
        .collect()
}

/// The spec's lines OUTSIDE the structural sections, `§F` and `§N` (V3), and
/// without the section headers themselves.
fn law_lines(spec: &str) -> impl Iterator<Item = &str> {
    let mut structural = false;
    spec.lines().filter(move |line| {
        if let Some(section) = line.strip_prefix("## \u{a7}") {
            structural = section.starts_with('F') || section.starts_with('N');
            return false;
        }
        !structural
    })
}

/// Whole-word match, so `plan` does not match `planning` and `check` does not
/// match `checked`. A substring match reported every row for every module on
/// the first tree this ran against.
///
/// CASE-SENSITIVE, because the earlier lowercasing split the filename
/// `SPEC.md` into `spec` and counted every row naming the FILE as a row about
/// the NODE: 54 rows against 27 real ones, half the column a filename
/// (`src/split:B5`). Node names are directory names and directories here are
/// lowercase, so the case carries the distinction for free.
fn names_word(line: &str, name: &str) -> bool {
    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|w| w == name)
}

#[cfg(test)]
#[path = "tests/split.rs"]
mod split_tests;

// ---- structure: what the code already separated ----

/// Why a module is a federation candidate. Ordered: a stronger grade is a
/// boundary the author drew more explicitly (`src/split:V2`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Evidence {
    /// Declared in the entry file and nothing more. The weakest grade, and
    /// the one `microlith` is made of: eleven `pub(crate) mod` lines, no
    /// directories, no families, no published surface. A crate can draw
    /// every boundary this way, and dropping the grade made such a crate
    /// propose NOTHING (`src/split:B2`).
    Declared,
    /// A naming family plus shared `use crate::` edges.
    Cohesion,
    /// `pub mod` -- the author published it.
    Published,
    /// Already a directory -- the author drew it.
    Drawn,
}

impl Evidence {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Drawn => "directory",
            Self::Published => "pub mod",
            Self::Cohesion => "family",
            Self::Declared => "declared",
        }
    }
}

/// A proposed node: what the code separated, and what it shares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposed {
    pub name: String,
    pub evidence: Evidence,
    /// Both `<name>.rs` AND `<name>/` exist -- the 2018 layout `.:§C`
    /// forbids, which puts the facade outside the directory it fronts.
    /// `rekall` has it for `cli`, and it has to be merged by hand before
    /// either half can carry a `SPEC.md`.
    pub split_layout: bool,
    /// Members, when a naming family stands in for several files.
    pub members: Vec<String>,
    /// Crate modules every member reaches for -- the hub a family shares.
    pub shared: Vec<String>,
}

/// Nodes derived from the source, strongest evidence first.
///
/// STRUCTURE FIRST (`src/split:V2`). Spec rows are attached afterwards by the caller
/// and are evidence ABOUT a node, never the thing that proposes it -- a
/// module the code separates and the prose never mentions is still a node,
/// which a row-count ranking cannot see (`src/split:B1`).
///
/// Families are reported, never auto-clustered beyond a shared suffix: a
/// graph clustering is where a proposer starts guessing, and `src/split:V1` says this
/// proposes.
///
/// Both readings: the Rust [`modules`], then what a shell codebase drew
/// (`src/split:V7`). A dir both find is proposed once, as the module.
#[must_use]
pub fn structure(dir: &Path) -> Vec<Proposed> {
    let mut out = modules(dir);
    let flat = !dir.join("src").is_dir();
    for c in shell::candidates(dir, &crate::fed::script_files(dir)) {
        let same_dir = flat && c.evidence == Evidence::Drawn;
        if !(same_dir && out.iter().any(|p| p.name == c.name)) {
            out.push(c);
        }
    }
    out
}

/// The RUST reading alone: modules declared in the crate's entry file,
/// graded by `Evidence`, strongest first.
#[must_use]
pub fn modules(dir: &Path) -> Vec<Proposed> {
    let src = if dir.join("src").is_dir() {
        dir.join("src")
    } else {
        dir.to_path_buf()
    };
    let entry = ["lib.rs", "main.rs", "mod.rs"]
        .iter()
        .map(|f| src.join(f))
        .find(|p| p.is_file());
    let decls = entry
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| crate::code::mod_decls(&t))
        .unwrap_or_default();

    let mut out: Vec<Proposed> = Vec::new();
    for family in families(&decls) {
        out.push(proposed_family(&src, family));
    }
    for d in &decls {
        if out.iter().any(|p| p.members.contains(&d.name)) {
            continue;
        }
        let evidence = if src.join(&d.name).is_dir() {
            Evidence::Drawn
        } else if d.is_pub {
            Evidence::Published
        } else {
            Evidence::Declared
        };
        out.push(Proposed {
            split_layout: src.join(&d.name).is_dir()
                && src.join(format!("{}.rs", d.name)).is_file(),
            name: d.name.clone(),
            evidence,
            members: vec![d.name.clone()],
            shared: Vec::new(),
        });
    }
    out.sort_by(|a, b| b.evidence.cmp(&a.evidence).then(a.name.cmp(&b.name)));
    out
}

/// Modules sharing a suffix, when there are enough of them to be a family
/// rather than a coincidence. Two files ending in `cmd` is a pair; eleven is
/// a concern the author named.
fn families(decls: &[crate::code::ModDecl]) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for suffix in ["cmd", "fmt", "args"] {
        let members: Vec<String> = decls
            .iter()
            .map(|d| d.name.clone())
            .filter(|n| n.len() > suffix.len() && n.ends_with(suffix))
            .collect();
        if members.len() >= 3 {
            out.push(members);
        }
    }
    out
}

/// A family, and the crate modules EVERY member reaches for.
fn proposed_family(src: &Path, members: Vec<String>) -> Proposed {
    let uses: Vec<Vec<String>> = members
        .iter()
        .filter_map(|m| {
            std::fs::read_to_string(src.join(format!("{m}.rs"))).ok()
        })
        .map(|t| crate::code::crate_uses(&t))
        .collect();
    let shared = shared_across(&uses);
    let name = members
        .first()
        .and_then(|m| m.get(m.len().saturating_sub(3)..))
        .unwrap_or("group")
        .to_string();
    Proposed {
        name,
        evidence: Evidence::Cohesion,
        members,
        shared,
        // A family is a set of files; the layout question is per-module.
        split_layout: false,
    }
}

/// Names present in EVERY list. A hub is what all members share, not what
/// any of them happens to use.
fn shared_across(lists: &[Vec<String>]) -> Vec<String> {
    let Some(first) = lists.first() else {
        return Vec::new();
    };
    first
        .iter()
        .filter(|n| lists.iter().all(|l| l.contains(n)))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "tests/structure.rs"]
mod structure_tests;

/// What one node's name costs the spec it is named in: rows and tokens.
///
/// Computed for a node whatever its state, which is the half `candidates`
/// could not do: that function lists modules to PROMOTE, so it excludes
/// directories that are already nodes -- and in a federated repository that
/// is every one of them, leaving the weight column reading zero for the only
/// tree where the question matters (`src/split:B3`).
///
/// The question here is the other one: given a node that EXISTS, how much of
/// its parent's spec is about it? Those rows are what a chain pays on every
/// descent and what moving them down would relieve.
#[must_use]
pub fn row_weight(spec: &str, name: &str) -> (usize, u64) {
    let rows = naming_rows(spec, name);
    let tokens = crate::tokens::count(&rows.join("\n")).tokens;
    (rows.len(), tokens)
}

/// What a candidate costs the spec: rows naming it, when it is a Rust
/// module (`by_name`, V2), or citing a script it OWNS (V7) -- each row once.
/// A shell dir's name is a word, and only its scripts are citations (V4).
///
/// `all` is every script under the node being split, so a basename two of
/// them share resolves to neither; [`ambiguous_scripts`] names those.
fn weight(
    spec: &str,
    p: &Proposed,
    by_name: bool,
    owned: &[PathBuf],
    all: &[PathBuf],
) -> (usize, u64) {
    let rows: Vec<&str> = law_lines(spec)
        .filter(|line| {
            (by_name && names_word(line, &p.name))
                || shell::script_tokens(line).any(|t| {
                    matches!(shell::resolve(t, all).as_slice(), [one] if owned.contains(one))
                })
        })
        .collect();
    let tokens = crate::tokens::count(&rows.join("\n")).tokens;
    (rows.len(), tokens)
}

/// The scripts a candidate under `dir` would own (`src/split:V7`): every
/// script beneath its directory, or a family's member files. Empty for a
/// Rust module with no scripts. `all` is [`crate::fed::script_files`] of
/// `dir`, passed in so a caller weighing many candidates walks once.
#[must_use]
pub fn scripts_of(dir: &Path, p: &Proposed, all: &[PathBuf]) -> Vec<PathBuf> {
    let home = dir.join(&p.name);
    if home.is_dir() {
        return all
            .iter()
            .filter(|s| s.starts_with(&home))
            .cloned()
            .collect();
    }
    let files: Vec<PathBuf> = p
        .members
        .iter()
        .map(|m| dir.join(format!("{m}.sh")))
        .collect();
    all.iter().filter(|s| files.contains(s)).cloned().collect()
}

/// Cited script names that resolve to two or more scripts, sorted. They
/// count for no candidate, and a reader is told which (`src/split:V7`).
#[must_use]
pub fn ambiguous_scripts(spec: &str, scripts: &[PathBuf]) -> Vec<String> {
    let mut out: Vec<String> = law_lines(spec)
        .flat_map(shell::script_tokens)
        .filter(|t| shell::resolve(t, scripts).len() > 1)
        .map(str::to_string)
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
#[path = "tests/weight.rs"]
mod weight_tests;

#[cfg(test)]
#[path = "tests/shell.rs"]
mod shell_tests;
