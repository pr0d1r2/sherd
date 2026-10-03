//! Shell rows (V26): a home that is a NODE rather than a `mod.rs`, and a
//! footprint read off the scripts a row cites.
//!
//! `classify` asks "could the tdd loop add one function to this `mod.rs`",
//! which is a Rust question. A federation whose code is `*.sh` has no
//! `mod.rs` anywhere, so every row read as a root row and nothing was
//! planned (#105). `split` (#81), `wave` (#82) and `adopt` (V10) had
//! already learned to read scripts; this is `plan` doing the same, through
//! the one citation reading they share (`split::cited_scripts`).

use super::Task;
use crate::fed;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The tree's nodes and scripts, read once per plan rather than per row.
pub(super) struct Scripts {
    /// Every discovered node directory, absolute.
    nodes: Vec<PathBuf>,
    /// Every tracked `*.sh`, absolute.
    all: Vec<PathBuf>,
}

/// Where a row lands, read the shell way.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Home {
    /// Not a shell row: `classify` and V5 decide, exactly as before.
    NotShell,
    /// Planned at this node, relative to the root.
    At(PathBuf),
    /// A root row citing scripts that sit in two or more nodes: no single
    /// home, and the row says so rather than picking one.
    Span,
}

impl Scripts {
    pub(super) fn read(root: &Path) -> Self {
        Self {
            nodes: fed::discover(root),
            all: fed::script_files(root),
        }
    }

    /// The scripts a row cites -- task text and cites cell both.
    fn cited(&self, t: &Task) -> Vec<PathBuf> {
        let text = format!("{} {}", t.text, t.cites);
        crate::split::cited_scripts(&text, &self.all)
    }

    /// The DEEPEST non-root node holding a script (`src/fed:V15`), or none.
    fn owner(&self, root: &Path, script: &Path) -> Option<&PathBuf> {
        self.nodes
            .iter()
            .filter(|n| n.as_path() != root && script.starts_with(n))
            .max_by_key(|n| n.components().count())
    }

    /// Does this node own any script outright?
    fn owns_any(&self, dir: &Path) -> bool {
        !fed::owned_script_files(dir, &self.nodes).is_empty()
    }
}

/// Where `t` is planned (V26).
///
/// A node with a `mod.rs` is Rust and not this function's business. A node
/// row is home when its node owns scripts or the row cites one. A ROOT row
/// moves to the one node that owns every script it cites; scripts in two
/// nodes are [`Home::Span`]; a script no node owns leaves V5 deciding.
pub(super) fn home(root: &Path, t: &Task, s: &Scripts) -> Home {
    let dir = root.join(&t.node);
    if dir.join("mod.rs").is_file() {
        return Home::NotShell;
    }
    let cited = s.cited(t);
    if !t.node.as_os_str().is_empty() {
        return if cited.is_empty() && !s.owns_any(&dir) {
            Home::NotShell
        } else {
            Home::At(t.node.clone())
        };
    }
    let owners: BTreeSet<Option<&PathBuf>> =
        cited.iter().map(|c| s.owner(root, c)).collect();
    let only: Vec<&Option<&PathBuf>> = owners.iter().collect();
    match only.as_slice() {
        [Some(n)] => Home::At(n.strip_prefix(root).unwrap_or(n).to_path_buf()),
        _ if cited.is_empty() || owners.contains(&None) => Home::NotShell,
        _ => Home::Span,
    }
}

/// A shell step's footprint: the scripts it cites, relative to the root.
/// Empty means UNKNOWN -- the row names no script -- never "touches nothing".
pub(super) fn touches(root: &Path, t: &Task, s: &Scripts) -> Vec<String> {
    s.cited(t)
        .iter()
        .map(|p| {
            p.strip_prefix(root)
                .unwrap_or(p)
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

/// The rows a step cites, as its cites cell spells them; `-` is none.
pub(super) fn cited_rows(t: &Task) -> Vec<String> {
    t.cites
        .split(',')
        .map(|c| c.trim().trim_matches('`').to_string())
        .filter(|c| !c.is_empty() && c != "-")
        .collect()
}

#[cfg(test)]
#[path = "tests/shell.rs"]
mod tests;
