//! Which generated blocks a change can possibly have invalidated.
//!
//! The gate hands a step the files that changed. Re-rendering every block for
//! every change is correct and wasteful; re-rendering none is fast and wrong.
//! The map below is the middle: each block declares the inputs it is rendered
//! FROM, and a change selects the blocks whose inputs it touches.
//!
//! It is a HEURISTIC in one direction only, and the direction matters. A
//! pattern that is too broad costs a re-render nobody needed. A pattern that
//! is too narrow lets a stale block through the commit hook -- so the wide
//! run still happens on push, where nothing is scoped and every block is
//! compared. Cheap and approximate at the near end, complete at the far one.

/// One generated block and the files it is rendered from.
pub struct Generated {
    pub name: &'static str,
    pub inputs: &'static [&'static str],
}

/// Every block `sherd-dev readme` maintains.
///
/// `**/SPEC.md` appears in all four because both halves read it: the graph
/// renderings come from the `§F` tables, and the badge block counts nodes by
/// walking for `SPEC.md` files. Adding a node moves the diagram AND the
/// `federated_nodes` badge, which is exactly the change that went unnoticed
/// in `.:B16`.
pub const BLOCKS: &[Generated] = &[
    Generated {
        name: "badges",
        inputs: &[
            "Cargo.toml",
            "hk.pkl",
            ".coverage",
            ".lint-debt",
            "flake.lock",
            ".github/workflows/ci.yml",
            "**/SPEC.md",
            "dev/src/**",
        ],
    },
    Generated {
        name: "graph-tree",
        inputs: &["**/SPEC.md", "dev/src/**"],
    },
    Generated {
        name: "graph-mermaid",
        inputs: &["**/SPEC.md", "dev/src/**"],
    },
    Generated {
        name: "graph-table",
        inputs: &["**/SPEC.md", "dev/src/**"],
    },
    // The Commands table renders from `sherd::cli::USAGE`, and the same file
    // carries the dispatch arms `undocumented` compares it against, so one
    // input covers both halves of `src/cli:V7`.
    Generated {
        name: "commands",
        inputs: &["src/cli/mod.rs", "SPEC.md", "dev/src/**"],
    },
];

/// Does one changed path match one declared input?
///
/// Three shapes, and no more, because a glob engine here would be a second
/// reading of `itok::glob` for a pattern set this file owns entirely:
/// a literal path, a `dir/**` prefix, and a `**/name` suffix.
#[must_use]
pub fn matches(pattern: &str, path: &str) -> bool {
    if let Some(prefix) = pattern.strip_suffix("/**") {
        return path == prefix || path.starts_with(&format!("{prefix}/"));
    }
    if let Some(name) = pattern.strip_prefix("**/") {
        return path == name || path.ends_with(&format!("/{name}"));
    }
    pattern == path
}

/// The blocks worth comparing, given what changed.
///
/// An EMPTY change set means "no scope given", which selects everything --
/// the shape `hk check --all` and CI use. A non-empty set that touches no
/// input selects nothing, and a checker with nothing to check is clean
/// rather than silent: the caller reports zero blocks examined, because a
/// vacuous pass and a real one must not read the same (`.:V16`).
#[must_use]
pub fn selected(changed: &[String]) -> Vec<&'static str> {
    if changed.is_empty() {
        return BLOCKS.iter().map(|b| b.name).collect();
    }
    BLOCKS
        .iter()
        .filter(|b| {
            b.inputs
                .iter()
                .any(|p| changed.iter().any(|c| matches(p, c)))
        })
        .map(|b| b.name)
        .collect()
}

#[cfg(test)]
#[path = "tests/select.rs"]
mod tests;
