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

/// Stable identity of a row: node + id + the text itself. Editing the text
/// makes it a NEW task, which is the only signal available that a human
/// considers it unfinished -- `apply` must never decide that for them.
#[must_use]
pub fn row_key(t: &Task) -> String {
    format!("{}:{}", t.node.display(), t.id)
}

#[must_use]
pub fn row_hash(t: &Task) -> String {
    crate::state::content_hash(t.text.as_bytes())
}

/// Has this exact row already been applied? Idempotence lives here rather than
/// in `§T`'s status column, because a row may be partly done ("X landed, Y
/// still open") and only its author can judge that.
#[must_use]
pub fn already_applied(st: &crate::state::State, t: &Task) -> bool {
    st.get("applied", &row_key(t)).is_some_and(|v| v.starts_with(&row_hash(t)))
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
    let st = crate::state::State::load();
    for t in all {
        let k = classify(&root.join(&t.node), &t.text);
        if k.actionable() && already_applied(&st, &t) {
            continue; // idempotent: same row, same text, already done
        }
        if k.actionable() && steps.len() < HORIZON {
            steps.push(t);
        } else if !k.actionable() {
            unmanaged.push((t, k));
        }
    }
    Plan { steps, unmanaged, total_open }
}

/// The first `§V` id a row cites -- the invariant `apply` will drive.
#[must_use]
pub fn cited_invariant(t: &Task) -> Option<String> {
    t.cites.split(',').map(str::trim)
        .find(|c| c.starts_with('V') && c[1..].chars().all(|d| d.is_ascii_digit()))
        .map(ToString::to_string)
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
    fn cited_invariant_takes_the_first_v_id() {
        let mk = |c: &str| Task { node: PathBuf::from("src/fed"), id: "T4".into(),
            status: '.', text: "x".into(), cites: c.into() };
        assert_eq!(cited_invariant(&mk("V2,V4")).as_deref(), Some("V2"));
        assert_eq!(cited_invariant(&mk("I,V7")).as_deref(), Some("V7"));
        assert_eq!(cited_invariant(&mk("-")), None);
        assert_eq!(cited_invariant(&mk("B9")), None, "a §B cite is not an invariant");
    }

    #[test]
    fn row_identity_follows_its_text() {
        let a = Task { node: PathBuf::from("src/fed"), id: "T4".into(), status: '.',
                       text: "do a thing".into(), cites: "V2".into() };
        let mut b = a.clone();
        b.text = "do a different thing".into();
        assert_eq!(row_key(&a), row_key(&b), "key is node+id");
        assert_ne!(row_hash(&a), row_hash(&b), "editing the text makes it new work");
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

// ---- apply: execute exactly one step, then stop ----

/// Refuse to run unless the tree is safe to commit into.
///
/// `apply` writes source AND commits. Both are recoverable only if the tree
/// was clean beforehand and the branch is not the trunk.
fn preflight(root: &Path) -> Result<String, String> {
    let git = |args: &[&str]| {
        std::process::Command::new("git").args(args).current_dir(root).output().ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let branch = git(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    if branch.is_empty() {
        return Err("not a git repo -- apply commits, so it needs one".into());
    }
    if branch == "main" || branch == "master" {
        return Err(format!("on {branch} -- apply commits generated code; branch first"));
    }
    if !git(&["status", "--porcelain"]).unwrap_or_default().is_empty() {
        return Err("working tree dirty -- commit or stash first, so the \
                    generated diff is the only thing in the commit".into());
    }
    Ok(branch)
}

/// Apply the first step of the current plan. ONE step, then stop -- the
/// applied task changes the specs that plan the next one, so continuing would
/// act on a plan that is already stale (V2, `.:tdd` B9).
///
/// # Errors
/// Preflight failure, nothing actionable, a row citing no invariant, or the
/// loop failing to reach green.
#[cfg(feature = "ollama")]
pub fn apply(root: &Path, max_repair: usize) -> Result<String, String> {
    let branch = preflight(root)?;
    let p = plan(root);
    let step = p.steps.first().ok_or("nothing actionable to apply")?;
    let inv = cited_invariant(step).ok_or_else(|| format!(
        "{} {} cites no §V id ({}) -- apply drives an INVARIANT, not prose",
        step.node.display(), step.id, step.cites))?;

    eprintln!("apply: {} {} on {branch}\n  invariant {inv}\n  task {}",
              step.node.display(), step.id, step.text);

    let node = root.join(&step.node);
    let log = crate::tdd::drive(root, &node, &inv, &step.text, max_repair)?;

    let sent: u64 = log.iter().map(|s| s.prompt_tokens).sum();
    let max = log.iter().map(|s| s.prompt_tokens).max().unwrap_or(0);
    let body = format!(
        "feat({}): {} via bbx apply\n\n\
         {inv}: driven from {} {}.\n\n\
         Written by a local model through the red -> judge -> green -> gate\n\
         loop: {} round-trips, {sent} tokens sent, max single call {max}.\n\
         The gate ({}) passed before this commit existed.\n\n\
         Generated code. Read the diff -- a green gate is not correctness\n\
         (.:tdd V11), and this loop has twice produced code that passed both\n\
         gates while testing the wrong thing.\n",
        step.node.file_name().and_then(|s| s.to_str()).unwrap_or("bbx"),
        step.text, step.node.display(), step.id, log.len(),
        "cargo test + bbx check");

    let git = |args: &[&str]| std::process::Command::new("git")
        .args(args).current_dir(root).status().map_err(|e| e.to_string());
    git(&["add", "-A"])?;
    let st = std::process::Command::new("git")
        .args(["commit", "-q", "-m", &body]).current_dir(root)
        .status().map_err(|e| e.to_string())?;
    if !st.success() {
        return Err("commit refused by the gate -- generated code is still in \
                    the tree, uncommitted".into());
    }
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"]).current_dir(root).output()
        .map_err(|e| e.to_string())?;
    let sha = String::from_utf8_lossy(&sha.stdout).trim().to_string();

    // Record it applied, keyed by the row's TEXT -- edit the row and it
    // becomes plannable again.
    let mut state = crate::state::State::load();
    state.set("applied", &row_key(step), format!("{} {sha}", row_hash(step)));
    state.clear_kind("plan"); // the plan that produced this is now stale
    state.save();
    Ok(sha)
}
