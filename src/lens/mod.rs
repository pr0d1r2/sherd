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
/// `NotFound` when `dir` carries no `SPEC.md` -- it is not a node, and its
/// ancestors' pack is not its pack (V5, B2). Otherwise propagates read
/// failures -- a node that cannot be read is a failure, not a skipped zero
/// (V48).
pub fn pack(root: &Path, dir: &Path, depth: Depth) -> std::io::Result<Pack> {
    // `V5`: a node that is not there is an error. `fed::chain` walks the
    // ANCESTORS that exist, so without this a missing node packed as its
    // parent under its own name (`B2`).
    if !fed::is_node(dir) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("{}: no SPEC.md -- not a node", dir.display()),
        ));
    }
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
        text.push_str(&render(raw, depth));
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

/// One spec as a pack at `depth` carries it.
fn render(raw: String, depth: Depth) -> String {
    match depth {
        Depth::All => raw,
        Depth::Rule | Depth::Why => crate::spec::rule_depth(&raw),
    }
}

/// What the node's OWN `SPEC.md` costs at `depth` -- its share of [`pack`]'s
/// `cost`, the chain minus everything it inherits. A split moves rows
/// between nodes; `cost` says what a worker loads, this says which spec in
/// the chain carries it.
///
/// # Errors
/// A read failure is propagated, never a zero (V5) -- `NotFound` included,
/// when `dir` is no node.
pub fn own_cost(dir: &Path, depth: Depth) -> std::io::Result<tokens::Count> {
    let raw = std::fs::read_to_string(dir.join("SPEC.md"))?;
    Ok(tokens::count(&render(raw, depth)))
}

/// The chain ceiling for a node, from `.context-limits`.
///
/// A ceiling has to come from somewhere: asked for a split hint "when the
/// pack exceeds the ceiling", the model invented a `budget.node` FILE and
/// read it (B1). Now there is a real source, in itok's format.
///
/// # Errors
/// Propagates a malformed `.context-limits` -- an unparsable ceiling is an
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
#[path = "tests/lens.rs"]
mod tests;
