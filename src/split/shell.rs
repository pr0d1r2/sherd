//! The shell half of `structure` (`src/split:V7`): the dirs and families a
//! shell codebase drew, and which spec rows cite their scripts.
//!
//! A script is cited by a token ending in `.sh`, matched against the tracked
//! scripts component-wise from the end -- `a/one.sh` and `one.sh` both name
//! `<root>/a/one.sh`. A token naming two or more scripts names neither. Calls
//! between scripts are a dependency, not ownership, and are `src/wave`'s.

use super::{Evidence, Proposed};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Candidates a shell codebase drew under `dir`: each DIRECT child dir
/// holding a script at any depth, then the prefix families among the
/// scripts directly in `dir`.
///
/// Direct children only, because an edge is parent + 1 (`src/fed:V2`). A
/// deep tree splits one level per run; `scripts_of` gives every script
/// beneath a candidate, so a reader sees where the next cut would fall.
pub(super) fn candidates(dir: &Path, scripts: &[PathBuf]) -> Vec<Proposed> {
    let mut children: BTreeSet<String> = BTreeSet::new();
    let mut flat = Vec::new();
    let under = scripts
        .iter()
        .filter_map(|s| s.strip_prefix(dir).ok().map(|rel| (s, rel)));
    for (s, rel) in under {
        let mut parts = rel.components();
        let first = parts.next();
        match (first, parts.next()) {
            (Some(child), Some(_)) => {
                children
                    .insert(child.as_os_str().to_string_lossy().to_string());
            }
            _ => flat.push(s.clone()),
        }
    }
    let mut out: Vec<Proposed> = children
        .into_iter()
        .map(|name| Proposed {
            members: vec![name.clone()],
            name,
            evidence: Evidence::Drawn,
            split_layout: false,
            shared: Vec::new(),
        })
        .collect();
    for f in families(&flat) {
        if !out.iter().any(|p| p.name == f.name) {
            out.push(f);
        }
    }
    out
}

/// Flat scripts sharing the name before their first `-`, three or more of
/// them -- the same line `src/split`'s Rust families draw, for the same
/// reason: two is a pair, three is a concern the author named.
fn families(flat: &[PathBuf]) -> Vec<Proposed> {
    let mut by_prefix: BTreeMap<String, Vec<(String, PathBuf)>> =
        BTreeMap::new();
    for s in flat {
        let stem = s
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if let Some((prefix, _)) = stem.split_once('-') {
            by_prefix
                .entry(prefix.to_string())
                .or_default()
                .push((stem.clone(), s.clone()));
        }
    }
    by_prefix
        .into_iter()
        .filter(|(_, m)| m.len() >= 3)
        .map(|(name, m)| Proposed {
            name,
            evidence: Evidence::Cohesion,
            split_layout: false,
            shared: Vec::new(),
            members: m.into_iter().map(|(stem, _)| stem).collect(),
        })
        .collect()
}

/// The tokens in a line that end in `.sh`: what a row cites a script by.
pub(super) fn script_tokens(line: &str) -> impl Iterator<Item = &str> {
    line.split(|c: char| !(c.is_alphanumeric() || "_./-".contains(c)))
        .map(|t| t.trim_start_matches("./"))
        .filter(|t| t.len() > ".sh".len() && t.ends_with(".sh"))
}

/// The scripts one token names: every tracked script whose path ENDS with
/// it, component by component.
pub(super) fn resolve<'a>(
    token: &str,
    scripts: &'a [PathBuf],
) -> Vec<&'a PathBuf> {
    let want = Path::new(token);
    scripts.iter().filter(|s| s.ends_with(want)).collect()
}
