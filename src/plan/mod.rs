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
/// Moved to [`crate::split`] (`src/plan:T14`) and re-exported here, because
/// every one of them was public at `0.5.1` and crates.io is immutable: a
/// caller of `sherd::plan::structure` must still compile. New code should
/// name `sherd::split`.
pub use crate::split::{
    Evidence, Proposal, Proposed, Ranked, propose, rank, row_weight, structure,
    uniform_evidence,
};
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
    ///
    /// The text form: [`Confidence::invalidators`] joined by ` · `. Kept as a
    /// `&str` because it is published API; a test pins the two together.
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

    /// [`Confidence::invalidated_by`], one entry per condition, so the JSON
    /// form carries a list rather than a string a caller must split on the
    /// text form's separator (V25).
    #[must_use]
    pub fn invalidators(self) -> &'static [&'static str] {
        match self {
            Confidence::Next => &[
                "judge rejects the test 3x",
                "gate still red after 3 repairs",
            ],
            Confidence::Likely => &[
                "step 1 adds a §B row to this node, changing its authoring prompt (tdd B9)",
            ],
            Confidence::Tentative => &[
                "any §T row added by steps above it; ordering has no declared `needs`",
            ],
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

/// Keep only the rows `milestone` claims in their OWN node's `§T` (V24).
///
/// Ids are node-scoped, so a milestone table speaks only for its own node:
/// each node is read through microlith's partition (`spec::milestones`), never
/// a second reading of the grammar. A suffixed id rides its base, as
/// `microlith/V14` orders it, so `T7a` is in whatever milestone claims 7.
///
/// Returns the kept rows and how many open rows sat in nodes that declare NO
/// milestones -- counted so the caller can say so, because a filter that
/// silently drops unscheduled work reads as "nothing left" (V3).
#[must_use]
pub fn in_milestone(
    root: &Path,
    rows: Vec<Task>,
    milestone: &str,
) -> (Vec<Task>, usize) {
    let mut parts: std::collections::BTreeMap<
        PathBuf,
        Vec<(String, Vec<u32>)>,
    > = std::collections::BTreeMap::new();
    let mut kept = Vec::new();
    let mut undeclared = 0usize;
    for t in rows {
        let node = parts.entry(t.node.clone()).or_insert_with(|| {
            std::fs::read_to_string(root.join(&t.node).join("SPEC.md"))
                .map(|s| crate::spec::milestones(&s))
                .unwrap_or_default()
        });
        if node.is_empty() {
            undeclared = undeclared.saturating_add(1);
            continue;
        }
        let claimed = task_number(&t.id).is_some_and(|n| {
            node.iter()
                .any(|(m, tasks)| m == milestone && tasks.contains(&n))
        });
        if claimed {
            kept.push(t);
        }
    }
    (kept, undeclared)
}

/// `T7` and `T7a` -> 7. A suffixed row rides its base (`microlith/V14`).
fn task_number(id: &str) -> Option<u32> {
    let digits: String = id
        .strip_prefix('T')?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Does any node declare this milestone? A name nobody declares is a typo,
/// and planning it would print an empty horizon that reads as "all done".
#[must_use]
pub fn milestone_declared(root: &Path, milestone: &str) -> bool {
    fed::discover(root).iter().any(|n| {
        std::fs::read_to_string(n.join("SPEC.md")).is_ok_and(|s| {
            crate::spec::milestones(&s)
                .iter()
                .any(|(m, _)| m == milestone)
        })
    })
}

/// Take the next [`HORIZON`] actionable rows and everything it cannot manage.
#[must_use]
pub fn plan(root: &Path) -> Plan {
    plan_in(root, None).0
}

/// [`plan`], optionally narrowed to one milestone (V24). The second value is
/// how many open rows the filter set aside because their node declares no
/// milestones -- 0 without a milestone.
#[must_use]
pub fn plan_in(root: &Path, milestone: Option<&str>) -> (Plan, usize) {
    let (all, outside) = match milestone {
        Some(m) => in_milestone(root, open_tasks(root), m),
        None => (open_tasks(root), 0),
    };
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
    (
        Plan {
            steps,
            unmanaged,
            total_open,
        },
        outside,
    )
}

/// `plan --format json`: the plumbing form of a [`plan_in`] result (V25).
///
/// `context_tokens[i]` is step `i`'s lens pack cost, measured by the caller
/// because the pack is `src/lens`'s. A missing entry reads as 0, as the text
/// form prints a pack that failed to build. Believability and the kept/tried
/// record come from `st`, the store the caller already holds. Keys are fixed
/// and ordered; the text layout may change, this may only grow.
#[must_use]
pub fn to_json(
    st: &crate::state::State,
    p: &Plan,
    (milestone, outside): (Option<&str>, usize),
    context_tokens: &[u64],
) -> String {
    let steps: Vec<String> = (1usize..)
        .zip(&p.steps)
        .map(|(rank, t)| {
            let c = Confidence::of(rank.saturating_sub(1));
            let (tried, kept) = record_in(st, &t.node);
            let why: Vec<String> =
                c.invalidators().iter().map(|w| json_str(w)).collect();
            format!(
                "{{\"rank\":{rank},\"kind\":{},\"node\":{},\"id\":{},\"text\":{},\
                 \"believability\":{},\"tried\":{tried},\"kept\":{kept},\
                 \"context_tokens\":{},\"invalidated_by\":[{}]}}",
                json_str(c.label().trim()),
                json_node(&t.node),
                json_str(&t.id),
                json_str(&t.text),
                believability_in(st, &t.node),
                context_tokens
                    .get(rank.saturating_sub(1))
                    .copied()
                    .unwrap_or(0),
                why.join(",")
            )
        })
        .collect();
    let unmanaged: Vec<String> = p
        .unmanaged
        .iter()
        .map(|(t, k)| {
            format!(
                "{{\"node\":{},\"id\":{},\"text\":{},\"reason\":{}}}",
                json_node(&t.node),
                json_str(&t.id),
                json_str(&t.text),
                json_str(k.why())
            )
        })
        .collect();
    format!(
        "{{\"horizon\":{},\"open\":{},\"unmanaged\":{},\"milestone\":{},\
         \"outside_milestones\":{outside},\"steps\":[{}],\"unmanaged_rows\":[{}]}}",
        p.steps.len(),
        p.total_open,
        p.unmanaged.len(),
        milestone.map_or_else(|| "null".to_string(), json_str),
        steps.join(","),
        unmanaged.join(",")
    )
}

/// A node as a caller names it. The root is `""` in memory and `.` in every
/// namespaced cite (`` `.:V83` ``), so the plumbing spells it `.` -- an empty
/// string is the one path a consumer cannot join or cite.
fn json_node(node: &Path) -> String {
    if node.as_os_str().is_empty() {
        json_str(".")
    } else {
        json_str(&node.to_string_lossy())
    }
}

/// A JSON string literal. `"`, `\` and every control char are escaped, which
/// is all RFC 8259 requires; everything else passes through as UTF-8.
fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len().saturating_add(2));
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() && u32::from(c) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", u32::from(c)));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
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
            // 2 spellings of 1 list: the text form ! stay the list, joined.
            assert_eq!(c.invalidated_by(), c.invalidators().join(" · "));
        }
    }

    fn row(node: &str, id: &str, text: &str) -> Task {
        Task {
            node: PathBuf::from(node),
            id: id.into(),
            status: '.',
            text: text.into(),
            cites: String::new(),
        }
    }

    /// V25: the JSON form is a CONTRACT a caller parses. Pinned whole, so a
    /// renamed key, a reordered field or a dropped row fails here rather than
    /// in the hallucinogen loop that reads it (issue #36).
    #[test]
    fn json_form_is_pinned() {
        let mut st = crate::state::State::default();
        record_outcome_in(&mut st, Path::new("src/b"), true);
        let p = Plan {
            steps: vec![row("src/a", "T1", "add `x`"), row("src/b", "T2", "y")],
            unmanaged: vec![(row("", "T9", "root"), Kind::NoModule)],
            total_open: 4,
        };
        assert_eq!(
            to_json(&st, &p, (Some("M1"), 1), &[120]),
            concat!(
                r#"{"horizon":2,"open":4,"unmanaged":1,"milestone":"M1","outside_milestones":1,"#,
                r#""steps":[{"rank":1,"kind":"NEXT","node":"src/a","id":"T1","text":"add `x`","#,
                r#""believability":0.5,"tried":0,"kept":0,"context_tokens":120,"#,
                r#""invalidated_by":["judge rejects the test 3x","gate still red after 3 repairs"]},"#,
                r#"{"rank":2,"kind":"LIKELY","node":"src/b","id":"T2","text":"y","#,
                r#""believability":0.6666666666666666,"tried":1,"kept":1,"context_tokens":0,"#,
                r#""invalidated_by":["step 1 adds a §B row to this node, changing its authoring prompt (tdd B9)"]}],"#,
                r#""unmanaged_rows":[{"node":".","id":"T9","text":"root","reason":"root row -- no mod.rs to add to"}]}"#,
            )
        );
    }

    /// An empty plan is still one object with every key: `null` milestone,
    /// empty lists. A caller reading `steps` ⊥ special-cases "nothing to do".
    #[test]
    fn json_of_an_empty_plan_keeps_every_key() {
        let p = Plan {
            steps: vec![],
            unmanaged: vec![],
            total_open: 0,
        };
        assert_eq!(
            to_json(&crate::state::State::default(), &p, (None, 0), &[]),
            r#"{"horizon":0,"open":0,"unmanaged":0,"milestone":null,"outside_milestones":0,"steps":[],"unmanaged_rows":[]}"#
        );
    }

    /// Row text is caveman prose from a `§T` cell: backslashes survive
    /// microlith's unescape (`src/fed:V4`), quotes are common, and a control
    /// char must ⊥ reach the output raw.
    #[test]
    fn json_str_escapes_what_rfc_8259_requires() {
        assert_eq!(
            json_str("a \"q\" C:\\p\n\t\u{1}|∴"),
            r#""a \"q\" C:\\p\n\t\u0001|∴""#
        );
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
/// Gated with `apply`, its only caller: ungated it is dead code in the
/// default build (`.:B26`).
#[cfg(feature = "ollama")]
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

    #[cfg(feature = "ollama")]
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

    #[cfg(feature = "ollama")]
    #[test]
    fn preflight_refuses_a_dirty_tree() {
        assert_eq!(check_dirty(), Ok(()));
    }

    /// A dirty tree means the generated diff would not be the only thing in
    /// the commit, which is the whole point of the branch `apply` makes.
    #[cfg(feature = "ollama")]
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

    const MS_ROOT: &str = "# SPEC\n\n## \u{a7}G GOAL\n\ntoy\n\n\
## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\na|alpha|beta|-\nb|beta|alpha|-\n";

    /// `a` declares milestones, `b` declares none; `T2` is done and `T2a`
    /// rides it, so M1 claims `T2a` through `T1-T2`.
    const MS_A: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nalpha\n\n## \u{a7}T TASKS\n\n\
| id | scope | tasks | done-when |\n|----|-------|-------|-----------|\n\
| M1 | first | T1-T2 | shipped |\n| M2 | later | T3 | shipped |\n\n\
id|status|task|cites\nT1|.|one|-\nT2|x|two|-\nT2a|.|two more|-\nT3|.|three|-\n";

    const MS_B: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nbeta\n\n## \u{a7}T TASKS\n\n\
id|status|task|cites\nT1|.|unscheduled|-\n";

    /// V24. A milestone keeps exactly the rows its OWN node's table claims,
    /// and the open rows of a node with no milestones are COUNTED, not
    /// silently dropped -- an empty horizon must never read as "all done".
    #[test]
    fn a_milestone_keeps_its_own_rows_and_counts_the_unscheduled()
    -> Result<(), String> {
        let r = TestRepo::new("plan-milestone")?;
        r.write("SPEC.md", MS_ROOT)?;
        r.write("a/SPEC.md", MS_A)?;
        r.write("b/SPEC.md", MS_B)?;
        let (kept, outside) =
            in_milestone(r.path(), open_tasks(r.path()), "M1");
        let ids: Vec<String> = kept
            .iter()
            .map(|t| format!("{}:{}", t.node.display(), t.id))
            .collect();
        assert_eq!(ids, vec!["a:T1", "a:T2a"]);
        assert_eq!(outside, 1, "b:T1 is counted, not dropped");
        assert!(milestone_declared(r.path(), "M2"));
        assert!(!milestone_declared(r.path(), "M9"), "a typo is not a plan");
        Ok(())
    }

    /// A suffixed id rides its base, as `microlith/V14` orders it.
    #[test]
    fn a_task_number_ignores_its_suffix() {
        assert_eq!(task_number("T7"), Some(7));
        assert_eq!(task_number("T7a"), Some(7));
        assert_eq!(task_number("V7"), None);
        assert_eq!(task_number("T"), None);
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

    #[cfg(feature = "ollama")]
    #[test]
    fn preflight_on_a_clean_repo_names_a_run_branch() {
        assert_eq!(check_clean(), Ok(()));
    }

    /// Generated code never lands on the trunk directly: `preflight` puts the
    /// run on its own branch, and `sherd land` is what moves it, on evidence.
    #[cfg(feature = "ollama")]
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
