//! The lens pack: what one node costs to work at.
//!
//! V15: self-contained at its altitude. V45: `rule` depth by default --
//! rationale is pulled on demand, never resident, because entry cost is
//! re-billed every turn.

use crate::{fed, tokens};
use std::path::{Path, PathBuf};

/// How much detail to render (V42's vertical axis).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Depth {
    /// Rules only. The default -- 80% of a mature §V is rationale.
    Rule,
    /// Rules plus rationale from `SPEC.why.md`.
    Why,
    /// Everything, archive included. For reading a node's history, never for
    /// handing to a worker.
    All,
}

#[derive(Debug)]
pub struct Pack {
    pub chain: Vec<PathBuf>,
    pub text: String,
    pub children: Vec<fed::Edge>,
    pub cost: tokens::Count,
}

/// Assemble the pack for `dir`: every ancestor spec, this node's spec, and
/// the one-line lens of each child.
///
/// # Errors
/// Propagates read failures -- a node that cannot be read is a failure, not
/// a skipped zero (V48).
pub fn pack(root: &Path, dir: &Path, depth: Depth) -> std::io::Result<Pack> {
    let chain = fed::chain(root, dir);
    let mut text = String::new();
    for spec in &chain {
        let raw = std::fs::read_to_string(spec)?;
        // `Depth` used to be consulted ONLY for `Why`, so `Rule` -- the
        // default §I documents -- selected nothing and every pack, every
        // budget and every `ask` carried §R and §B (`.:B8`, `.:V105`). The
        // archive is what was tried and measured; a worker acts on rules.
        //
        // §F leaves the TEXT here and stays reachable: `Pack::children` is
        // parsed from the node's own spec below and rendered separately, so
        // navigation survives without the table riding in every prompt.
        text.push_str(&match depth {
            Depth::All => raw,
            Depth::Rule | Depth::Why => crate::spec::rule_depth(&raw),
        });
        text.push('\n');
    }
    if depth == Depth::Why {
        let why = dir.join("SPEC.why.md");
        if why.is_file() {
            text.push_str(&std::fs::read_to_string(why)?);
        }
    }
    let own = chain.last().map_or(String::new(), |p| {
        std::fs::read_to_string(p).unwrap_or_default()
    });
    Ok(Pack {
        chain,
        children: fed::edges(&own),
        cost: tokens::count(&text),
        text,
    })
}

/// The chain ceiling for a node, from `.context-limits`.
///
/// A ceiling has to come from somewhere: asked for a split hint "when the
/// pack exceeds the ceiling", the model invented a `budget.node` FILE and
/// read it (B1). Now there is a real source, in itok's format.
///
/// # Errors
/// Propagates a malformed `.context-limits` -- an unparseable ceiling is an
/// error, not a silent default.
pub fn ceiling_for(root: &Path, node: &Path) -> Result<u64, String> {
    let rel = node.strip_prefix(root).unwrap_or(node);
    let key = if rel.as_os_str().is_empty() {
        "SPEC.md".into()
    } else {
        rel.to_string_lossy().to_string()
    };
    Ok(tokens::Ceilings::load(root)?.for_path(&key))
}

/// Budget verdict for a pack against a working-token allowance.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    Fits { slack: u64 },
    Over { by: u64 },
}

#[must_use]
pub fn verdict(cost: u64, budget: u64) -> Verdict {
    if cost <= budget {
        Verdict::Fits {
            slack: budget - cost,
        }
    } else {
        Verdict::Over { by: cost - budget }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_node_ceiling_comes_from_the_file_not_a_constant() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // .context-limits names src/tdd; the value is read, not assumed.
        let tdd = ceiling_for(root, &root.join("src/tdd")).unwrap();
        assert!(
            tdd > tokens::DEFAULT_NODE,
            "src/tdd is listed and should not fall back to the default: {tdd}"
        );
        // A new node under src inherits src's ceiling -- prefix matching, so
        // adding a node does not silently drop it to the global default.
        assert_eq!(
            ceiling_for(root, &root.join("src/nope")).unwrap(),
            ceiling_for(root, &root.join("src")).unwrap()
        );
        // A path sharing no listed prefix falls back, which is NOT "no limit".
        assert_eq!(
            ceiling_for(root, &root.join("docs")).unwrap(),
            tokens::DEFAULT_NODE
        );
    }

    #[test]
    fn verdict_reports_direction_and_distance() {
        assert_eq!(verdict(100, 500), Verdict::Fits { slack: 400 });
        assert_eq!(verdict(900, 500), Verdict::Over { by: 400 });
    }

    #[test]
    fn the_root_ceiling_comes_from_its_spec_row_not_the_default() {
        // `.context-limits` names the root `SPEC.md`, while a node is
        // addressed as `.` -- so the lookup has to bridge those two spellings
        // or the root silently falls to DEFAULT_NODE and reads as 5x over
        // (`.:V104`: absence must never read as a verdict).
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c = ceiling_for(root, root).unwrap();
        assert!(
            c > tokens::DEFAULT_NODE,
            "root must resolve to its SPEC.md row, got the default: {c}"
        );
    }

    #[test]
    fn depth_selects_something_or_it_is_a_flag_that_lies() {
        // V105, and the test that would have caught B8 the day `Depth` was
        // introduced: same input, two settings, DIFFERENT output. `Rule` was
        // consulted nowhere, so the two branches agreed for the project's
        // whole life while §I advertised a choice.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let rule = pack(root, root, Depth::Rule).unwrap();
        let all = pack(root, root, Depth::All).unwrap();
        assert!(
            rule.cost.tokens < all.cost.tokens,
            "rule {} must be cheaper than all {}",
            rule.cost.tokens,
            all.cost.tokens
        );
        assert!(all.text.contains("## \u{a7}B"), "all keeps the archive");
        assert!(
            !rule.text.contains("## \u{a7}B"),
            "rule drops §B -- history is not a rule"
        );
        assert!(
            !rule.text.contains("## \u{a7}R"),
            "rule drops §R -- a measurement is not a rule"
        );
        assert!(
            rule.text.contains("## \u{a7}V"),
            "rule keeps §V -- that is the point of it"
        );
    }

    #[test]
    fn every_node_resolves_to_a_real_ceiling() {
        // V104's second half, made mechanical: absence must never read as
        // permission. DEFAULT_NODE is 2,000 -- a NODE budget, impossible as a
        // CHAIN ceiling -- so a node landing on it means no row covers it,
        // directly or by prefix, and it would be gated against a number
        // nobody chose.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for node in fed::discover(root) {
            let c = ceiling_for(root, &node).unwrap();
            assert_ne!(
                c,
                tokens::DEFAULT_NODE,
                "{} fell back to the node default -- give it a row in \
                 .context-limits, or a prefix that covers it",
                node.display()
            );
        }
    }

    #[test]
    fn a_ceiling_is_compared_at_the_boundary_not_near_it() {
        // T10 turns this comparison into an exit code, so off-by-one here is
        // a gate that fires on compliant nodes or misses drifting ones.
        assert_eq!(verdict(500, 500), Verdict::Fits { slack: 0 });
        assert_eq!(verdict(501, 500), Verdict::Over { by: 1 });
    }

    #[test]
    fn chain_of_repo_root_is_at_least_the_root_spec() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(
            !fed::chain(root, root).is_empty(),
            "root SPEC.md must exist (V5)"
        );
    }
}
