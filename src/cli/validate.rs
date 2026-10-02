//! `validate` -- one verdict over the whole federation, COMPOSED from the
//! families other verbs own: structure, edges, ceilings, slice drift.

use super::*;

/// One verdict over the whole federation, for CI and for a stranger who
/// wants to know whether a repository is coherent before reading it.
///
/// COMPOSES what already has owners rather than re-deciding anything: the
/// structural check (`spec`), the DAG's shape (`fed`), every chain against
/// its ceiling (`lens`), and slice drift (`slice`). §C forbids a second
/// reading of a rule that has an owner, and a validator that re-implemented
/// any of these would be exactly that.
///
/// It REPORTS WHAT IT EXAMINED, not only what failed. "0 violations" and "I
/// checked nothing" are the same output otherwise, which is `.:V48` and the
/// vacuous pass every gate here is written against.
pub(super) fn validate(root: &Path) -> ExitCode {
    let nodes = fed::discover(root);
    let structural = validate_specs(&nodes);
    let edges = validate_edges(root);
    let over = validate_ceilings(root, &nodes);
    let drift = validate_drift(root);
    println!(
        "\n  {} nodes · {structural} structural · {edges} edge · \
         {over} over ceiling · {drift} drifted",
        nodes.len()
    );
    verdict(structural + edges + over + drift)
}

/// `validate --format json`: the same four families, collected rather
/// than printed.
pub(super) fn validate_json_cmd(root: &Path) -> ExitCode {
    let nodes = fed::discover(root);
    let (over, cold) = ceiling_findings(root, &nodes);
    let v = ValidateReport {
        nodes: nodes.len(),
        structural: spec_findings(&nodes),
        edges: edge_findings(root),
        over,
        cold,
        slices: slices(root),
    };
    println!("{}", validate_json(root, &v));
    verdict(v.failures())
}

/// What `validate` collected, for the json form.
pub(super) struct ValidateReport {
    pub nodes: usize,
    pub structural: Vec<Finding>,
    pub edges: Vec<Finding>,
    pub over: Vec<Finding>,
    pub cold: bool,
    pub slices: Slices,
}

impl ValidateReport {
    pub(super) fn failures(&self) -> usize {
        let drift = match &self.slices {
            Slices::Absent => 0,
            Slices::Read(d) => d.len(),
            Slices::Unreadable(_) => 1,
        };
        fatal(&self.structural) + fatal(&self.edges) + fatal(&self.over) + drift
    }
}

/// The slice registry, three ways. ABSENT is legal (`src/cli:V12`);
/// UNREADABLE is one failure, never zero (`.:V48`).
pub(super) enum Slices {
    Absent,
    Read(Vec<PathBuf>),
    Unreadable(String),
}

pub(super) fn slices(root: &Path) -> Slices {
    // A repository with NO slice registry has nothing to drift from, and
    // absence is legal: `sherd init` then `sherd validate` has to be able to
    // pass, or the two verbs contradict each other. Distinguished from a
    // registry that cannot be READ, which stays a failure.
    if !root.join(".sherd-slices").exists() {
        return Slices::Absent;
    }
    match slice::drifted(root) {
        Ok(d) => Slices::Read(d),
        Err(e) => Slices::Unreadable(e.to_string()),
    }
}

/// The structural check, on every node's spec.
pub(super) fn validate_specs(nodes: &[PathBuf]) -> usize {
    print_findings(&spec_findings(nodes))
}

pub(super) fn spec_findings(nodes: &[PathBuf]) -> Vec<Finding> {
    let mut out = Vec::new();
    for node in nodes {
        let path = node.join("SPEC.md");
        match std::fs::read_to_string(&path) {
            Ok(text) => out.extend(microlith_findings(&path, &text)),
            Err(e) => out.push(unread(&path, &e)),
        }
    }
    out
}

/// Distilled slices against their sources. An unreadable tree counts as one
/// failure rather than zero: "could not look" and "nothing wrong" are the
/// same output otherwise, which is the vacuous pass `.:V48` forbids.
pub(super) fn validate_drift(root: &Path) -> usize {
    match slices(root) {
        Slices::Absent => {
            println!("slice: no registry (none required)");
            0
        }
        Slices::Read(drifted) => report_drift(&drifted),
        Slices::Unreadable(e) => {
            println!("slice: {e}");
            1
        }
    }
}

pub(super) fn report_drift(drifted: &[PathBuf]) -> usize {
    for p in drifted {
        println!("{}: slice drifted from its source", p.display());
    }
    drifted.len()
}

/// Depth and ownership rules over the `§F` edges of every node.
pub(super) fn validate_edges(root: &Path) -> usize {
    print_findings(&edge_findings(root))
}

pub(super) fn edge_findings(root: &Path) -> Vec<Finding> {
    let mut out = Vec::new();
    for node in fed::discover(root) {
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
            continue;
        };
        let edges = fed::edges(&text);
        for e in fed::depth_violations(&edges) {
            out.push(Finding::bare(
                &node,
                format!("edge to `{}` skips a level", e.dir),
            ));
        }
    }
    out
}

/// Every chain against the ceiling it inherits.
pub(super) fn validate_ceilings(root: &Path, nodes: &[PathBuf]) -> usize {
    let (over, cold) = ceiling_findings(root, nodes);
    print_findings(&over);
    // Still REPORTED when cold -- `.:V48` -- just not counted.
    if cold && !over.is_empty() {
        println!(
            "  ({} over the {} tok default; set .context-limits to gate it)",
            over.len(),
            tokens::DEFAULT_NODE
        );
    }
    fatal(&over)
}

/// The chains over their ceilings, and whether the ceilings are COLD.
///
/// A COLD START is not a breach: with no `.context-limits` the default is a
/// suggestion nobody wrote, and one verdict that fails on it is a claim
/// about a rule that does not exist (`B8`). Cold findings are advisory.
pub(super) fn ceiling_findings(
    root: &Path,
    nodes: &[PathBuf],
) -> (Vec<Finding>, bool) {
    let cold = tokens::Ceilings::load(root).is_ok_and(|c| c.is_cold());
    let over = nodes
        .iter()
        .filter_map(|node| over_ceiling(root, node))
        .map(|f| if cold { f.advisory() } else { f })
        .collect();
    (over, cold)
}

/// One chain against the ceiling it inherits. A node whose pack or ceiling
/// cannot be read is not over -- it is unmeasured, and `budget` is the verb
/// that reports that.
pub(super) fn over_ceiling(root: &Path, node: &Path) -> Option<Finding> {
    let (Ok(pack), Ok(ceiling)) = (
        lens::pack(root, node, lens::Depth::Rule),
        lens::ceiling_for(root, node),
    ) else {
        return None;
    };
    (pack.cost.tokens > ceiling).then(|| {
        Finding::bare(
            node,
            format!(
                "chain {} tok over its ceiling of {ceiling}",
                pack.cost.tokens
            ),
        )
    })
}
