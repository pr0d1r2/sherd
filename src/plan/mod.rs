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

mod shell;
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
        .filter(|p| fed::is_node(p))
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
    let scripts = shell::Scripts::read(root);
    let mut candidates = Vec::new();
    for mut t in all {
        // V26: a shell row is homed by its node and the scripts it cites,
        // never by `classify`, which asks a `mod.rs` question. A root row
        // MOVES to the node owning its scripts before anything else is read.
        let shell = shell::home(root, &t, &scripts);
        if let shell::Home::At(node) = &shell {
            t.node.clone_from(node);
        }
        let dir = root.join(&t.node);
        // A frozen node is unmanaged whatever the row says: the freeze is
        // root POLICY, not a property of the text, so no amount of reading
        // the row can reach it (B16).
        let k = match shell {
            _ if frozen.contains(&dir) => Kind::Frozen,
            shell::Home::At(_) => Kind::NodeFn,
            // Scripts in two nodes: its footprint crosses a node boundary.
            shell::Home::Span => Kind::MultiFile,
            shell::Home::NotShell => classify(&dir, &t.text),
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
    to_json_with(st, p, (milestone, outside), context_tokens, &[])
}

/// [`to_json`], with each step's [`footprint`] (V26). A shell step gains
/// `touches` -- its scripts, or `null` when the row cites none -- and its
/// `invalidated_by` comes from [`invalidators_of`]. A Rust step, or a step
/// with no entry in `touches`, is the object [`to_json`] always wrote.
#[must_use]
pub fn to_json_with(
    st: &crate::state::State,
    p: &Plan,
    (milestone, outside): (Option<&str>, usize),
    context_tokens: &[u64],
    touches: &[Option<Vec<String>>],
) -> String {
    let steps: Vec<String> = (1usize..)
        .zip(&p.steps)
        .map(|(rank, t)| {
            let c = Confidence::of(rank.saturating_sub(1));
            let (tried, kept) = record_in(st, &t.node);
            let foot = touches.get(rank.saturating_sub(1)).and_then(Option::as_deref);
            let why: Vec<String> = invalidators_of(c, t, foot)
                .iter()
                .map(|w| json_str(w))
                .collect();
            format!(
                "{{\"rank\":{rank},\"kind\":{},\"node\":{},\"id\":{},\"text\":{},\
                 \"believability\":{},\"tried\":{tried},\"kept\":{kept},\
                 \"context_tokens\":{},\"invalidated_by\":[{}]{}}}",
                json_str(c.label().trim()),
                json_node(&t.node),
                json_str(&t.id),
                json_str(&t.text),
                believability_in(st, &t.node),
                context_tokens
                    .get(rank.saturating_sub(1))
                    .copied()
                    .unwrap_or(0),
                why.join(","),
                foot.map_or_else(String::new, touches_key)
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

/// The `touches` key of a shell step: its scripts, or `null` when unknown.
fn touches_key(scripts: &[String]) -> String {
    if scripts.is_empty() {
        return ",\"touches\":null".into();
    }
    let list: Vec<String> = scripts.iter().map(|s| json_str(s)).collect();
    format!(",\"touches\":[{}]", list.join(","))
}

/// A step's FOOTPRINT (V26): `None` for a Rust step, which the tdd loop
/// bounds by its `mod.rs`; for a shell step the scripts it cites, relative
/// to the root, and an EMPTY list when it cites none -- unknown, never
/// "touches nothing".
#[must_use]
pub fn footprint(root: &Path, t: &Task) -> Option<Vec<String>> {
    if root.join(&t.node).join("mod.rs").is_file() {
        return None;
    }
    Some(shell::touches(root, t, &shell::Scripts::read(root)))
}

/// What would make step `t` wrong, one entry per condition (V1).
///
/// A Rust step (`touches` is `None`) gets [`Confidence::invalidators`]. A
/// shell step is not driven by the tdd loop, so the loop's conditions do not
/// apply: it is invalidated by a change to a script it touches or a row it
/// cites (V26), and a `Tentative` one still by the ordering. An unknown
/// footprint is stated as the whole node, the honest bound.
#[must_use]
pub fn invalidators_of(
    c: Confidence,
    t: &Task,
    touches: Option<&[String]>,
) -> Vec<String> {
    let Some(touches) = touches else {
        return c.invalidators().iter().map(|w| (*w).to_string()).collect();
    };
    let mut out: Vec<String> =
        touches.iter().map(|s| format!("a change to {s}")).collect();
    if touches.is_empty() {
        out.push(format!("a change to any script under {}", t.node.display()));
    }
    out.extend(
        shell::cited_rows(t)
            .iter()
            .map(|r| format!("a change to {r}")),
    );
    if c == Confidence::Tentative {
        out.extend(c.invalidators().iter().map(|w| (*w).to_string()));
    }
    out
}

/// A node as a caller names it. The root is `""` in memory and `.` in every
/// namespaced cite (`` `.:V83` ``), so the plumbing spells it `.` -- an empty
/// string is the one path a consumer cannot join or cite.
pub(crate) fn json_node(node: &Path) -> String {
    if node.as_os_str().is_empty() {
        json_str(".")
    } else {
        json_str(&node.to_string_lossy())
    }
}

/// A JSON string literal. `"`, `\` and every control char are escaped, which
/// is all RFC 8259 requires; everything else passes through as UTF-8.
pub(crate) fn json_str(s: &str) -> String {
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
#[path = "tests/plan.rs"]
mod tests;

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
    // V26: a shell step is PLANNED, not driven -- the tdd loop adds a
    // function to a `mod.rs`, and a shell node has none.
    let step = p
        .steps
        .iter()
        .find(|t| footprint(root, t).is_none())
        .ok_or("nothing actionable to apply -- shell steps are planned, not applied")?;
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
#[path = "tests/git.rs"]
mod git_tests;

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
#[path = "tests/route.rs"]
mod route_tests;
