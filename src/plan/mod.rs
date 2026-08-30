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
    /// Replaces or removes existing code. `insert_impl` only APPENDS, so the
    /// loop would write a second copy beside the first -- exactly the
    /// duplication such a row exists to remove.
    Replaces,
    /// Adds nothing callable: edits specs, moves rows, wires things together.
    NotAFunction,
    /// A root-level row: no `mod.rs` to add anything to.
    NoModule,
    /// In a node the root spec declares FROZEN (`.:V117`).
    Frozen,
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
            Kind::Replaces => "replaces existing code -- the loop only appends",
            Kind::NotAFunction => {
                "no new function to add -- edits specs or wiring"
            }
            Kind::NoModule => "root row -- no mod.rs to add to",
            Kind::Frozen => "node FROZEN by the root spec until its rung lands",
        }
    }
}

/// Nodes the root spec declares FROZEN, derived rather than listed.
///
/// `.:V117` freezes the model half until rung 0.7, and `plan` offered
/// `src/ollama` T3 as a step anyway -- work the spec forbids, ranked and
/// recommended (B16). The list is read from the row that declares it and
/// resolved against the tree, so a node added to or removed from the freeze
/// needs no edit here: `V15` records what a hardcoded vocabulary costs.
#[must_use]
pub fn frozen_nodes(root: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string(root.join("SPEC.md")) else {
        return Vec::new();
    };
    text.lines()
        .filter(|l| l.contains("FROZEN"))
        .flat_map(|l| l.split('`').skip(1).step_by(2))
        .map(|p| root.join(p))
        .filter(|p| p.join("SPEC.md").is_file())
        .collect()
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
    // WORD match, because "report" contains "port" and a substring list
    // classified every `report ...` row as a replacement (B5).
    let words: Vec<&str> = t
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    // STEM match: "wiring" did not match "wire" and a row needing wiring
    // counted as actionable (B8).
    //
    // The trailing `e` is TRIMMED, which B8's own fix did not do: `"wiring"
    // .starts_with("wire")` is false, so nine of these eighteen stems --
    // every one ending in `e` -- still missed their `-ing` form, including
    // the exact word B8 names (B11). `V14` is the rule that came out of it.
    let word = |ks: &[&str]| {
        ks.iter().any(|k| {
            let stem = k.trim_end_matches('e');
            words.iter().any(|w| w.starts_with(stem))
        })
    };
    // WHITELIST, not blacklist. A row is actionable when it says "add one
    // function", not merely when it fails to match known-bad shapes. The
    // blacklist marked "replace the hand-rolled walk" actionable, and the loop
    // cannot replace anything (B4).
    // POSITION words name where new code goes RELATIVE to existing code, and
    // every such position requires editing an existing call site. The loop
    // only appends, so these are replacements however the row is phrased --
    // "retry with bounded backoff AROUND `Transport::post`" reads as adding
    // one function and cannot be done by adding one function (B9).
    if word(&[
        "replace",
        "remove",
        "port",
        "migrate",
        "rewrite",
        "delete",
        "supersede",
        "around",
        "wrap",
        "inside",
    ]) {
        Kind::Replaces
    } else if word(&[
        "blocked", "needs", "promote", "wire", "move", "record", "flip",
        "plant",
    ]) {
        Kind::NotAFunction
    } else if has(&["cmd", "cli", "verb", "`sherd ", "flag", "--"]) {
        Kind::Cli
    } else if word(&["upstream", "audit", "corpus", "fixture"])
        || has(&["ci ", "gate:"])
    {
        Kind::NotCode
    } else if has(&["\u{a7}n", "sync", "baseline", ".md`", "write own"]) {
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
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
            continue;
        };
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
            let [id, state, text, ..] = cells.as_slice() else {
                continue;
            };
            let status = state.chars().next().unwrap_or(' ');
            if !id.starts_with('T') || (status != '.' && status != '~') {
                continue;
            }
            out.push(Task {
                node: node.strip_prefix(root).unwrap_or(&node).to_path_buf(),
                id: (*id).to_string(),
                status,
                text: (*text).to_string(),
                cites: cells.get(3).unwrap_or(&"-").to_string(),
            });
        }
    }
    // Shallow before deep: a node's dependencies sit above it in the chain.
    // This is the ONLY ordering signal available -- §T's `cites` points at §V,
    // never at another §T -- so it is stated as weak rather than dressed up.
    out.sort_by_key(|t| {
        (t.node.components().count(), t.node.clone(), t.id.clone())
    });
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
            Confidence::Next => {
                "judge rejects the test 3x · gate still red after 3 repairs"
            }
            Confidence::Likely => {
                "step 1 adds a §B row to this node, changing its authoring prompt (tdd B9)"
            }
            Confidence::Tentative => {
                "any §T row added by steps above it; ordering has no declared `needs`"
            }
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
    st.get("applied", &row_key(t))
        .is_some_and(|v| v.starts_with(&row_hash(t)))
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
    let frozen = frozen_nodes(root);
    let mut candidates = Vec::new();
    for t in all {
        let dir = root.join(&t.node);
        // A frozen node is unmanaged whatever the row says: the freeze is
        // root POLICY, not a property of the text, so no amount of reading
        // the row can reach it (B16).
        let k = if frozen.contains(&dir) {
            Kind::Frozen
        } else {
            classify(&dir, &t.text)
        };
        if k.actionable() && already_applied(&st, &t) {
            continue; // idempotent: same row, same text, already done
        }
        if k.actionable() {
            candidates.push(t);
        } else {
            unmanaged.push((t, k));
        }
    }
    // Most-believable node first. A node that has failed three times running
    // should not keep supplying step 1, which is what depth-ordering did.
    candidates.sort_by(|a, b| {
        believability(&b.node)
            .total_cmp(&believability(&a.node))
            .then_with(|| {
                a.node
                    .components()
                    .count()
                    .cmp(&b.node.components().count())
            })
            .then_with(|| a.id.cmp(&b.id))
    });
    steps.extend(candidates.into_iter().take(HORIZON));
    Plan {
        steps,
        unmanaged,
        total_open,
    }
}

/// The first `§V` a row cites, as `(where it is declared, id)`.
///
/// A cite may be bare (`V3`, this node) or namespaced (`` `.:V73` ``, root;
/// `` `src/fed:V9` ``, that node). Moving rows down rewrote every cite to the
/// namespaced form, which is correct for microlith and was invisible to this
/// parser -- so every moved row became undrivable (B6).
#[must_use]
pub fn cited_invariant(t: &Task) -> Option<(std::path::PathBuf, String)> {
    for raw in t.cites.split(',') {
        let c = raw.trim().trim_matches('`');
        let (owner, id) = match c.rsplit_once(':') {
            Some((path, id)) => (path, id),
            None => ("", c),
        };
        if id.starts_with('V')
            && id.len() > 1
            && id[1..].chars().all(|d| d.is_ascii_digit())
        {
            let node = match owner {
                "" => t.node.clone(),             // bare: this node
                "." => std::path::PathBuf::new(), // root
                p => std::path::PathBuf::from(p),
            };
            return Some((node, id.to_string()));
        }
    }
    None
}

/// One proposal with the spec weight that ranks it.
#[derive(Debug, Clone)]
pub struct Ranked<'a> {
    pub node: &'a Proposed,
    /// Spec rows in the parent naming this module.
    pub rows: usize,
    /// What those rows cost.
    pub tokens: u64,
}

/// Proposals HEAVIEST first.
///
/// The evidence grade discriminates in ONE of six repositories measured --
/// `itok`. In the other five every module carries the same grade, which
/// leaves the spec rows as the only signal present, and alphabetical order
/// threw it away: `metope` spans 0 to 58 rows and led with its 58-row node
/// by luck of the letter (`B17`).
///
/// Grade breaks ties, then name, so the order is stable between runs.
#[must_use]
pub fn rank<'a>(proposed: &'a [Proposed], spec: &str) -> Vec<Ranked<'a>> {
    let mut out: Vec<Ranked<'a>> = proposed
        .iter()
        .map(|node| {
            let (rows, tokens) = row_weight(spec, &node.name);
            Ranked { node, rows, tokens }
        })
        .collect();
    out.sort_by(|a, b| {
        b.rows
            .cmp(&a.rows)
            .then_with(|| a.node.evidence.cmp(&b.node.evidence))
            .then_with(|| a.node.name.cmp(&b.node.name))
    });
    out
}

/// Do ALL proposals carry the same grade? Then it ranks nothing, and a reader
/// who takes the order for a verdict is reading spec rows, not structure.
#[must_use]
pub fn uniform_evidence(proposed: &[Proposed]) -> bool {
    let mut grades = proposed.iter().map(|p| p.evidence);
    let Some(first) = grades.next() else {
        return false;
    };
    proposed.len() > 1 && grades.all(|g| g == first)
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
            assert!(
                !c.invalidated_by().is_empty(),
                "a plan that cannot say how it fails is a promise"
            );
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
    fn report_is_not_a_replacement() {
        // "report" contains "port"; a substring list classified every report
        // row as a replacement (B5).
        let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
        assert_eq!(classify(&n, "report duplicate rows"), Kind::NodeFn);
    }

    #[test]
    fn a_replacement_row_is_not_actionable() {
        // "replace the hand-rolled walk with itok::walk" -- insert_impl only
        // appends, so the loop would add a SECOND walk (B4).
        let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
        assert_eq!(
            classify(&n, "replace the hand-rolled walk with `itok::walk`"),
            Kind::Replaces
        );
        assert!(!Kind::Replaces.actionable());
    }

    #[test]
    fn a_row_that_names_a_position_is_a_replacement() {
        // The row that cost two runs and 17,551 tokens. It reads as "add one
        // function" and every verb-based check passed it, but "AROUND an
        // existing function" means editing that function's call site, which
        // the loop cannot do (B9).
        let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ollama");
        for row in [
            "retry w/ bounded backoff around `Transport::post`",
            "wrap the transport in a retrying decorator",
            "cache lookups inside `generate_via`",
        ] {
            assert_eq!(
                classify(&n, row),
                Kind::Replaces,
                "should not be drivable: {row}"
            );
        }
        // Still whitelist, not blacklist: adding a free function stays actionable.
        assert_eq!(
            classify(
                &n,
                "`backoff_delay(attempt)` returns the delay before one retry"
            ),
            Kind::NodeFn
        );
    }

    #[test]
    fn a_blocked_row_is_never_handed_back() {
        // Marking a row BLOCKED in its text did nothing -- plan handed it
        // straight back as step 1 (plan B7).
        let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
        assert!(
            !classify(&n, "BLOCKED — needs Rust source, ⊥ §F data")
                .actionable()
        );
    }

    #[test]
    fn a_spec_editing_row_is_not_actionable() {
        let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
        assert_eq!(
            classify(
                &n,
                "promote an invariant from a leaf to the common ancestor"
            ),
            Kind::NotAFunction
        );
    }

    #[test]
    fn adding_a_function_stays_actionable() {
        let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
        assert_eq!(
            classify(&n, "report `§F` rows naming a dir twice"),
            Kind::NodeFn
        );
    }

    #[test]
    fn classify_is_word_order_independent() {
        // Both phrasings describe writing §N into other nodes' specs.
        let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
        assert_eq!(
            classify(&n, "derive `§N` from parent `§F`"),
            Kind::MultiFile
        );
        assert_eq!(
            classify(&n, "`§N` derive from parent `§F`"),
            Kind::MultiFile
        );
    }

    /// `B17`: `plan` recommended `src/ollama` T3 as a step while `.:V117`
    /// freezes that node until rung 0.7. The freeze is root POLICY, so no
    /// amount of reading the row's text can reach it.
    ///
    /// Derived from the row that declares it, not listed: `V15` records what
    /// a hardcoded vocabulary costs -- it knew nine of seventeen nodes and
    /// silently missed every node added after it was written.
    #[test]
    fn a_frozen_node_is_never_offered_as_a_step() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let frozen = frozen_nodes(root);
        assert!(
            frozen.contains(&root.join("src/ollama")),
            "the root spec freezes the model half: {frozen:?}"
        );
        let p = plan(root);
        for s in &p.steps {
            assert!(
                !frozen.contains(&root.join(&s.node)),
                "{} is frozen and was offered as a step",
                s.node.display()
            );
        }
        // Listed, not hidden -- `V3` says silence would read as coverage.
        assert!(
            p.unmanaged.iter().any(|(_, k)| *k == Kind::Frozen),
            "frozen rows are reported with their reason"
        );
    }

    /// The list resolves against the TREE, so a name in the row that is not a
    /// node cannot silently freeze nothing -- or everything.
    #[test]
    fn the_freeze_names_only_real_nodes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for f in frozen_nodes(root) {
            assert!(
                f.join("SPEC.md").is_file(),
                "{} is not a node",
                f.display()
            );
        }
    }

    #[test]
    fn an_untried_node_outranks_one_that_has_failed() {
        // Laplace: untried 0.5, one failure 1/3, three failures 1/5.
        let untried = 1.0 / 2.0;
        let failed_once = 1.0 / 3.0;
        let failed_thrice = 1.0 / 5.0;
        assert!(untried > failed_once && failed_once > failed_thrice);
    }

    #[test]
    fn believability_of_an_unknown_node_is_neutral() {
        let b = believability(Path::new("src/never-seen-before"));
        assert!((b - 0.5).abs() < 1e-9, "untried must be neutral, got {b}");
    }

    #[test]
    fn propose_moves_a_single_node_row() {
        assert_eq!(
            propose("orphan check: SPEC w/o parent §F row"),
            Proposal::Move("fed")
        );
        assert_eq!(
            propose("tier select from sherd.toml"),
            Proposal::Move("tokens")
        );
    }

    #[test]
    fn propose_decomposes_a_multi_node_row() {
        match propose("`§F`.tokens staleness, tier-tagged") {
            Proposal::Decompose(ns) => assert!(ns.len() > 1, "{ns:?}"),
            other => panic!("expected Decompose, got {other:?}"),
        }
    }

    #[test]
    fn propose_keeps_a_row_with_no_node_vocabulary() {
        assert_eq!(
            propose("report the caveman finding upstream"),
            Proposal::Keep
        );
    }

    #[test]
    fn triage_never_proposes_moving_a_row_to_where_it_already_is() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (t, _, p) in triage(root) {
            if let Proposal::Move(n) = p {
                assert!(
                    t.node.file_name().is_none_or(|f| f != n),
                    "{} {} proposed to move to its own node",
                    t.node.display(),
                    t.id
                );
            }
        }
    }

    #[test]
    fn triage_returns_only_unmanaged_rows() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (t, k, _) in triage(root) {
            assert!(
                !k.actionable(),
                "{} {} is actionable, should not be triaged",
                t.node.display(),
                t.id
            );
        }
    }

    #[test]
    fn cited_invariant_takes_the_first_v_id() {
        let mk = |c: &str| Task {
            node: PathBuf::from("src/fed"),
            id: "T4".into(),
            status: '.',
            text: "x".into(),
            cites: c.into(),
        };
        assert_eq!(cited_invariant(&mk("V2,V4")).unwrap().1, "V2");
        assert_eq!(cited_invariant(&mk("I,V7")).unwrap().1, "V7");
        assert_eq!(cited_invariant(&mk("-")), None);
        assert_eq!(
            cited_invariant(&mk("B9")),
            None,
            "a §B cite is not an invariant"
        );
        // bare -> this node; `.:` -> root; `path:` -> that node (B6)
        assert_eq!(
            cited_invariant(&mk("V2")).unwrap().0,
            PathBuf::from("src/fed")
        );
        let (owner, id) = cited_invariant(&mk("`.:V73`")).unwrap();
        assert_eq!((owner, id.as_str()), (PathBuf::new(), "V73"));
        assert_eq!(
            cited_invariant(&mk("`src/lens:V4`")).unwrap().0,
            PathBuf::from("src/lens")
        );
    }

    #[test]
    fn row_identity_follows_its_text() {
        let a = Task {
            node: PathBuf::from("src/fed"),
            id: "T4".into(),
            status: '.',
            text: "do a thing".into(),
            cites: "V2".into(),
        };
        let mut b = a.clone();
        b.text = "do a different thing".into();
        assert_eq!(row_key(&a), row_key(&b), "key is node+id");
        assert_ne!(
            row_hash(&a),
            row_hash(&b),
            "editing the text makes it new work"
        );
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

// ---- believability: weight a node by its track record ----

/// Record an attempt against a node, and how it turned out.
///
/// `kept` counts only what survived REVIEW, not what passed the gate -- the
/// gate has gone green on three stubs, so counting commits would measure the
/// wrong thing.
/// Record one outcome IN a given store.
///
/// The store is a parameter because the ambient one is shared: `.sherd-state`
/// lives in the repo, every test that touches it races every other, and
/// `.coverage` records this suite's coverage FLAPPING for exactly that
/// reason (`src/ollama:T4`). A scorekeeper that can only write one global
/// file cannot be tested without becoming the thing it is measuring.
pub fn record_outcome_in(
    st: &mut crate::state::State,
    node: &Path,
    kept: bool,
) {
    let key = node.to_string_lossy().to_string();
    let mut bump = |k: &str| {
        let n = st.get_u64("score", &format!("{key}.{k}")).unwrap_or(0) + 1;
        st.set("score", &format!("{key}.{k}"), n.to_string());
    };
    bump("tried");
    if kept {
        bump("kept");
    }
}

/// Record one outcome in the ambient store, and save it.
pub fn record_outcome(node: &Path, kept: bool) {
    let mut st = crate::state::State::load();
    record_outcome_in(&mut st, node, kept);
    st.save();
}

/// How much to believe a node's next row will survive review, from a given
/// store.
///
/// Laplace-smoothed: `(kept + 1) / (tried + 2)`. An untried node scores 0.5,
/// so it outranks one that has failed three times without pretending to know
/// it is good. Dalio's claim, mechanised: opinions are not equal, weight them
/// by track record -- and `plan` was treating every row as equally likely to
/// work, which is exactly what he argues against.
#[must_use]
pub fn believability_in(st: &crate::state::State, node: &Path) -> f64 {
    let (tried, kept) = record_in(st, node);
    #[allow(clippy::cast_precision_loss)]
    {
        (kept as f64 + 1.0) / (tried as f64 + 2.0)
    }
}

/// [`believability_in`] against the ambient store.
#[must_use]
pub fn believability(node: &Path) -> f64 {
    believability_in(&crate::state::State::load(), node)
}

/// `(tried, kept)` for a node, from a given store.
#[must_use]
pub fn record_in(st: &crate::state::State, node: &Path) -> (u64, u64) {
    let key = node.to_string_lossy().to_string();
    (
        st.get_u64("score", &format!("{key}.tried")).unwrap_or(0),
        st.get_u64("score", &format!("{key}.kept")).unwrap_or(0),
    )
}

/// `(tried, kept)` for a node from the ambient store, for reporting.
#[must_use]
pub fn record(node: &Path) -> (u64, u64) {
    record_in(&crate::state::State::load(), node)
}

// ---- triage: where does an unmanaged row belong? ----

/// Node keywords. A row naming exactly one node's vocabulary probably belongs
/// to that node. ADVISORY -- prose classification is wrong-by-default (V4,
/// B1), so this proposes and a reader decides.
const VOCAB: [(&str, &[&str]); 9] = [
    (
        "fed",
        &[
            "§f",
            "§n",
            "edge",
            "dag",
            "cycle",
            "chain",
            "discover",
            "graph",
            "orphan",
            "promotion",
        ],
    ),
    ("lens", &["lens", "pack", "budget", "depth", "why", "facet"]),
    ("tokens", &["token", "tier", "itok", "count", "ceiling"]),
    ("spec", &["microlith", "section", "record", "format", "fmt"]),
    ("ollama", &["ollama", "endpoint", "retry", "model"]),
    ("tdd", &["tdd", "judge", "red", "green", "repair"]),
    ("plan", &["plan", "apply", "horizon", "needs", "actionable"]),
    ("review", &["review", "unwired", "negative"]),
    ("state", &["state", "cache", "idempot"]),
];

/// What triage proposes for one row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Proposal {
    /// Work lands in exactly one node: move the row there.
    Move(&'static str),
    /// Names several nodes' work: split into one row per node.
    Decompose(Vec<&'static str>),
    /// No node vocabulary -- root, CLI, or research. Read it.
    Keep,
}

/// Propose where a row belongs, from the vocabulary it uses.
#[must_use]
pub fn propose(text: &str) -> Proposal {
    let t = text.to_lowercase();
    let hits: Vec<&'static str> = VOCAB
        .iter()
        .filter(|(_, ks)| ks.iter().any(|k| t.contains(k)))
        .map(|(n, _)| *n)
        .collect();
    // The pattern carries the arity: exactly one hit MOVES, and anything
    // else is Keep or Decompose. A `len()` match plus an index said it twice.
    match hits.as_slice() {
        [one] => Proposal::Move(one),
        [] => Proposal::Keep,
        _ => Proposal::Decompose(hits),
    }
}

/// Every unmanaged row with a proposal and the reason it is unmanaged.
#[must_use]
pub fn triage(root: &Path) -> Vec<(Task, Kind, Proposal)> {
    open_tasks(root)
        .into_iter()
        .map(|t| {
            let k = classify(&root.join(&t.node), &t.text);
            let mut p = propose(&t.text);
            // A row already living in the node it names is not a move.
            if let Proposal::Move(n) = p
                && t.node.file_name().is_some_and(|f| f == n)
            {
                p = Proposal::Keep;
            }
            (t, k, p)
        })
        .filter(|(_, k, _)| !k.actionable())
        .collect()
}

// ---- apply: execute exactly one step, then stop ----

/// Refuse to run unless the tree is safe to commit into.
///
/// `apply` writes source AND commits. Both are recoverable only if the tree
/// was clean beforehand and the branch is not the trunk.
fn preflight(root: &Path) -> Result<String, String> {
    let git = |args: &[&str]| {
        crate::git::at(root, args)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    };
    let branch =
        git(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
    if branch.is_empty() {
        return Err("not a git repo -- apply commits, so it needs one".into());
    }
    if !git(&["status", "--porcelain"])
        .unwrap_or_default()
        .is_empty()
    {
        return Err("working tree dirty -- commit or stash first, so the \
                    generated diff is the only thing in the commit"
            .into());
    }
    // Generated code never lands on the trunk directly. This used to REFUSE
    // on main; refusing is the right requirement expressed as an obstacle, so
    // it now satisfies the requirement instead -- the run gets a branch. What
    // moves that branch onto main is `sherd land`, which asks for evidence.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let want = crate::land::run_branch(&branch, secs);
    if want != branch {
        crate::git::at(root, &["checkout", "-q", "-b", &want])
            .status()
            .map_err(|e| e.to_string())?;
        eprintln!("apply: on {branch} -- generated code goes to {want}");
        return Ok(want);
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
    let (owner, inv) = cited_invariant(step).ok_or_else(|| {
        format!(
            "{} {} cites no §V id ({}) -- apply drives an INVARIANT, not prose",
            step.node.display(),
            step.id,
            step.cites
        )
    })?;

    eprintln!(
        "apply: {} {} on {branch}\n  invariant {inv} (declared in {})\n  task {}",
        step.node.display(),
        step.id,
        if owner.as_os_str().is_empty() {
            ".".into()
        } else {
            owner.display().to_string()
        },
        step.text
    );

    let node = root.join(&step.node);
    let log = match crate::tdd::drive_from(
        root,
        &node,
        &root.join(&owner),
        &inv,
        &step.text,
        max_repair,
    ) {
        Ok(l) => l,
        Err(e) => {
            record_outcome(&step.node, false);
            return Err(e);
        }
    };

    let sent: u64 = log.iter().map(|s| s.prompt_tokens).sum();
    let max = log.iter().map(|s| s.prompt_tokens).max().unwrap_or(0);
    let body = format!(
        "feat({}): {} via sherd apply\n\n\
         {inv}: driven from {} {}.\n\n\
         Written by a local model through the red -> judge -> green -> gate\n\
         loop: {} round-trips, {sent} tokens sent, max single call {max}.\n\
         The gate ({}) passed before this commit existed.\n\n\
         Generated code. Read the diff -- a green gate is not correctness\n\
         (.:tdd V11), and this loop has twice produced code that passed both\n\
         gates while testing the wrong thing.\n",
        step.node
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("sherd"),
        step.text,
        step.node.display(),
        step.id,
        log.len(),
        "cargo test + sherd check"
    );

    let git = |args: &[&str]| {
        crate::git::at(root, args)
            .status()
            .map_err(|e| e.to_string())
    };
    git(&["add", "-A"])?;
    let st = crate::git::at(root, &["commit", "-q", "-m", &body])
        .status()
        .map_err(|e| e.to_string())?;
    if !st.success() {
        return Err(
            "commit refused by the gate -- generated code is still in \
                    the tree, uncommitted"
                .into(),
        );
    }
    let sha = crate::git::at(root, &["rev-parse", "--short", "HEAD"])
        .output()
        .map_err(|e| e.to_string())?;
    let sha = String::from_utf8_lossy(&sha.stdout).trim().to_string();

    // SYNC: the gate said green; review may not agree. Surface the
    // disagreement at the moment it happens rather than waiting for someone
    // to type `sherd review` -- which is how an unwired function and an ignored
    // input both landed unnoticed.
    match crate::review::commit(root, &sha) {
        Ok(f) if f.is_empty() => eprintln!("  review: no findings"),
        Ok(f) => {
            eprintln!(
                "\n  REVIEW DISAGREES WITH THE GATE -- {} finding(s):",
                f.len()
            );
            for (file, x) in &f {
                eprintln!("    {}: {}: {}", file.display(), x.rule, x.detail);
            }
            eprintln!(
                "  advisory (review V3). Read the diff before `sherd outcome ... kept`."
            );
        }
        Err(e) => eprintln!("  review: could not run -- {e}"),
    }

    // Every commit goes to the remote so CI runs on it -- an independent
    // check on a machine that did not write the code. The run BRANCH, never
    // main; the trunk moves only through `sherd land`.
    crate::land::push_branch(root, &branch);

    // Record it applied, keyed by the row's TEXT -- edit the row and it
    // becomes plannable again.
    // Counted as an ATTEMPT here; `kept` is claimed only after review, via
    // `sherd outcome`. A commit is not survival -- three stubs have committed.
    record_outcome(&step.node, false);
    let mut state = crate::state::State::load();
    state.set(
        "applied",
        &row_key(step),
        format!("{} {sha}", row_hash(step)),
    );
    state.clear_kind("plan"); // the plan that produced this is now stale
    state.save();
    Ok(sha)
}

#[cfg(test)]
mod git_tests {
    use super::*;
    use crate::testrepo::TestRepo;

    #[test]
    fn preflight_refuses_a_tree_that_is_not_a_repo() {
        // `apply` COMMITS, so it needs a repo. Saying so beats failing later
        // with a git error nobody reads.
        let d = std::env::temp_dir().join("sherd-not-a-repo");
        let _ = std::fs::create_dir_all(&d);
        let r = preflight(&d);
        let _ = std::fs::remove_dir_all(&d);
        assert!(r.is_err(), "a non-repo must be refused");
    }

    #[test]
    fn preflight_refuses_a_dirty_tree() {
        assert_eq!(check_dirty(), Ok(()));
    }

    /// A dirty tree means the generated diff would not be the only thing in
    /// the commit, which is the whole point of the branch `apply` makes.
    fn check_dirty() -> Result<(), String> {
        let r = TestRepo::new("plan-dirty")?;
        r.write("stray.txt", "uncommitted\n")?;
        let out = preflight(r.path());
        let Err(msg) = out else {
            return Err("a dirty tree must be refused".into());
        };
        assert!(msg.contains("dirty"), "the refusal must say why: {msg}");
        Ok(())
    }

    /// A real node, so `classify` gets past its `mod.rs` guard.
    fn node() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed")
    }

    #[test]
    fn a_dir_with_no_module_is_never_actionable() {
        // The loop edits ONE node's `mod.rs`. A row whose node has none has
        // nowhere for the code to go, whatever the row says.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(classify(root, "add one pure function"), Kind::NoModule);
    }

    /// Rows the loop CANNOT do, each with the bug that put it there.
    ///
    /// `classify` is five recorded defects deep -- B1, B4, B5, B8, B9 -- and
    /// every one was a row read as ACTIONABLE that the loop then could not
    /// perform. A misclassification does not fail loudly: it sends the 20B at
    /// work it cannot do, which is `src/tdd:T13`'s zero.
    #[test]
    fn a_row_that_edits_existing_code_is_never_actionable() {
        let n = node();
        // B4: a BLACKLIST marked "replace the hand-rolled walk" actionable,
        // and the loop only appends.
        assert_eq!(
            classify(&n, "replace the hand-rolled walk"),
            Kind::Replaces
        );
        // B9: a POSITION word names where code goes RELATIVE to existing
        // code, so it needs an edited call site however it is phrased.
        assert_eq!(
            classify(&n, "retry around `Transport::post`"),
            Kind::Replaces
        );
        assert_eq!(classify(&n, "wrap the existing judge"), Kind::Replaces);
    }

    #[test]
    fn a_row_that_is_not_a_function_at_all_is_named_as_such() {
        let n = node();
        assert_eq!(
            classify(&n, "blocked -- needs Rust source"),
            Kind::NotAFunction
        );
        assert_eq!(classify(&n, "promote the node"), Kind::NotAFunction);
        assert_eq!(classify(&n, "add a `sherd foo` verb"), Kind::Cli);
        assert_eq!(classify(&n, "audit the corpus upstream"), Kind::NotCode);
    }

    #[test]
    fn a_multi_file_row_is_caught_whatever_the_word_order() {
        // B1: the first version looked for "derive `§n`" and the row said
        // "`§N` derive from parent `§F`". Widening a substring list is a
        // patch; word-order independence is the fix.
        let n = node();
        let k = Kind::MultiFile;
        assert_eq!(classify(&n, "`\u{a7}N` derive from parent `\u{a7}F`"), k);
        assert_eq!(classify(&n, "derive `\u{a7}N` from the parent table"), k);
    }

    #[test]
    fn the_whitelist_still_says_yes_to_one_added_function() {
        // The classifier must not become a machine that refuses everything:
        // a detector tested only on the negative case is satisfied by
        // returning the negative (`src/fed:V10`).
        assert_eq!(
            classify(&node(), "add a pure function that counts cells"),
            Kind::NodeFn
        );
    }

    /// Every stem, in its `-ing` form, with the class it must land in.
    const ING: &[(&str, Kind)] = &[
        ("wiring the detector into check", Kind::NotAFunction),
        ("moving the corpus to a fixture", Kind::NotAFunction),
        ("promoting the node to a sibling", Kind::NotAFunction),
        ("recording the outcome", Kind::NotAFunction),
        ("replacing the hand-rolled walk", Kind::Replaces),
        ("removing the stub", Kind::Replaces),
        ("migrating to itok::walk", Kind::Replaces),
        ("rewriting the judge", Kind::Replaces),
        ("deleting the dead arm", Kind::Replaces),
        ("superseding the old row", Kind::Replaces),
    ];

    #[test]
    fn every_stem_matches_its_own_ing_form() {
        // B11, and `V14`: B8 recorded "missed `wiring` (list had `wire`)"
        // and shipped a stem match that STILL did not match `wiring`, so the
        // row read as closed while its own example still failed. Nine of the
        // eighteen stems were affected -- every one ending in `e`.
        let n = node();
        for (row, want) in ING {
            assert_eq!(
                classify(&n, row),
                *want,
                "`{row}` must not read as actionable -- the loop cannot do it"
            );
        }
    }

    #[test]
    fn report_is_not_a_replacement_even_though_it_contains_port() {
        // B5 exactly: a SUBSTRING list classified every `report ...` row as a
        // replacement, because "report" contains "port". The fix was to match
        // WORDS, and this is the assertion that holds it.
        assert_eq!(classify(&node(), "report the drift"), Kind::NodeFn);
    }

    /// A `§T` row citing `cites`, at `src/plan`.
    fn cite_row(cites: &str) -> Task {
        Task {
            node: std::path::PathBuf::from("src/plan"),
            id: "T1".into(),
            text: String::new(),
            cites: cites.into(),
            status: '.',
        }
    }

    /// A store of its own, so this test races nothing.
    fn store(tag: &str) -> crate::state::State {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        crate::state::State::at(
            std::env::temp_dir()
                .join(format!("sherd-score-{tag}-{}-{n}", std::process::id())),
        )
    }

    #[test]
    fn an_untried_node_outranks_one_that_has_failed() {
        // Laplace: `(kept + 1) / (tried + 2)`. An untried node scores 0.5, so
        // it goes ahead of a node that failed three times WITHOUT pretending
        // to know it is good. A raw ratio would score the untried node 0/0
        // and the failed one 0.00, making them indistinguishable.
        let mut st = store("laplace");
        let untried = Path::new("src/never-tried");
        let failed = Path::new("src/failed");
        for _ in 0..3 {
            record_outcome_in(&mut st, failed, false);
        }
        let u = believability_in(&st, untried);
        let f = believability_in(&st, failed);
        assert!(
            (u - 0.5).abs() < 1e-9,
            "an untried node sits at 0.5, got {u}"
        );
        assert!(f < u, "three failures rank BELOW untried: {f} vs {u}");
        assert_eq!(record_in(&st, failed), (3, 0), "tried counted, kept not");
    }

    #[test]
    fn five_consecutive_keeps_clears_the_landing_bar() {
        // `src/land:V2` puts LAND_MIN at 0.85 and calls it "5 consecutive
        // keeps under Laplace". That is an arithmetic claim in prose, and
        // nothing checked it: 6/7 = 0.857, so five is the number and four
        // (5/6 = 0.833) is not.
        let mut st = store("bar");
        let n = Path::new("src/proven");
        for _ in 0..4 {
            record_outcome_in(&mut st, n, true);
        }
        assert!(
            believability_in(&st, n) < 0.85,
            "four keeps is 5/6 = 0.833, below the bar"
        );
        record_outcome_in(&mut st, n, true);
        assert!(
            believability_in(&st, n) >= 0.85,
            "five keeps is 6/7 = 0.857, and V2 says that clears it"
        );
    }

    #[test]
    fn a_kept_outcome_counts_in_both_tallies_and_a_reverted_one_in_neither() {
        // `kept` counts what survived REVIEW, not what passed the gate -- the
        // gate has gone green on three stubs, so counting commits would
        // measure the wrong thing.
        let mut st = store("tally");
        let n = Path::new("src/mixed");
        record_outcome_in(&mut st, n, true);
        record_outcome_in(&mut st, n, false);
        assert_eq!(record_in(&st, n), (2, 1), "two tries, one kept");
    }

    #[test]
    fn a_bare_cite_belongs_to_the_row_s_own_node() {
        // Ids are node-scoped (`.:V10`): a bare `V9` and a namespaced
        // `src/fed:V9` must not resolve to the same file.
        assert_eq!(
            cited_invariant(&cite_row("V9")),
            Some((std::path::PathBuf::from("src/plan"), "V9".into()))
        );
    }

    #[test]
    fn a_namespaced_cite_names_its_owner_and_dot_is_root() {
        assert_eq!(
            cited_invariant(&cite_row("`src/fed:V9`")),
            Some((std::path::PathBuf::from("src/fed"), "V9".into()))
        );
        assert_eq!(
            cited_invariant(&cite_row("`.:V73`")),
            Some((std::path::PathBuf::new(), "V73".into())),
            "`.` is the root node"
        );
    }

    #[test]
    fn a_row_citing_no_invariant_yields_none() {
        // Not every row cites a §V, and inventing one would send the loop at
        // an invariant nobody wrote.
        assert_eq!(cited_invariant(&cite_row("R44,I")), None);
        assert_eq!(cited_invariant(&cite_row("Vx,V1a")), None, "not an id");
    }

    #[test]
    fn a_row_naming_two_nodes_is_decomposed_not_moved() {
        // A row that names one node's vocabulary can MOVE; one that names
        // two is work for two nodes and moving it would just relocate the
        // ambiguity.
        assert!(matches!(propose("count the tokens"), Proposal::Move(_)));
        assert!(matches!(
            propose("count the tokens and render the lens pack"),
            Proposal::Decompose(v) if v.len() >= 2
        ));
        assert!(matches!(propose("think about it"), Proposal::Keep));
    }

    #[test]
    fn open_tasks_reads_this_repo_and_skips_what_is_done() {
        // The federation's own §T rows. `x` is history, and a machine told to
        // test what already passes learns nothing (`src/fed:V9`).
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let ts = open_tasks(root);
        assert!(!ts.is_empty(), "this repo has open rows");
        assert!(
            ts.iter().all(|t| t.status != 'x'),
            "a done row is not an open task"
        );
        assert!(
            ts.iter().any(|t| t.node.ends_with("src/fed")),
            "rows are collected across nodes, not just root"
        );
    }

    #[test]
    fn a_plan_ranks_what_it_can_act_on_and_counts_what_it_cannot() {
        // R46: 3 actionable of 78. The UNMANAGED list is the honest half --
        // a horizon of 3 that hid 75 rows would read as a nearly finished
        // project, and `.:B4` is exactly a ratio whose denominator lied.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let p = plan(root);
        assert!(p.total_open > 0, "this repo has open rows");
        assert!(
            p.steps.len() <= p.total_open,
            "the horizon cannot exceed the rows it came from"
        );
        assert!(
            !p.unmanaged.is_empty(),
            "R46: most rows are unmanaged, and they must be REPORTED"
        );
        assert!(
            p.steps.len() + p.unmanaged.len() <= p.total_open,
            "no row may be counted in both halves"
        );
    }

    #[test]
    fn preflight_on_a_clean_repo_names_a_run_branch() {
        assert_eq!(check_clean(), Ok(()));
    }

    /// Generated code never lands on the trunk directly: `preflight` puts the
    /// run on its own branch, and `sherd land` is what moves it, on evidence.
    fn check_clean() -> Result<(), String> {
        let r = TestRepo::new("plan-clean")?;
        let branch = preflight(r.path())?;
        assert!(
            branch.starts_with("sherd/"),
            "a run gets its own branch, got {branch}"
        );
        let now = r.git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
        assert_eq!(now, branch, "preflight must have switched to it");
        Ok(())
    }
}

// ---- route: which node answers this question? ----

/// Where a query lands, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Exactly one node matched. The reason is the words that matched.
    Hit(std::path::PathBuf, Vec<String>),
    /// Several matched: the query spans nodes, and picking one would be a
    /// guess dressed as an answer.
    Ambiguous(Vec<std::path::PathBuf>),
    /// Nothing matched. A miss is reported, never rounded to the nearest
    /// node -- `.:V4`'s rule, since prose classification is wrong by default.
    Miss,
}

/// The words one node answers to: its directory name, plus its `§G`.
fn node_words(node: &Path) -> Vec<String> {
    let mut words = Vec::new();
    if let Some(name) = node.file_name().and_then(|n| n.to_str()) {
        words.push(name.to_lowercase());
    }
    if let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) {
        words.extend(goal_words(&text));
    }
    words.sort();
    words.dedup();
    words
}

/// The words that identify a node, DERIVED from the tree rather than listed.
///
/// A hardcoded table covers the nodes it was written for and silently misses
/// every one added since -- `VOCAB` above knows nine of this repository's
/// seventeen, which is the shape of drift `sherd check` exists to catch, in
/// the checker's own source. So the vocabulary is read: a node's directory
/// name, plus the significant words of its `§G` goal.
///
/// Words shorter than four characters are dropped. They are the articles and
/// operators of caveman prose, and one of them appearing in a query would
/// match every node that used it.
#[must_use]
pub fn vocabulary(root: &Path) -> Vec<(std::path::PathBuf, Vec<String>)> {
    fed::discover(root)
        .into_iter()
        .map(|node| {
            let words = node_words(&node);
            (node, words)
        })
        .collect()
}

/// The `§G` line's own words, lowercased, four characters or longer.
fn goal_words(spec: &str) -> Vec<String> {
    crate::spec::sections(spec)
        .into_iter()
        .find(|(name, _)| {
            // `sections` yields the WHOLE heading line, `## §G GOAL`, so a
            // `starts_with("§G")` matches nothing and every node silently
            // reduces to its directory name.
            name.trim_start_matches('#')
                .trim_start()
                .starts_with("\u{a7}G")
        })
        .map(|(_, body)| {
            body.split(|c: char| !c.is_alphanumeric())
                .filter(|w| w.chars().count() >= 4)
                .map(str::to_lowercase)
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve a query to the node that owns it.
///
/// ADVISORY in the same sense `propose` is: this reports what the specs say
/// about themselves, and a reader decides. What it will not do is round a
/// miss up to the nearest node, because a confident wrong answer costs more
/// than "I do not know" -- the query's author can read a miss and rephrase.
#[must_use]
/// Every node whose vocabulary the query touches, and the words it touched.
///
/// The ROOT is never an answer. It owns the whole repository by definition,
/// and its `§G` names every concern below it, so leaving it in makes almost
/// every query ambiguous against a node that tells the asker nothing.
fn route_hits(
    root: &Path,
    terms: &[&str],
) -> Vec<(std::path::PathBuf, Vec<String>)> {
    let mut hits: Vec<(std::path::PathBuf, Vec<String>)> = Vec::new();
    for (node, words) in vocabulary(root) {
        if node == root {
            continue;
        }
        let matched: Vec<String> = words
            .into_iter()
            .filter(|w| terms.iter().any(|t| t == w))
            .collect();
        if !matched.is_empty() {
            hits.push((node, matched));
        }
    }
    hits
}

#[must_use]
pub fn route(root: &Path, query: &str) -> Route {
    let q = query.to_lowercase();
    let terms: Vec<&str> = q
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4)
        .collect();
    let mut hits = route_hits(root, &terms);
    // RANKED, not counted. Several nodes share a word -- "spec" appears in
    // most goals -- so presence alone makes everything ambiguous; the node
    // matching MORE of the query is the one that owns it. A tie is genuinely
    // ambiguous and says so.
    let best = hits.iter().map(|(_, w)| w.len()).max().unwrap_or(0);
    hits.retain(|(_, w)| w.len() == best);
    match hits.len() {
        0 => Route::Miss,
        1 => hits
            .pop()
            .map_or(Route::Miss, |(node, why)| Route::Hit(node, why)),
        _ => Route::Ambiguous(hits.into_iter().map(|(n, _)| n).collect()),
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;

    #[test]
    fn a_query_naming_one_node_resolves_to_it() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Route::Hit(node, why) =
            route(root, "how does the ollama endpoint retry")
        else {
            unreachable!("`ollama` names exactly one node")
        };
        assert!(node.ends_with("ollama"), "{}", node.display());
        assert!(why.contains(&"ollama".to_string()), "the reason: {why:?}");
    }

    /// A miss is REPORTED, never rounded to the nearest node. The query's
    /// author can read a miss and rephrase; a confident wrong node sends
    /// them to read the wrong file.
    #[test]
    fn a_query_naming_nothing_is_a_miss() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert_eq!(route(root, "wombat marmalade trebuchet"), Route::Miss);
    }

    /// A tie is genuinely ambiguous, and saying so beats picking the first.
    #[test]
    fn a_query_spanning_two_nodes_equally_is_ambiguous() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Route::Ambiguous(nodes) = route(root, "ollama tokens") else {
            unreachable!("one word each from two nodes is a tie")
        };
        assert!(nodes.len() >= 2, "{nodes:?}");
    }

    /// The ROOT owns everything and therefore answers nothing.
    #[test]
    fn the_root_is_never_the_answer() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for q in ["federation", "spec", "token"] {
            match route(root, q) {
                Route::Hit(node, _) => assert_ne!(node, root, "{q}"),
                Route::Ambiguous(nodes) => {
                    assert!(!nodes.contains(&root.to_path_buf()), "{q}");
                }
                Route::Miss => {}
            }
        }
    }

    /// Short words are dropped: they are caveman prose's articles, and one
    /// of them would match every node that ever used it.
    #[test]
    fn words_under_four_characters_carry_no_signal() {
        assert!(goal_words("## \u{a7}G GOAL\n\na of the is\n").is_empty());
    }

    /// The §G half has to be REAL, not merely non-empty: the directory name
    /// alone satisfies "has words", and it did while `goal_words` silently
    /// returned nothing for every node.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the count and the CONTENT are one property here: a \
                  vocabulary of the right size built from directory names \
                  alone is exactly the defect this asserts against"
    )]
    fn a_vocabulary_is_derived_for_every_node_the_walk_finds() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let vocab = vocabulary(root);
        assert_eq!(vocab.len(), fed::discover(root).len());
        assert!(
            vocab.iter().all(|(_, w)| !w.is_empty()),
            "a node with no words can never be routed to"
        );
        let fed_words = vocab
            .iter()
            .find(|(n, _)| n.ends_with("fed"))
            .map(|(_, w)| w.clone())
            .unwrap_or_default();
        assert!(
            fed_words.iter().any(|w| w == "federation"),
            "§G's own words reach the vocabulary: {fed_words:?}"
        );
    }
}

// ---- split: what would come off this node's chain? ----

/// Spec lines naming one module, whole-word and case-sensitive, outside the
/// STRUCTURAL sections.
///
/// `§F` and `§N` describe the federation rather than state law about it, and
/// `§N` is GENERATED by `sync` into every node, where it names every sibling.
/// Counting them made this function read its own generator's output: split,
/// `sync` writes more `§N`, the weights rise, split proposes more (`V18`).
fn naming_rows(spec: &str, name: &str) -> Vec<String> {
    let mut rows = Vec::new();
    let mut structural = false;
    for line in spec.lines() {
        if let Some(section) = line.strip_prefix("## \u{a7}") {
            structural = section.starts_with('F') || section.starts_with('N');
            continue;
        }
        if !structural && names_word(line, name) {
            rows.push(line.to_string());
        }
    }
    rows
}

/// Whole-word match, so `plan` does not match `planning` and `check` does not
/// match `checked`. A substring match reported every row for every module on
/// the first tree this ran against.
///
/// CASE-SENSITIVE, because the earlier lowercasing split the filename
/// `SPEC.md` into `spec` and counted every row naming the FILE as a row about
/// the NODE: 54 rows against 27 real ones, half the column a filename
/// (`B17`). Node names are directory names and directories here are
/// lowercase, so the case carries the distinction for free.
fn names_word(line: &str, name: &str) -> bool {
    line.split(|c: char| !c.is_alphanumeric() && c != '_')
        .any(|w| w == name)
}

#[cfg(test)]
mod split_tests {
    use super::*;

    #[test]
    fn a_module_named_by_no_row_is_not_a_candidate() {
        let spec = "## \u{a7}V INVARIANTS\n\nV1: the ledger counts fires\n";
        assert!(naming_rows(spec, "ledger").len() == 1);
        assert!(naming_rows(spec, "corpus").is_empty());
    }

    /// `B17`: the evidence grade discriminates in ONE of six repositories
    /// measured. Everywhere else every module carries the same grade, which
    /// leaves the spec rows as the only signal -- and alphabetical order
    /// threw it away.
    #[test]
    fn proposals_lead_with_the_heaviest_not_the_alphabetically_first() {
        let spec = "## \u{a7}V INVARIANTS\n\
                    V1: `heavy` does a thing\nV2: `heavy` does another\n\
                    V3: `heavy` again\nV4: `light` once\n";
        let p = |name: &str, e: Evidence| Proposed {
            name: name.to_string(),
            evidence: e,
            members: vec![name.to_string()],
            shared: Vec::new(),
            split_layout: false,
        };
        let nodes = [
            p("light", Evidence::Published),
            p("heavy", Evidence::Published),
        ];
        let order: Vec<&str> = rank(&nodes, spec)
            .iter()
            .map(|r| r.node.name.as_str())
            .collect();
        assert_eq!(order, vec!["heavy", "light"], "heaviest first");
        assert_eq!(rank(&nodes, spec).first().map(|r| r.rows), Some(3));
    }

    /// A grade every module shares ranks nothing, and a reader who takes the
    /// order for a verdict is reading spec rows rather than structure.
    ///
    /// MEASURED: 4 of 6 repositories are perfectly uniform -- `sherd` all
    /// directory, `ashlar` and `metope` all `pub mod`, `microlith` all
    /// declared -- and only `itok` carries a mixed profile.
    #[test]
    fn a_grade_every_module_shares_is_reported_as_ranking_nothing() {
        let p = |name: &str, e: Evidence| Proposed {
            name: name.to_string(),
            evidence: e,
            members: vec![name.to_string()],
            shared: Vec::new(),
            split_layout: false,
        };
        assert!(uniform_evidence(&[
            p("a", Evidence::Published),
            p("b", Evidence::Published)
        ]));
        assert!(!uniform_evidence(&[
            p("a", Evidence::Drawn),
            p("b", Evidence::Published)
        ]));
        // One node ranks nothing either way, and saying so would be noise.
        assert!(!uniform_evidence(&[p("a", Evidence::Drawn)]));
        assert!(!uniform_evidence(&[]));
    }

    /// Whole-word, or every module matches every row: `plan` inside
    /// "planning" and `check` inside "checked" made the first version
    /// propose the entire spec for every candidate.
    #[test]
    fn a_name_matches_as_a_word_and_not_as_a_substring() {
        assert!(names_word("the plan is fixed", "plan"));
        assert!(!names_word("planning is not planning", "plan"));
        assert!(names_word("src/ledger.rs holds it", "ledger"));
        assert!(!names_word("no mention here", "ledger"));
    }

    /// The layout `.:§C` forbids, detected where the node is proposed: both
    /// `<name>.rs` and `<name>/` exist, so neither half can carry a spec
    /// until they are merged. `rekall` has it for `cli`.
    #[test]
    fn a_module_that_is_both_a_file_and_a_directory_is_flagged() {
        let dir = std::env::temp_dir().join(format!(
            "sherd-layout-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let src = dir.join("src");
        let Ok(()) = std::fs::create_dir_all(src.join("both")) else {
            unreachable!("a src dir is creatable")
        };
        let Ok(()) =
            std::fs::write(src.join("lib.rs"), "mod both;\nmod solo;\n")
        else {
            unreachable!("a lib.rs is writable")
        };
        let Ok(()) = std::fs::write(src.join("both.rs"), "") else {
            unreachable!("a module file is writable")
        };
        let Ok(()) = std::fs::write(src.join("solo.rs"), "") else {
            unreachable!("a module file is writable")
        };

        let found = structure(&dir);
        let flagged = |n: &str| {
            found
                .iter()
                .find(|p| p.name == n)
                .is_some_and(|p| p.split_layout)
        };
        assert!(flagged("both"), "both.rs and both/ exist: {found:?}");
        assert!(!flagged("solo"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A FEDERATED tree proposes the nodes it already has, graded
    /// `directory`, and each still carries whatever weight the parent spec
    /// gives it. `B14` is what the opposite assumption cost: excluding
    /// already-nodes left the weight column reading zero for every one of
    /// them, in the only kind of repository where the question matters.
    #[test]
    fn a_federated_tree_proposes_and_weighs_its_existing_nodes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let found = structure(root);
        assert!(
            found.iter().all(|p| p.evidence == Evidence::Drawn),
            "every module here is a directory: {found:?}"
        );
        let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
            unreachable!("this repository has a root spec")
        };
        let weighed = found
            .iter()
            .filter(|p| row_weight(&spec, &p.name).0 > 0)
            .count();
        assert!(weighed > 1, "root names its nodes on real rows");
    }
}

// ---- structure: what the code already separated ----

/// Why a module is a federation candidate. Ordered: a stronger grade is a
/// boundary the author drew more explicitly (`V17`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Evidence {
    /// Declared in the entry file and nothing more. The weakest grade, and
    /// the one `microlith` is made of: eleven `pub(crate) mod` lines, no
    /// directories, no families, no published surface. A crate can draw
    /// every boundary this way, and dropping the grade made such a crate
    /// propose NOTHING (`B13`).
    Declared,
    /// A naming family plus shared `use crate::` edges.
    Cohesion,
    /// `pub mod` -- the author published it.
    Published,
    /// Already a directory -- the author drew it.
    Drawn,
}

impl Evidence {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Drawn => "directory",
            Self::Published => "pub mod",
            Self::Cohesion => "family",
            Self::Declared => "declared",
        }
    }
}

/// A proposed node: what the code separated, and what it shares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposed {
    pub name: String,
    pub evidence: Evidence,
    /// Both `<name>.rs` AND `<name>/` exist -- the 2018 layout `.:§C`
    /// forbids, which puts the facade outside the directory it fronts.
    /// `rekall` has it for `cli`, and it has to be merged by hand before
    /// either half can carry a `SPEC.md`.
    pub split_layout: bool,
    /// Members, when a naming family stands in for several files.
    pub members: Vec<String>,
    /// Crate modules every member reaches for -- the hub a family shares.
    pub shared: Vec<String>,
}

/// Nodes derived from the source, strongest evidence first.
///
/// STRUCTURE FIRST (`V17`). Spec rows are attached afterwards by the caller
/// and are evidence ABOUT a node, never the thing that proposes it -- a
/// module the code separates and the prose never mentions is still a node,
/// which a row-count ranking cannot see (`B12`).
///
/// Families are reported, never auto-clustered beyond a shared suffix: a
/// graph clustering is where a proposer starts guessing, and `V16` says this
/// proposes.
#[must_use]
pub fn structure(dir: &Path) -> Vec<Proposed> {
    let src = if dir.join("src").is_dir() {
        dir.join("src")
    } else {
        dir.to_path_buf()
    };
    let entry = ["lib.rs", "main.rs", "mod.rs"]
        .iter()
        .map(|f| src.join(f))
        .find(|p| p.is_file());
    let decls = entry
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|t| crate::code::mod_decls(&t))
        .unwrap_or_default();

    let mut out: Vec<Proposed> = Vec::new();
    for family in families(&decls) {
        out.push(proposed_family(&src, family));
    }
    for d in &decls {
        if out.iter().any(|p| p.members.contains(&d.name)) {
            continue;
        }
        let evidence = if src.join(&d.name).is_dir() {
            Evidence::Drawn
        } else if d.is_pub {
            Evidence::Published
        } else {
            Evidence::Declared
        };
        out.push(Proposed {
            split_layout: src.join(&d.name).is_dir()
                && src.join(format!("{}.rs", d.name)).is_file(),
            name: d.name.clone(),
            evidence,
            members: vec![d.name.clone()],
            shared: Vec::new(),
        });
    }
    out.sort_by(|a, b| b.evidence.cmp(&a.evidence).then(a.name.cmp(&b.name)));
    out
}

/// Modules sharing a suffix, when there are enough of them to be a family
/// rather than a coincidence. Two files ending in `cmd` is a pair; eleven is
/// a concern the author named.
fn families(decls: &[crate::code::ModDecl]) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for suffix in ["cmd", "fmt", "args"] {
        let members: Vec<String> = decls
            .iter()
            .map(|d| d.name.clone())
            .filter(|n| n.len() > suffix.len() && n.ends_with(suffix))
            .collect();
        if members.len() >= 3 {
            out.push(members);
        }
    }
    out
}

/// A family, and the crate modules EVERY member reaches for.
fn proposed_family(src: &Path, members: Vec<String>) -> Proposed {
    let uses: Vec<Vec<String>> = members
        .iter()
        .filter_map(|m| {
            std::fs::read_to_string(src.join(format!("{m}.rs"))).ok()
        })
        .map(|t| crate::code::crate_uses(&t))
        .collect();
    let shared = shared_across(&uses);
    let name = members
        .first()
        .and_then(|m| m.get(m.len().saturating_sub(3)..))
        .unwrap_or("group")
        .to_string();
    Proposed {
        name,
        evidence: Evidence::Cohesion,
        members,
        shared,
        // A family is a set of files; the layout question is per-module.
        split_layout: false,
    }
}

/// Names present in EVERY list. A hub is what all members share, not what
/// any of them happens to use.
fn shared_across(lists: &[Vec<String>]) -> Vec<String> {
    let Some(first) = lists.first() else {
        return Vec::new();
    };
    first
        .iter()
        .filter(|n| lists.iter().all(|l| l.contains(n)))
        .cloned()
        .collect()
}

#[cfg(test)]
mod structure_tests {
    use super::*;

    /// This crate is already federated, so every module `lib.rs` declares is
    /// a directory: the strongest grade, and nothing left to infer.
    #[test]
    fn an_already_federated_crate_proposes_its_directories() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let found = structure(root);
        assert!(!found.is_empty());
        assert!(
            found.iter().all(|p| p.evidence == Evidence::Drawn),
            "every module of an already-federated crate is a directory: {found:?}"
        );
    }

    /// A family with a shared hub: the `use crate::` intersection across all
    /// members, which is what makes eleven `*cmd` files one node instead of
    /// eleven (`V17`).
    #[test]
    fn a_family_is_proposed_with_the_hub_its_members_share() {
        let dir = std::env::temp_dir().join(format!(
            "sherd-family-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let src = dir.join("src");
        let Ok(()) = std::fs::create_dir_all(&src) else {
            unreachable!("a src dir is creatable")
        };
        let write = |name: &str, body: &str| {
            let Ok(()) = std::fs::write(src.join(name), body) else {
                unreachable!("a fixture file is writable")
            };
        };
        write("lib.rs", "mod acmd;\nmod bcmd;\nmod ccmd;\n");
        write("acmd.rs", "use crate::render;\nuse crate::units;\n");
        write("bcmd.rs", "use crate::render;\n");
        write("ccmd.rs", "use crate::render;\n");

        let found = structure(&dir);
        let family = found.iter().find(|p| p.members.len() == 3);
        let Some(family) = family else {
            unreachable!("three of a suffix is a family: {found:?}")
        };
        assert_eq!(family.evidence, Evidence::Cohesion);
        assert_eq!(family.shared, vec!["render"], "units is used by one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A hub is what EVERY member reaches for. `render` is shared; `units`
    /// is used by one member and is not.
    #[test]
    fn a_shared_hub_is_present_in_every_member() {
        let lists = vec![
            vec!["cli".to_string(), "render".to_string(), "units".to_string()],
            vec!["cli".to_string(), "render".to_string()],
        ];
        assert_eq!(shared_across(&lists), vec!["cli", "render"]);
        assert!(shared_across(&[]).is_empty());
    }

    /// Two of a suffix is a coincidence; three is a family. Without the
    /// floor, every `*s` plural in a crate becomes a proposed node.
    #[test]
    fn a_family_needs_more_than_a_pair() {
        let decl = |n: &str| crate::code::ModDecl {
            name: n.to_string(),
            is_pub: false,
        };
        let pair = [decl("acmd"), decl("bcmd")];
        assert!(families(&pair).is_empty());
        let three = [decl("acmd"), decl("bcmd"), decl("ccmd")];
        assert_eq!(families(&three).len(), 1);
    }
}

/// What one node's name costs the spec it is named in: rows and tokens.
///
/// Computed for a node whatever its state, which is the half `candidates`
/// could not do: that function lists modules to PROMOTE, so it excludes
/// directories that are already nodes -- and in a federated repository that
/// is every one of them, leaving the weight column reading zero for the only
/// tree where the question matters (`B14`).
///
/// The question here is the other one: given a node that EXISTS, how much of
/// its parent's spec is about it? Those rows are what a chain pays on every
/// descent and what moving them down would relieve.
#[must_use]
pub fn row_weight(spec: &str, name: &str) -> (usize, u64) {
    let rows = naming_rows(spec, name);
    let tokens = crate::tokens::count(&rows.join("\n")).tokens;
    (rows.len(), tokens)
}

#[cfg(test)]
mod weight_tests {
    use super::*;

    /// The defect `B14` names: this repository's root spec talks about `tdd`
    /// on dozens of rows, and the promotable-candidate path reported zero
    /// because `src/tdd` is already a node.
    #[test]
    fn an_existing_node_still_has_a_weight_in_its_parent() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
            unreachable!("this repository has a root spec")
        };
        let (rows, tokens) = row_weight(&spec, "tdd");
        assert!(rows > 0, "root names `tdd` on real rows");
        assert!(tokens > 0);

        // And a name nothing mentions weighs nothing.
        assert_eq!(row_weight(&spec, "wombat"), (0, 0));
    }

    /// `B15`, first cause, on the example that row names: lowercasing split
    /// the filename `SPEC.md` into `spec`, so every row naming the FILE
    /// counted as a row about the NODE. Measured on this repository's own
    /// root spec the column read 54 where a case-sensitive count reads 27.
    #[test]
    fn a_filename_is_not_a_row_about_the_node_it_spells() {
        assert!(!names_word("the node carries `SPEC.md`", "spec"));
        assert!(names_word("`spec` owns the section split", "spec"));

        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Ok(text) = std::fs::read_to_string(root.join("SPEC.md")) else {
            unreachable!("this repository has a root spec")
        };
        let counted = naming_rows(&text, "spec").len();
        let lowercased = text
            .lines()
            .filter(|l| !l.starts_with("## \u{a7}"))
            .filter(|l| {
                l.to_lowercase()
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .any(|w| w == "spec")
            })
            .count();
        assert!(
            lowercased > counted,
            "the lowercasing counted more than the case-sensitive read: \
             {lowercased} against {counted}"
        );
        for row in naming_rows(&text, "spec") {
            assert!(
                names_word(row.as_str(), "spec"),
                "a row was counted only for spelling the FILE: {row}"
            );
        }
    }

    /// `B15`, second cause: `sync` GENERATES `§N` into every node and `§N`
    /// names every sibling, so counting it makes this function read its own
    /// generator's output and the loop never converges (`V18`). `§F` is
    /// authored rather than generated and is excluded for the same reason --
    /// it is structure, not law.
    #[test]
    fn generated_navigation_is_not_law_about_a_sibling() {
        let spec = "## \u{a7}N NAV\n\nrel|path|lens\nsib|src/tdd|the loop\n\n\
                    ## \u{a7}F FEDERATION\n\ndir|lens|grade\ntdd|the loop|HT\n\n\
                    ## \u{a7}V INVARIANTS\n\nV1: `tdd` refuses a green test\n";
        assert_eq!(
            naming_rows(spec, "tdd"),
            vec!["V1: `tdd` refuses a green test".to_string()],
            "only the §V row is law about `tdd`"
        );
    }

    /// The same, on the tree that measured it: `src/tdd/SPEC.md` names five
    /// siblings and every one of those mentions is a generated `§N` row.
    #[test]
    fn a_leaf_spec_weighs_nothing_for_its_siblings() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let Ok(text) = std::fs::read_to_string(root.join("src/tdd/SPEC.md"))
        else {
            unreachable!("this repository has a spec for `src/tdd`")
        };
        assert!(
            text.contains("sib|src/lens"),
            "the nav does list `lens` as a sibling"
        );
        for row in naming_rows(&text, "lens") {
            assert!(
                !row.starts_with("sib|")
                    && !row.starts_with("up|")
                    && !row.starts_with("rel|"),
                "a nav row was weighed as law about a sibling: {row}"
            );
        }
    }
}
