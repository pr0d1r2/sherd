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
        text.push_str(&std::fs::read_to_string(spec)?);
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
    Ok(Pack { chain, children: fed::edges(&own), cost: tokens::count(&text), text })
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
        Verdict::Fits { slack: budget - cost }
    } else {
        Verdict::Over { by: cost - budget }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_reports_direction_and_distance() {
        assert_eq!(verdict(100, 500), Verdict::Fits { slack: 400 });
        assert_eq!(verdict(900, 500), Verdict::Over { by: 400 });
    }

    #[test]
    fn chain_of_repo_root_is_at_least_the_root_spec() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(!fed::chain(root, root).is_empty(), "root SPEC.md must exist (V5)");
    }
}
