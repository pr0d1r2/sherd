//! `split` -- propose a federation: which rows and modules would come off a
//! node's chain, and where an unmanaged row belongs.
//!
//! Out of `src/plan` (`src/plan:T14`): planning the next step and proposing
//! a structure are two subjects, and they answer to different rules. Every
//! function here PROPOSES and writes nothing -- where law belongs is a
//! judgement over prose, and a tool moving it on its own rewrites law it
//! cannot read.

use std::path::Path;

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
/// by luck of the letter (`src/plan:B17`).
///
/// Grade breaks ties, then name, so the order is stable between runs.
#[must_use]
pub fn rank<'a>(proposed: &'a [Proposed], spec: &str) -> Vec<Ranked<'a>> {
    let mut out: Vec<Ranked<'a>> = proposed
        .iter()
        .map(|node| {
            let (rows, tokens) = row_weight(spec, &node.name);
            Ranked { node, rows, tokens }
        })
        .collect();
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
mod tests {
    use super::*;

    #[test]
    fn propose_moves_a_single_node_row() {
        assert_eq!(
            propose("orphan check: SPEC w/o parent §F row"),
            Proposal::Move("fed")
        );
        assert_eq!(
            propose("tier select from sherd.toml"),
            Proposal::Move("tokens")
        );
    }

    #[test]
    fn propose_decomposes_a_multi_node_row() {
        match propose("`§F`.tokens staleness, tier-tagged") {
            Proposal::Decompose(ns) => assert!(ns.len() > 1, "{ns:?}"),
            other => panic!("expected Decompose, got {other:?}"),
        }
    }

    #[test]
    fn propose_keeps_a_row_with_no_node_vocabulary() {
        assert_eq!(
            propose("report the caveman finding upstream"),
            Proposal::Keep
        );
    }
}

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
/// `sync` writes more `§N`, the weights rise, split proposes more (`src/plan:V18`).
fn naming_rows(spec: &str, name: &str) -> Vec<String> {
    let mut rows = Vec::new();
    let mut structural = false;
    for line in spec.lines() {
        if let Some(section) = line.strip_prefix("## \u{a7}") {
            structural = section.starts_with('F') || section.starts_with('N');
            continue;
        }
        if !structural && names_word(line, name) {
            rows.push(line.to_string());
        }
    }
    rows
}

/// Whole-word match, so `plan` does not match `planning` and `check` does not
/// match `checked`. A substring match reported every row for every module on
/// the first tree this ran against.
///
/// CASE-SENSITIVE, because the earlier lowercasing split the filename
/// `SPEC.md` into `spec` and counted every row naming the FILE as a row about
/// the NODE: 54 rows against 27 real ones, half the column a filename
/// (`src/plan:B17`). Node names are directory names and directories here are
/// lowercase, so the case carries the distinction for free.
fn names_word(line: &str, name: &str) -> bool {
    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|w| w == name)
}

#[cfg(test)]
mod split_tests {
    use super::*;

    #[test]
    fn a_module_named_by_no_row_is_not_a_candidate() {
        let spec = "## \u{a7}V INVARIANTS\n\nV1: the ledger counts fires\n";
        assert!(naming_rows(spec, "ledger").len() == 1);
        assert!(naming_rows(spec, "corpus").is_empty());
    }

    /// `src/plan:B17`: the evidence grade discriminates in ONE of six repositories
    /// measured. Everywhere else every module carries the same grade, which
    /// leaves the spec rows as the only signal -- and alphabetical order
    /// threw it away.
    #[test]
    fn proposals_lead_with_the_heaviest_not_the_alphabetically_first() {
        let spec = "## \u{a7}V INVARIANTS\n\
                    V1: `heavy` does a thing\nV2: `heavy` does another\n\
                    V3: `heavy` again\nV4: `light` once\n";
        let p = |name: &str, e: Evidence| Proposed {
            name: name.to_string(),
            evidence: e,
            members: vec![name.to_string()],
            shared: Vec::new(),
            split_layout: false,
        };
        let nodes = [
            p("light", Evidence::Published),
            p("heavy", Evidence::Published),
        ];
        let order: Vec<&str> = rank(&nodes, spec)
            .iter()
            .map(|r| r.node.name.as_str())
            .collect();
        assert_eq!(order, vec!["heavy", "light"], "heaviest first");
        assert_eq!(rank(&nodes, spec).first().map(|r| r.rows), Some(3));
    }

    /// A grade every module shares ranks nothing, and a reader who takes the
    /// order for a verdict is reading spec rows rather than structure.
    ///
    /// MEASURED: 4 of 6 repositories are perfectly uniform -- `sherd` all
    /// directory, `ashlar` and `metope` all `pub mod`, `microlith` all
    /// declared -- and only `itok` carries a mixed profile.
    #[test]
    fn a_grade_every_module_shares_is_reported_as_ranking_nothing() {
        let p = |name: &str, e: Evidence| Proposed {
            name: name.to_string(),
            evidence: e,
            members: vec![name.to_string()],
            shared: Vec::new(),
            split_layout: false,
        };
        assert!(uniform_evidence(&[
            p("a", Evidence::Published),
            p("b", Evidence::Published)
        ]));
        assert!(!uniform_evidence(&[
            p("a", Evidence::Drawn),
            p("b", Evidence::Published)
        ]));
        // One node ranks nothing either way, and saying so would be noise.
        assert!(!uniform_evidence(&[p("a", Evidence::Drawn)]));
        assert!(!uniform_evidence(&[]));
    }

    /// Whole-word, or every module matches every row: `plan` inside
    /// "planning" and `check` inside "checked" made the first version
    /// propose the entire spec for every candidate.
    #[test]
    fn a_name_matches_as_a_word_and_not_as_a_substring() {
        assert!(names_word("the plan is fixed", "plan"));
        assert!(!names_word("planning is not planning", "plan"));
        assert!(names_word("src/ledger.rs holds it", "ledger"));
        assert!(!names_word("no mention here", "ledger"));
    }

    /// The layout `.:§C` forbids, detected where the node is proposed: both
    /// `<name>.rs` and `<name>/` exist, so neither half can carry a spec
    /// until they are merged. `rekall` has it for `cli`.
    #[test]
    fn a_module_that_is_both_a_file_and_a_directory_is_flagged() {
        let dir = std::env::temp_dir().join(format!(
            "sherd-layout-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let src = dir.join("src");
        let Ok(()) = std::fs::create_dir_all(src.join("both")) else {
            unreachable!("a src dir is creatable")
        };
        let Ok(()) =
            std::fs::write(src.join("lib.rs"), "mod both;\nmod solo;\n")
        else {
            unreachable!("a lib.rs is writable")
        };
        let Ok(()) = std::fs::write(src.join("both.rs"), "") else {
            unreachable!("a module file is writable")
        };
        let Ok(()) = std::fs::write(src.join("solo.rs"), "") else {
            unreachable!("a module file is writable")
        };

        let found = structure(&dir);
        let flagged = |n: &str| {
            found
                .iter()
                .find(|p| p.name == n)
                .is_some_and(|p| p.split_layout)
        };
        assert!(flagged("both"), "both.rs and both/ exist: {found:?}");
        assert!(!flagged("solo"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A FEDERATED tree proposes the nodes it already has, graded
    /// `directory`, and each still carries whatever weight the parent spec
    /// gives it. `src/plan:B14` is what the opposite assumption cost: excluding
    /// already-nodes left the weight column reading zero for every one of
    /// them, in the only kind of repository where the question matters.
    #[test]
    fn a_federated_tree_proposes_and_weighs_its_existing_nodes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let found = structure(root);
        assert!(
            found.iter().all(|p| p.evidence == Evidence::Drawn),
            "every module here is a directory: {found:?}"
        );
        let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
            unreachable!("this repository has a root spec")
        };
        let weighed = found
            .iter()
            .filter(|p| row_weight(&spec, &p.name).0 > 0)
            .count();
        assert!(weighed > 1, "root names its nodes on real rows");
    }
}

// ---- structure: what the code already separated ----

/// Why a module is a federation candidate. Ordered: a stronger grade is a
/// boundary the author drew more explicitly (`src/plan:V17`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Evidence {
    /// Declared in the entry file and nothing more. The weakest grade, and
    /// the one `microlith` is made of: eleven `pub(crate) mod` lines, no
    /// directories, no families, no published surface. A crate can draw
    /// every boundary this way, and dropping the grade made such a crate
    /// propose NOTHING (`src/plan:B13`).
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
/// STRUCTURE FIRST (`src/plan:V17`). Spec rows are attached afterwards by the caller
/// and are evidence ABOUT a node, never the thing that proposes it -- a
/// module the code separates and the prose never mentions is still a node,
/// which a row-count ranking cannot see (`src/plan:B12`).
///
/// Families are reported, never auto-clustered beyond a shared suffix: a
/// graph clustering is where a proposer starts guessing, and `src/plan:V16` says this
/// proposes.
#[must_use]
pub fn structure(dir: &Path) -> Vec<Proposed> {
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
mod structure_tests {
    use super::*;

    /// This crate is already federated, so every module `lib.rs` declares is
    /// a directory: the strongest grade, and nothing left to infer.
    #[test]
    fn an_already_federated_crate_proposes_its_directories() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let found = structure(root);
        assert!(!found.is_empty());
        assert!(
            found.iter().all(|p| p.evidence == Evidence::Drawn),
            "every module of an already-federated crate is a directory: {found:?}"
        );
    }

    /// A family with a shared hub: the `use crate::` intersection across all
    /// members, which is what makes eleven `*cmd` files one node instead of
    /// eleven (`src/plan:V17`).
    #[test]
    fn a_family_is_proposed_with_the_hub_its_members_share() {
        let dir = std::env::temp_dir().join(format!(
            "sherd-family-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let src = dir.join("src");
        let Ok(()) = std::fs::create_dir_all(&src) else {
            unreachable!("a src dir is creatable")
        };
        let write = |name: &str, body: &str| {
            let Ok(()) = std::fs::write(src.join(name), body) else {
                unreachable!("a fixture file is writable")
            };
        };
        write("lib.rs", "mod acmd;\nmod bcmd;\nmod ccmd;\n");
        write("acmd.rs", "use crate::render;\nuse crate::units;\n");
        write("bcmd.rs", "use crate::render;\n");
        write("ccmd.rs", "use crate::render;\n");

        let found = structure(&dir);
        let family = found.iter().find(|p| p.members.len() == 3);
        let Some(family) = family else {
            unreachable!("three of a suffix is a family: {found:?}")
        };
        assert_eq!(family.evidence, Evidence::Cohesion);
        assert_eq!(family.shared, vec!["render"], "units is used by one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A hub is what EVERY member reaches for. `render` is shared; `units`
    /// is used by one member and is not.
    #[test]
    fn a_shared_hub_is_present_in_every_member() {
        let lists = vec![
            vec!["cli".to_string(), "render".to_string(), "units".to_string()],
            vec!["cli".to_string(), "render".to_string()],
        ];
        assert_eq!(shared_across(&lists), vec!["cli", "render"]);
        assert!(shared_across(&[]).is_empty());
    }

    /// Two of a suffix is a coincidence; three is a family. Without the
    /// floor, every `*s` plural in a crate becomes a proposed node.
    #[test]
    fn a_family_needs_more_than_a_pair() {
        let decl = |n: &str| crate::code::ModDecl {
            name: n.to_string(),
            is_pub: false,
        };
        let pair = [decl("acmd"), decl("bcmd")];
        assert!(families(&pair).is_empty());
        let three = [decl("acmd"), decl("bcmd"), decl("ccmd")];
        assert_eq!(families(&three).len(), 1);
    }
}

/// What one node's name costs the spec it is named in: rows and tokens.
///
/// Computed for a node whatever its state, which is the half `candidates`
/// could not do: that function lists modules to PROMOTE, so it excludes
/// directories that are already nodes -- and in a federated repository that
/// is every one of them, leaving the weight column reading zero for the only
/// tree where the question matters (`src/plan:B14`).
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

#[cfg(test)]
mod weight_tests {
    use super::*;

    /// The defect `src/plan:B14` names: this repository's root spec talks about `tdd`
    /// on dozens of rows, and the promotable-candidate path reported zero
    /// because `src/tdd` is already a node.
    #[test]
    fn an_existing_node_still_has_a_weight_in_its_parent() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
            unreachable!("this repository has a root spec")
        };
        let (rows, tokens) = row_weight(&spec, "tdd");
        assert!(rows > 0, "root names `tdd` on real rows");
        assert!(tokens > 0);

        // And a name nothing mentions weighs nothing.
        assert_eq!(row_weight(&spec, "wombat"), (0, 0));
    }

    /// `src/plan:B15`, first cause, on the example that row names: lowercasing split
    /// the filename `SPEC.md` into `spec`, so every row naming the FILE
    /// counted as a row about the NODE. Measured on this repository's own
    /// root spec the column read 54 where a case-sensitive count reads 27.
    #[test]
    fn a_filename_is_not_a_row_about_the_node_it_spells() {
        assert!(!names_word("the node carries `SPEC.md`", "spec"));
        assert!(names_word("`spec` owns the section split", "spec"));

        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Ok(text) = std::fs::read_to_string(root.join("SPEC.md")) else {
            unreachable!("this repository has a root spec")
        };
        let counted = naming_rows(&text, "spec").len();
        let lowercased = text
            .lines()
            .filter(|l| !l.starts_with("## \u{a7}"))
            .filter(|l| {
                l.to_lowercase()
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|w| w == "spec")
            })
            .count();
        assert!(
            lowercased > counted,
            "the lowercasing counted more than the case-sensitive read: \
             {lowercased} against {counted}"
        );
        for row in naming_rows(&text, "spec") {
            assert!(
                names_word(row.as_str(), "spec"),
                "a row was counted only for spelling the FILE: {row}"
            );
        }
    }

    /// `src/plan:B15`, second cause: `sync` GENERATES `§N` into every node and `§N`
    /// names every sibling, so counting it makes this function read its own
    /// generator's output and the loop never converges (`src/plan:V18`). `§F` is
    /// authored rather than generated and is excluded for the same reason --
    /// it is structure, not law.
    #[test]
    fn generated_navigation_is_not_law_about_a_sibling() {
        let spec = "## \u{a7}N NAV\n\nrel|path|lens\nsib|src/tdd|the loop\n\n\
                    ## \u{a7}F FEDERATION\n\ndir|lens|grade\ntdd|the loop|HT\n\n\
                    ## \u{a7}V INVARIANTS\n\nV1: `tdd` refuses a green test\n";
        assert_eq!(
            naming_rows(spec, "tdd"),
            vec!["V1: `tdd` refuses a green test".to_string()],
            "only the §V row is law about `tdd`"
        );
    }

    /// The same, on the tree that measured it: `src/tdd/SPEC.md` names five
    /// siblings and every one of those mentions is a generated `§N` row.
    #[test]
    fn a_leaf_spec_weighs_nothing_for_its_siblings() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Ok(text) = std::fs::read_to_string(root.join("src/tdd/SPEC.md"))
        else {
            unreachable!("this repository has a spec for `src/tdd`")
        };
        assert!(
            text.contains("sib|src/lens"),
            "the nav does list `lens` as a sibling"
        );
        for row in naming_rows(&text, "lens") {
            assert!(
                !row.starts_with("sib|")
                    && !row.starts_with("up|")
                    && !row.starts_with("rel|"),
                "a nav row was weighed as law about a sibling: {row}"
            );
        }
    }
}
