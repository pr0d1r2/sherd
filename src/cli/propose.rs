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

    // STRUCTURE FIRST (`.:src/split:V2`): what the code already separated, then
    // the prose weight of each. A module the spec never mentions is still a
    // node; a ranking by rows cannot see it (`.:src/split:B1`).
    let proposed = split::structure(dir);
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
pub(super) fn print_structure(proposed: &[split::Proposed], spec: &str) {
    // Heaviest FIRST. The evidence grade discriminates in ONE of six
    // repositories measured -- `itok` -- and in the other five every module
    // carries the same grade, which leaves the row weight as the only signal
    // present. Alphabetical order threw it away: `metope` spans 0 to 58 rows
    // and put its 58-row node first by luck of the letter b (`plan:B16`).
    let mut ranked: Vec<(&split::Proposed, (usize, u64))> = proposed
        .iter()
        .map(|p| (p, split::row_weight(spec, &p.name)))
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
    let tally = |e: split::Evidence| {
        proposed.iter().filter(|p| p.evidence == e).count()
    };
    println!(
        "\n  {} node(s) over {named} module(s): {} directory · {} pub mod · \
         {} family · {} declared.",
        proposed.len(),
        tally(split::Evidence::Drawn),
        tally(split::Evidence::Published),
        tally(split::Evidence::Cohesion),
        tally(split::Evidence::Declared),
    );
    println!(
        "  Evidence is how explicitly the author drew the boundary. A \
         `declared` node is a module and nothing more -- real, and the \
         weakest reason to promote one. Grouping those is a judgement this \
         does not make."
    );
    // A grade every module shares ranks nothing. Say so, or a reader takes
    // the order for a verdict (`plan:B16`).
    if split::uniform_evidence(proposed) {
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
#[path = "tests/propose.rs"]
mod tests;
