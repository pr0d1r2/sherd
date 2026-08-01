//! `plan` -- what to attempt next, and why it might not survive contact.
//!
//! Terraform plans the whole graph because providers are deterministic and
//! state does not rewrite itself. Ours does: applying a task edits the source
//! AND the spec that plans the next task. Measured -- one new §B row moved an
//! authoring prompt 1,210 -> 1,520 tokens and flipped a passing run to
//! rejected (`.:tdd` B9).
//!
//! So the horizon is short and honest: three steps, each carrying its
//! confidence and what would invalidate it. Beyond that is fiction.

use crate::fed;
use std::path::{Path, PathBuf};

/// How far ahead a plan is worth stating.
pub const HORIZON: usize = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub node: PathBuf,
    pub id: String,
    /// `.` todo, `~` in progress. `x` rows are not open.
    pub status: char,
    pub text: String,
    pub cites: String,
}

/// Why a task is or is not something the loop can attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A function added to one node's `mod.rs` -- the only shape `tdd` handles.
    NodeFn,
    /// Needs `main.rs`, which `tdd` does not edit.
    Cli,
    /// Writes or reads files beyond the node's own module.
    MultiFile,
    /// Research, reporting, CI -- not code at all.
    NotCode,
    /// A root-level row: no `mod.rs` to add anything to.
    NoModule,
}

impl Kind {
    #[must_use]
    pub fn actionable(self) -> bool {
        self == Kind::NodeFn
    }
    #[must_use]
    pub fn why(self) -> &'static str {
        match self {
            Kind::NodeFn => "single-node function",
            Kind::Cli => "needs main.rs -- tdd edits one node's mod.rs only",
            Kind::MultiFile => "writes files beyond its own module",
            Kind::NotCode => "research or reporting, not code",
            Kind::NoModule => "root row -- no mod.rs to add to",
        }
    }
}

/// Classify by what the loop would have to touch. Deliberately conservative:
/// an unactionable row wrongly attempted burns a whole run and edits source.
#[must_use]
pub fn classify(node: &Path, text: &str) -> Kind {
    if !node.join("mod.rs").is_file() {
        return Kind::NoModule;
    }
    let t = text.to_lowercase();
    let has = |ks: &[&str]| ks.iter().any(|k| t.contains(k));
    if has(&["cmd", "cli", "verb", "`bbx ", "flag", "--"]) {
        Kind::Cli
    } else if has(&["ci", "gate:", "upstream", "audit", "corpus", "fixture", "record "]) {
        Kind::NotCode
    } else if has(&["\u{a7}n", "sync", "baseline", ".md`", "write own", "promotion"]) {
        // Word-order independent, because the first version looked for
        // "derive `§n`" and the row said "`§N` derive from parent `§F`" (B1).
        // Widening a substring list is a patch, not a fix -- T4's declared
        // marker is the fix, and this heuristic stays conservative until then.
        Kind::MultiFile
    } else {
        Kind::NodeFn
    }
}

/// Every open §T row across the federation, in DAG order (shallow first).
#[must_use]
pub fn open_tasks(root: &Path) -> Vec<Task> {
    let mut out = Vec::new();
    for node in fed::discover(root) {
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else { continue };
        let mut in_t = false;
        for line in text.lines() {
            if line.starts_with("## \u{a7}") {
                in_t = line.starts_with("## \u{a7}T");
                continue;
            }
            if !in_t {
                continue;
            }
            let cells: Vec<&str> = line.split('|').collect();
            if cells.len() < 3 || !cells[0].starts_with('T') {
                continue;
            }
            let status = cells[1].chars().next().unwrap_or(' ');
            if status != '.' && status != '~' {
                continue;
            }
            out.push(Task {
                node: node.strip_prefix(root).unwrap_or(&node).to_path_buf(),
                id: cells[0].to_string(),
                status,
                text: cells[2].to_string(),
                cites: cells.get(3).unwrap_or(&"-").to_string(),
            });
        }
    }
    // Shallow before deep: a node's dependencies sit above it in the chain.
    // This is the ONLY ordering signal available -- §T's `cites` points at §V,
    // never at another §T -- so it is stated as weak rather than dressed up.
    out.sort_by_key(|t| (t.node.components().count(), t.node.clone(), t.id.clone()));
    out
}

/// How much of a plan step is actually known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Row and ordering known; contract not yet extracted.
    Next,
    /// Likely, but the step before it will move the specs it reads.
    Likely,
    /// Order may change once anything above it lands.
    Tentative,
}

impl Confidence {
    #[must_use]
    pub fn of(step: usize) -> Self {
        match step {
            0 => Confidence::Next,
            1 => Confidence::Likely,
            _ => Confidence::Tentative,
        }
    }
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Confidence::Next => "NEXT      ",
            Confidence::Likely => "LIKELY    ",
            Confidence::Tentative => "TENTATIVE ",
        }
    }
    /// What would make this step wrong. Stated per step, because a plan that
    /// does not say how it fails is a promise rather than a plan.
    #[must_use]
    pub fn invalidated_by(self) -> &'static str {
        match self {
            Confidence::Next =>
                "judge rejects the test 3x · gate still red after 3 repairs",
            Confidence::Likely =>
                "step 1 adds a §B row to this node, changing its authoring prompt (tdd B9)",
            Confidence::Tentative =>
                "any §T row added by steps above it; ordering has no declared `needs`",
        }
    }
}

#[derive(Debug)]
pub struct Plan {
    pub steps: Vec<Task>,
    pub unmanaged: Vec<(Task, Kind)>,
    pub total_open: usize,
}

/// Take the next [`HORIZON`] actionable rows and everything it cannot manage.
#[must_use]
pub fn plan(root: &Path) -> Plan {
    let all = open_tasks(root);
    let total_open = all.len();
    let mut steps = Vec::new();
    let mut unmanaged = Vec::new();
    for t in all {
        let k = classify(&root.join(&t.node), &t.text);
        if k.actionable() && steps.len() < HORIZON {
            steps.push(t);
        } else if !k.actionable() {
            unmanaged.push((t, k));
        }
    }
    Plan { steps, unmanaged, total_open }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizon_is_short_because_the_state_self_modifies() {
        assert_eq!(HORIZON, 3, "a longer horizon would be fiction (tdd B9)");
    }

    #[test]
    fn every_confidence_states_how_it_fails() {
        for c in [Confidence::Next, Confidence::Likely, Confidence::Tentative] {
            assert!(!c.invalidated_by().is_empty(),
                    "a plan that cannot say how it fails is a promise");
        }
    }

    #[test]
    fn confidence_degrades_with_distance() {
        assert_eq!(Confidence::of(0), Confidence::Next);
        assert_eq!(Confidence::of(1), Confidence::Likely);
        assert_eq!(Confidence::of(2), Confidence::Tentative);
        assert_eq!(Confidence::of(99), Confidence::Tentative);
    }

    #[test]
    fn classify_is_word_order_independent() {
        // Both phrasings describe writing §N into other nodes' specs.
        let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
        assert_eq!(classify(&n, "derive `§N` from parent `§F`"), Kind::MultiFile);
        assert_eq!(classify(&n, "`§N` derive from parent `§F`"), Kind::MultiFile);
    }

    #[test]
    fn a_root_row_is_never_actionable() {
        assert_eq!(classify(Path::new("."), "anything at all"), Kind::NoModule);
        assert!(!Kind::NoModule.actionable());
    }

    #[test]
    fn plan_never_exceeds_the_horizon() {
        let p = plan(Path::new(env!("CARGO_MANIFEST_DIR")));
        assert!(p.steps.len() <= HORIZON, "{} steps", p.steps.len());
        assert!(p.total_open >= p.steps.len());
    }
}
