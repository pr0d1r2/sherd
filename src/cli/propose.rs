//! `split`, `seam` and `wave` -- the verbs that PROPOSE a structure and write nothing.

use super::*;

/// `sherd split <dir> [--apply]` -- propose a federation split, write nothing.
///
/// A PROPOSAL, and the refusal to write without `--apply` is the point: which
/// module owns which rule is a judgement, and a tool that moved spec rows on
/// its own would be rewriting law it cannot read. `--apply` is not built, and
/// says so rather than silently doing nothing.
pub(super) fn split_cmd(root: &Path, dir: &Path, apply: bool) -> ExitCode {
    if apply {
        eprintln!(
            "sherd: --apply is not built. `split` proposes; moving rows between \
             specs is a judgement a reader makes."
        );
        return ExitCode::from(2);
    }
    if !dir.join("SPEC.md").is_file() {
        eprintln!("sherd: {} carries no SPEC.md", dir.display());
        return ExitCode::from(2);
    }
    let (cost, ceiling) = split_budget(root, dir);
    println!(
        "{}: chain {cost} tok of {ceiling}",
        fed::node_label(root, dir)
    );

    // STRUCTURE FIRST (`.:src/plan:V17`): what the code already separated, then
    // the prose weight of each. A module the spec never mentions is still a
    // node; a ranking by rows cannot see it (`.:src/plan:B12`).
    let proposed = plan::structure(dir);
    if proposed.is_empty() {
        println!("  no module declarations found -- nothing to propose");
        return ExitCode::SUCCESS;
    }
    let spec = std::fs::read_to_string(dir.join("SPEC.md")).unwrap_or_default();
    print_structure(&proposed, &spec);
    ExitCode::SUCCESS
}

/// `sherd seam [dir]` -- the vocabulary a parallel build needs.
///
/// Per node, the PUBLIC TYPES it declares: the names a sibling can spell
/// before either node is written. `.:R57` is the measurement -- a sibling
/// repo built seven federated nodes in parallel, one worker each, after ONE
/// commit declared every node's types, and scheduling the same DAG by
/// dependency instead was five deep with at most three nodes ever in flight.
///
/// Report-only, like `split` and for the same reason: WHICH of these names
/// are shared is a judgement a reader makes.
pub(super) fn seam_cmd(root: &Path, dir: &Path) -> ExitCode {
    println!("seam -- the public types a node declares\n");
    let nodes = fed::discover(root);
    let mut examined = 0usize;
    let mut declared = 0usize;
    for node in nodes.iter().filter(|n| n.starts_with(dir)) {
        let types = node_types(node, &nodes);
        examined = examined.saturating_add(1);
        declared = declared.saturating_add(types.len());
        print_seam(&fed::node_label(root, node), &types);
    }
    seam_summary(examined, declared);
    // Examining NOTHING is not passing, the same shape `budget` records:
    // an empty table and a repo with no types read identically (`B5`).
    if examined == 0 {
        eprintln!("{}", no_node(root, dir));
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// What was examined, not only what was found (`.:V48`).
pub(super) fn seam_summary(examined: usize, declared: usize) {
    println!("\n  {examined} node(s) examined · {declared} public type(s)");
    println!(
        "  A parallel build needs these names BEFORE any node is written. \
         WHICH of them are shared is a judgement this does not make."
    );
}

/// One node's row, and its vocabulary beneath it.
///
/// A node declaring nothing is still PRINTED: absence is an answer -- this
/// node adds no name a sibling has to know -- and a silent row would be the
/// conflation `V12` records (`B6`).
pub(super) fn print_seam(label: &str, types: &[code::PubType]) {
    if types.is_empty() {
        println!("  {label:<24} no public types");
        return;
    }
    println!("  {label:<24} {} type(s)", types.len());
    for t in types {
        println!("      {} {}", t.kind, t.name);
    }
}

/// `sherd wave [dir]` -- the SCHEDULE a parallel build would follow.
///
/// The DETERMINISTIC half of `.:V123`, and only that half: the code DAG, the
/// ready set of each round, and the two numbers that decide whether a
/// parallel build is worth running -- DEPTH, the rounds it cannot avoid, and
/// WIDTH, the most workers it can ever keep busy. No worktree is created, no
/// worker is started, no executor is named and no model is contacted. WHO
/// writes the code is a swappable adapter whose choice changes nothing here,
/// and the model half is frozen until rung 0.7 (`.:V117`).
///
/// REPORT-ONLY, like `seam` and `split`, so a CYCLE exits 0. A `use crate::`
/// cycle is legal, ordinary Rust; it bounds how parallel a build can be
/// rather than breaking a rule, and naming it is the whole finding. `check`
/// and `validate` are the verbs that hold verdicts, and `.:V4`'s cycle rule
/// is about the FEDERATION dag -- a different graph over the same
/// directories (`src/wave:V1`). Exit 2 stays for a dir matching no node:
/// examining nothing is not passing (`B5`).
pub(super) fn wave_cmd(root: &Path, dir: &Path) -> ExitCode {
    println!("wave -- the schedule a parallel build would follow\n");
    let s = wave::wave(root, dir);
    for (i, round) in s.rounds.iter().enumerate() {
        print_round(i.saturating_add(1), round);
    }
    print_blocked(&s.blocked);
    let examined = s
        .rounds
        .iter()
        .map(Vec::len)
        .sum::<usize>()
        .saturating_add(s.blocked.len());
    wave_summary(&s, examined);
    if examined == 0 {
        eprintln!("{}", no_node(root, dir));
        return ExitCode::from(2);
    }
    ExitCode::SUCCESS
}

/// One round: how many nodes may be built at once, and which.
pub(super) fn print_round(n: usize, nodes: &[String]) {
    println!("  round {n:<3} {:>2} node(s)", nodes.len());
    for node in nodes {
        println!("      {node}");
    }
}

/// Nodes no round can reach. A FINDING, not a failure: they import each
/// other, so they are one unit of work rather than a broken repository.
pub(super) fn print_blocked(blocked: &[String]) {
    if blocked.is_empty() {
        return;
    }
    println!("\n  BLOCKED -- a cycle in the code DAG, so no round reaches:");
    for node in blocked {
        println!("      {node}");
    }
    println!(
        "      these import each other, so they are ONE unit of work -- a \
         wave cannot split them across workers."
    );
}

/// The two numbers that make the case, then what this did NOT do (`.:V48`).
pub(super) fn wave_summary(s: &wave::Schedule, examined: usize) {
    println!(
        "\n  {examined} node(s) examined · depth {} · width {}",
        s.depth(),
        s.width()
    );
    println!(
        "  DEPTH is the critical path -- rounds a wave cannot avoid. WIDTH \
         is the most workers it can ever keep busy at once."
    );
    wave_edges(s);
    wave_notes();
}

/// Both edge counts, because only one of them decided the rounds
/// (`src/wave:V4`, `src/wave:B1`).
///
/// A type-only import is an edge and is not a wait: once a seam commit has
/// declared the vocabulary, `use crate::lint::Level` is a reference to a type
/// that already exists. Reporting the total alone made `depth` read as a
/// fact, when on a repository built behind a seam it was an upper bound
/// nobody could see past.
pub(super) fn wave_edges(s: &wave::Schedule) {
    let seam = s.edges.saturating_sub(s.blocking);
    println!(
        "  {} sibling edge(s) · {} blocking · {seam} type-only",
        s.edges, s.blocking
    );
    if seam > 0 {
        println!(
            "      the type-only ones name a sibling's public TYPES and \
             nothing else, so a `sherd seam` commit satisfies them before \
             any node is written -- they are not waits."
        );
    }
}

/// What the edges ARE, and what the verb refused to do.
pub(super) fn wave_notes() {
    println!(
        "  Edges are `use crate::` imports between sibling nodes: the CODE \
         dag, NOT the §F federation dag (`src/wave:V1`)."
    );
    println!(
        "  Nothing was written: no worktree, no worker, no executor, no \
         model. WHO writes the code is named and swappable (`.:V123`)."
    );
}

/// The public types one node declares, sorted and deduplicated.
pub(super) fn node_types(node: &Path, nodes: &[PathBuf]) -> Vec<code::PubType> {
    let sources: Vec<String> = fed::owned_rust_files(node, nodes)
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .collect();
    code::types_in(&sources)
}

/// The proposal: what the code separated, graded, with the prose weight of
/// each node beside it.
///
/// `rows` is EVIDENCE ABOUT a node rather than the reason for it -- a `0`
/// there means the spec never mentions a module the author already split
/// out, which is a gap in the spec and not a reason to skip the node.
pub(super) fn print_structure(proposed: &[plan::Proposed], spec: &str) {
    // Heaviest FIRST. The evidence grade discriminates in ONE of six
    // repositories measured -- `itok` -- and in the other five every module
    // carries the same grade, which leaves the row weight as the only signal
    // present. Alphabetical order threw it away: `metope` spans 0 to 58 rows
    // and put its 58-row node first by luck of the letter b (`plan:B16`).
    let mut ranked: Vec<(&plan::Proposed, (usize, u64))> = proposed
        .iter()
        .map(|p| (p, plan::row_weight(spec, &p.name)))
        .collect();
    ranked.sort_by(|a, b| {
        b.1.0
            .cmp(&a.1.0)
            .then_with(|| a.0.evidence.cmp(&b.0.evidence))
            .then_with(|| a.0.name.cmp(&b.0.name))
    });
    println!("\n  node           evidence    rows   tok  members");
    for (p, (rows, tokens)) in ranked {
        let members = if p.members.len() > 1 {
            format!("{} ({})", p.members.len(), p.members.join(" "))
        } else {
            String::new()
        };
        let note = if p.split_layout {
            " MERGE the .rs into mod.rs first"
        } else {
            ""
        };
        println!(
            "  {:<14} {:<10} {:>4}  {:>4}  {members}{note}",
            p.name,
            p.evidence.label(),
            rows,
            tokens,
        );
        if !p.shared.is_empty() {
            println!("  {:<14} shares: {}", "", p.shared.join(", "));
        }
    }
    let named: usize = proposed.iter().map(|p| p.members.len()).sum();
    let tally =
        |e: plan::Evidence| proposed.iter().filter(|p| p.evidence == e).count();
    println!(
        "\n  {} node(s) over {named} module(s): {} directory · {} pub mod · \
         {} family · {} declared.",
        proposed.len(),
        tally(plan::Evidence::Drawn),
        tally(plan::Evidence::Published),
        tally(plan::Evidence::Cohesion),
        tally(plan::Evidence::Declared),
    );
    println!(
        "  Evidence is how explicitly the author drew the boundary. A \
         `declared` node is a module and nothing more -- real, and the \
         weakest reason to promote one. Grouping those is a judgement this \
         does not make."
    );
    // A grade every module shares ranks nothing. Say so, or a reader takes
    // the order for a verdict (`plan:B16`).
    if plan::uniform_evidence(proposed) {
        println!(
            "  Every module carries the SAME grade, so it ranks nothing here \
             -- the order above is by spec rows, which is the only signal \
             this tree offers."
        );
    }
}

/// A node's chain cost and the ceiling it inherits, or zeroes when either
/// cannot be read -- `budget` is the verb that reports why.
pub(super) fn split_budget(root: &Path, dir: &Path) -> (u64, u64) {
    let cost = lens::pack(root, dir, lens::Depth::Rule)
        .map(|p| p.cost.tokens)
        .unwrap_or_default();
    let ceiling = lens::ceiling_for(root, dir).unwrap_or_default();
    (cost, ceiling)
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::*;

    /// `split` PROPOSES and never writes, which is the property worth
    /// pinning: `--apply` refuses, and a federated node has nothing left to
    /// promote so the table is empty rather than noise.
    #[test]
    fn split_proposes_and_refuses_to_apply() {
        let repo = routing_fixture("cli-split");
        let root = repo.path();
        // `alpha` and `beta` are nodes already; a flat module is not.
        let Ok(()) =
            std::fs::write(root.join("gamma.rs"), "pub fn gamma() {}\n")
        else {
            unreachable!("a module file is writable")
        };
        write_spec(root, "alpha", "widgets and sprockets");
        let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
            unreachable!("the fixture spec is readable")
        };
        let Ok(()) = std::fs::write(
            root.join("SPEC.md"),
            format!("{spec}\nV1: gamma holds the gamma rule\n"),
        ) else {
            unreachable!("the fixture spec is writable")
        };

        assert_eq!(split_cmd(root, root, true), ExitCode::from(2), "--apply");
        assert_eq!(split_cmd(root, root, false), ExitCode::SUCCESS);
        // Proposing must not have written anything.
        assert!(!root.join("gamma").exists(), "split created a directory");
    }

    /// The structure-first proposal on a fixture whose modules the spec
    /// never names: `.:src/plan:B12` is that a row ranking sees nothing here,
    /// while the code plainly declares two nodes.
    /// Every grade, including the bottom rung that always fires: a plain
    /// `mod` and a `pub(crate) mod` are both DECLARED, which is what
    /// `microlith` is made of (`.:src/plan:B13`).
    #[test]
    fn a_private_or_crate_visible_module_is_still_a_node() {
        let repo = routing_fixture("cli-split-grades");
        let root = repo.path();
        let Ok(()) = std::fs::create_dir_all(root.join("src")) else {
            unreachable!("a src dir is creatable")
        };
        let Ok(()) = std::fs::write(
            root.join("src").join("lib.rs"),
            "pub mod api;\npub(crate) mod inner;\nmod hidden;\n",
        ) else {
            unreachable!("a lib.rs is writable")
        };
        let found = plan::structure(root);
        let grade =
            |n: &str| found.iter().find(|p| p.name == n).map(|p| p.evidence);
        assert_eq!(grade("api"), Some(plan::Evidence::Published));
        assert_eq!(
            grade("inner"),
            Some(plan::Evidence::Declared),
            "pub(crate) is not published"
        );
        assert_eq!(grade("hidden"), Some(plan::Evidence::Declared));
    }

    #[test]
    fn split_proposes_nodes_the_spec_never_mentions() {
        let repo = routing_fixture("cli-split-structure");
        let root = repo.path();
        let Ok(()) = std::fs::create_dir_all(root.join("src")) else {
            unreachable!("a src dir is creatable")
        };
        let Ok(()) = std::fs::write(
            root.join("src").join("lib.rs"),
            "pub mod widget;\nmod helper;\n#[cfg(test)]\nmod testonly;\n",
        ) else {
            unreachable!("a lib.rs is writable")
        };
        let found = plan::structure(root);
        let names: Vec<&str> = found.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["widget", "helper"],
            "published first, then declared: {found:?}"
        );
        assert_eq!(split_cmd(root, root, false), ExitCode::SUCCESS);
    }

    /// A node with no `SPEC.md` is a usage error, not an empty proposal.
    #[test]
    fn split_on_a_directory_with_no_spec_is_usage() {
        let repo = routing_fixture("cli-split-nospec");
        let bare = repo.path().join("bare");
        let Ok(()) = std::fs::create_dir_all(&bare) else {
            unreachable!("a dir is creatable")
        };
        assert_eq!(split_cmd(repo.path(), &bare, false), ExitCode::from(2));
    }

    /// A node declaring one public type and one private one, beside a
    /// sibling node that declares none.
    fn seam_fixture() -> crate::testrepo::TestRepo {
        let repo = routing_fixture("cli-seam");
        let Ok(()) = std::fs::write(
            repo.path().join("alpha").join("lib.rs"),
            "pub struct Shared {}\nstruct Hidden;\n",
        ) else {
            unreachable!("a module file is writable")
        };
        repo
    }

    /// The vocabulary a sibling can NAME. A private type is not one: a
    /// parallel worker cannot write against a name it cannot spell.
    #[test]
    fn seam_names_a_public_type_and_not_a_private_one() {
        let repo = seam_fixture();
        let nodes = fed::discover(repo.path());
        let t = node_types(&repo.path().join("alpha"), &nodes);
        let named: Vec<&str> = t.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(named, vec!["Shared"], "a private type is not vocabulary");
    }

    /// Absence is an answer, not a failure (`V12`): a node with no types
    /// adds no name a sibling has to know, and the verb still exits 0.
    #[test]
    fn a_node_declaring_no_types_reports_rather_than_erroring() {
        let repo = seam_fixture();
        let root = repo.path();
        let nodes = fed::discover(root);
        assert!(node_types(&root.join("beta"), &nodes).is_empty());
        assert_eq!(seam_cmd(root, root), ExitCode::SUCCESS, "absence is legal");
    }

    /// A file belongs to the NEAREST node. `fed::rust_files` recurses and
    /// the nodes nest, so without that the root row would be the whole
    /// crate and every type would be counted once per ancestor.
    #[test]
    fn a_type_belongs_to_the_nearest_node_not_to_every_ancestor() {
        let repo = seam_fixture();
        let nodes = fed::discover(repo.path());
        let at_root = node_types(repo.path(), &nodes);
        assert!(at_root.is_empty(), "alpha owns Shared: {at_root:?}");
    }

    /// A dir naming no node is a usage error rather than an empty report,
    /// the shape `B5` records one verb over.
    #[test]
    fn seam_on_a_dir_matching_no_node_is_usage() {
        assert_eq!(run_args(argv(&["seam", "no-such-dir"])), ExitCode::from(2));
    }
}
