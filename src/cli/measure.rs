//! `budget`, `lens`, `fed`, `route`, `debt` and `coverage` -- the verbs that MEASURE and report a number.

use super::*;

/// Working budget on the confirmed target tier: gpt-oss:20b at its full
/// 131,072 window, minus measured harness entry cost.
pub(super) const WINDOW: u64 = 131_072;

/// `--record`: bring `.lint-debt`'s numbers current, refusing a raise.
pub(super) fn record_debt(root: &Path, now: crate::debt::Measured) -> ExitCode {
    match crate::debt::record(root, now) {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sherd: {e}");
            ExitCode::from(1)
        }
    }
}

/// `sherd coverage [--check|--record]` -- the floor, as `hk` runs it.
///
/// The MIRROR of `debt`: this ratchet may only RISE. Same reason for
/// existing -- the rule was stated twice in `hk.pkl` and the recording path
/// was a hand edit, which is how a floor got lowered to match a drop today
/// (`src/debt:B7`).
pub(super) fn coverage_cmd(
    root: &Path,
    mode: Option<&str>,
    cargo: &str,
) -> ExitCode {
    use crate::debt::Build;
    // Every build that has a floor row is measured (`src/debt:V4`, `B8`).
    // `lines` is required. `lines-default` is optional: its absence is
    // REPORTED and never a failure (V12), so a `.coverage` written before
    // the row existed still checks.
    let mut code = 0u8;
    for build in Build::BOTH {
        let row = build.row();
        let Some(was) = crate::debt::recorded_floor_of(root, build) else {
            if build == Build::All {
                let path = root.join(".coverage");
                eprintln!("sherd: {}: no `lines` row to read", path.display());
                return ExitCode::from(2);
            }
            println!("  {row}: no floor recorded (none required)");
            continue;
        };
        let Some(now) = crate::debt::coverage_of(root, cargo, build) else {
            eprintln!(
                "sherd: could not read a coverage total for `{row}` -- that \
                 is an ERROR, not a floor breach (sherd/tdd:V26)"
            );
            return ExitCode::from(2);
        };
        let one = if mode == Some("--record") {
            record_coverage(root, build, now)
        } else {
            check_coverage(row, now, was)
        };
        code = code.max(one);
    }
    ExitCode::from(code)
}

/// `--record` for one build: raise its row, refusing a drop.
fn record_coverage(root: &Path, build: crate::debt::Build, now: usize) -> u8 {
    match crate::debt::record_coverage_of(root, build, now) {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("sherd: {e}");
            1
        }
    }
}

/// `--check` for one build, naming the row either way (`.:V48`).
fn check_coverage(row: &str, now: usize, was: usize) -> u8 {
    let (ok, report) = crate::debt::coverage_verdict(now, was);
    if ok {
        println!("  {row}: {}", report.trim_start());
        return 0;
    }
    eprintln!("{row}: {report}");
    1
}

/// `sherd debt [--check|--record]` -- the ratchet, as `hk` runs it.
///
/// The gate CALLS this instead of re-deriving the formula in awk. It was
/// stated three times before -- twice in `hk.pkl`, once in `src/land` -- and
/// the Rust copy diverged on all three of its inputs at once
/// (`src/debt:B6`). One caller cannot disagree with itself.
pub(super) fn debt_cmd(
    root: &Path,
    mode: Option<&str>,
    cargo: &str,
) -> ExitCode {
    let Some(now) = crate::debt::measure(root, cargo) else {
        eprintln!(
            "sherd: clippy did not COMPILE, so its count is not a \
             measurement -- a target that fails to build emits no warnings \
             at all (sherd/debt:V3)."
        );
        return ExitCode::from(2);
    };
    let Some(was) = crate::debt::recorded_ceilings(root) else {
        // ABSENT and MALFORMED are different mistakes and only one of them
        // is the reader's fault (`V12`, `B6` again one verb over).
        let registry = root.join(".lint-debt");
        if registry.is_file() {
            eprintln!(
                "sherd: {}: no `density` and `shape` rows to read",
                registry.display()
            );
        } else {
            eprintln!(
                "sherd: {}: no ratchet here. `sherd debt` needs a `.lint-debt` \
                 carrying `density` and `shape`.",
                registry.display()
            );
        }
        return ExitCode::from(2);
    };
    if mode == Some("--record") {
        return record_debt(root, now);
    }
    let (ok, report) = crate::debt::verdict(now, was);
    if ok {
        println!("{report}");
    } else {
        eprintln!("{report}");
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// One node's row in the `budget` table.
pub(super) struct BudgetRow {
    pub node: PathBuf,
    pub chain_tokens: u64,
    pub own_tokens: u64,
    pub chain_nodes: usize,
    pub ceiling: u64,
    pub over_by: Option<u64>,
}

/// What `budget` measured, before either form renders it.
///
/// Measurement STOPS at the first node it cannot read, as the text form
/// always has: `unmeasured` names that node, and `stderr` carries the line
/// the text form reports it with.
#[derive(Default)]
pub(super) struct Budget {
    pub rows: Vec<BudgetRow>,
    pub total: u64,
    pub over: usize,
    pub method: Option<&'static str>,
    pub unmeasured: Vec<(PathBuf, String)>,
    pub stderr: Option<String>,
    pub cold: bool,
}

impl Budget {
    /// The exit code, decided ONCE so the json `ok` and the process cannot
    /// disagree: 1 unmeasured · 2 no node matched · 1 over a WARM ceiling.
    pub(super) fn code(&self) -> u8 {
        if self.stderr.is_some() {
            1
        } else if self.rows.is_empty() {
            2
        } else if self.over > 0 && !self.cold {
            1
        } else {
            0
        }
    }
}

pub(super) fn measure_budget(root: &Path, dir: &Path) -> Budget {
    let mut b = Budget {
        cold: tokens::Ceilings::load(root).is_ok_and(|c| c.is_cold()),
        ..Budget::default()
    };
    for node in fed::discover(root) {
        // §I declares `sherd budget [dir]`. The argument was parsed by
        // `arg_dir` and then dropped, so every invocation reported the whole
        // repo -- an interface promised and unread, which is `.:V104`'s own
        // shape appearing in the command that enforces it.
        if !node.starts_with(dir) {
            continue;
        }
        let packed = lens::pack(root, &node, lens::Depth::Rule)
            .and_then(|p| Ok((lens::own_cost(&node, lens::Depth::Rule)?, p)));
        let (own, p) = match packed {
            Ok(both) => both,
            Err(e) => {
                b.stderr = Some(format!("sherd: {}: {e}", node.display()));
                b.unmeasured.push((node, e.to_string()));
                return b;
            }
        };
        // Re-read per node rather than hoisting the load: the key mapping
        // lives in `ceiling_for` and copying it here to save twelve reads of
        // a one-kilobyte file would be two readings of one rule.
        let ceiling = match lens::ceiling_for(root, &node) {
            Ok(c) => c,
            Err(e) => {
                b.stderr = Some(format!("sherd: {e}"));
                b.unmeasured.push((node, e));
                return b;
            }
        };
        b.total += p.cost.tokens;
        b.method = Some(p.cost.method);
        // A verdict states direction and distance (`src/lens:V4`): "over"
        // without "by how much" cannot tell a node that needs splitting from
        // one that drifted eleven tokens past.
        let over_by = match lens::verdict(p.cost.tokens, ceiling) {
            lens::Verdict::Fits { .. } => None,
            lens::Verdict::Over { by } => {
                b.over += 1;
                Some(by)
            }
        };
        b.rows.push(BudgetRow {
            node,
            chain_tokens: p.cost.tokens,
            own_tokens: own.tokens,
            chain_nodes: p.chain.len(),
            ceiling,
            over_by,
        });
    }
    b
}

/// The text form, exactly as `budget` printed it before it had a json one.
pub(super) fn budget_text(root: &Path, b: &Budget) -> String {
    let mut out = format!(
        "window {WINDOW} · entry {} · working {}\n\n",
        tokens::ENTRY_COST,
        tokens::working(WINDOW)
    );
    for r in &b.rows {
        let rel = r.node.strip_prefix(root).unwrap_or(&r.node);
        let name = if rel.as_os_str().is_empty() {
            Path::new(".")
        } else {
            rel
        };
        let mark = r
            .over_by
            .map_or(String::new(), |by| format!("  OVER by {by}"));
        out.push_str(&format!(
            "  {:<24} chain {:>6} tok  ({} nodes)  ceiling {:>6}{mark}\n",
            name.display(),
            r.chain_tokens,
            r.chain_nodes,
            r.ceiling
        ));
    }
    if b.stderr.is_some() {
        return out;
    }
    // V48: say what was examined, not only what failed.
    out.push_str(&format!(
        "\n  {} nodes examined · {} tok if all chains loaded · {} over ceiling\n",
        b.rows.len(),
        b.total,
        b.over
    ));
    // A COLD START is not a breach. With no `.context-limits` at all the
    // default is a suggestion nobody wrote, and failing a stranger against
    // it is a claim about a rule that does not exist (`B8`). An unlisted
    // PATH inside an existing file still takes the default -- `.:V6`.
    if b.cold && !b.rows.is_empty() {
        out.push_str(&format!(
            "  no .context-limits: ceilings are the {} tok default, and \
             over is advisory until you set them\n",
            tokens::DEFAULT_NODE
        ));
    }
    out
}

/// The text form, as dispatch called it before `--format` -- kept for the
/// tests that drive it, and compiled as one (see `repo_root`).
#[cfg(test)]
pub(super) fn budget(root: &Path, dir: PathBuf) -> ExitCode {
    budget_as(root, &dir, false)
}

pub(super) fn budget_as(root: &Path, dir: &Path, json: bool) -> ExitCode {
    let b = measure_budget(root, dir);
    if json {
        println!("{}", budget_json(root, &b));
    } else {
        print!("{}", budget_text(root, &b));
    }
    if let Some(e) = &b.stderr {
        eprintln!("{e}");
    } else if b.rows.is_empty() {
        // Examining NOTHING is not passing. A dir naming no node printed an
        // empty table and exited 0, which is indistinguishable from a clean
        // repo -- the same vacuous-pass shape as `src/tdd:V26`.
        eprintln!("{}", no_node(root, dir));
    }
    // T10/V104: the number exists to be COMPARED. Printing it and exiting 0
    // is what let the chains drift over unseen (B7).
    ExitCode::from(b.code())
}

pub(super) fn lens_cmd(
    root: &Path,
    dir: &Path,
    depth: lens::Depth,
) -> ExitCode {
    match lens::pack(root, dir, depth) {
        Ok(p) => {
            eprintln!("# chain: {} nodes · {}", p.chain.len(), p.cost);
            for c in &p.children {
                eprintln!(
                    "#   -> {:<16} {}  [not: {}]",
                    c.dir, c.owns, c.not_owns
                );
            }
            print!("{}", p.text);
            ExitCode::SUCCESS
        }
        // The miss `budget` reports, with the spelling that would have
        // worked (`V2`, `src/lens:B2`).
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("{}", no_node(root, dir));
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("sherd: {}: {e}", dir.display());
            ExitCode::from(2)
        }
    }
}

pub(super) fn fed_cmd(dir: &Path) -> ExitCode {
    let path = dir.join("SPEC.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("sherd: no SPEC.md at {}", dir.display());
        return ExitCode::from(2);
    };
    let edges = fed::edges(&text);
    for e in &edges {
        println!("{:<16} {:<40} not: {}", e.dir, e.owns, e.not_owns);
    }
    // `.:V48`: state what was EXAMINED. An unfederated spec printed NOTHING
    // and exited 0, which reads as "no problems" rather than "no §F table"
    // -- the shape `src/fed:B6` records, met on a stranger (`B6` here).
    if edges.is_empty() {
        println!(
            "{}: no §F table -- this node federates nothing. \
             `sherd split` proposes where the boundaries are.",
            dir.display()
        );
    }
    ExitCode::SUCCESS
}

/// `sherd route "<query>"` -- which node owns this question.
///
/// Exit codes carry the answer, because a script asking "where does this
/// belong" needs to tell a hit from a guess: `0` one node, `2` no node, `3`
/// several. Rounding an ambiguous query to its first match would make the
/// interesting case indistinguishable from the certain one.
pub(super) fn route_cmd(root: &Path, query: &str) -> ExitCode {
    match plan::route(root, query) {
        plan::Route::Hit(node, why) => {
            println!(
                "{}\n  matched: {}",
                node.strip_prefix(root).unwrap_or(&node).display(),
                why.join(", ")
            );
            ExitCode::SUCCESS
        }
        plan::Route::Miss => route_miss(root),
        plan::Route::Ambiguous(nodes) => route_ambiguous(root, &nodes),
    }
}

/// A miss is REPORTED, and points at the table that lists what exists.
///
/// An UNFEDERATED repository is a different answer wearing the same exit
/// code: with only a root node there is nothing `route` can ever return,
/// because the root owns everything and therefore answers nothing. Saying
/// "no node matched" there sends the asker off to rephrase a query that
/// could not have succeeded (`.:B19`, found on a foreign repository).
pub(super) fn route_miss(root: &Path) -> ExitCode {
    if fed::discover(root).len() <= 1 {
        println!(
            "this repository has one node, so there is nothing to route to. \
             `sherd init <dir>` starts a federation."
        );
    } else {
        println!(
            "no node matched. `sherd graph --table` lists what each one owns."
        );
    }
    ExitCode::from(2)
}

/// Several nodes tied. Listing them beats picking one: the asker can see
/// that their question spans a boundary, which is itself the answer.
pub(super) fn route_ambiguous(root: &Path, nodes: &[PathBuf]) -> ExitCode {
    println!("ambiguous -- the query spans {} nodes:", nodes.len());
    for n in nodes {
        println!("  {}", n.strip_prefix(root).unwrap_or(n).display());
    }
    ExitCode::from(3)
}

#[cfg(test)]
#[path = "tests/measure.rs"]
mod tests;
