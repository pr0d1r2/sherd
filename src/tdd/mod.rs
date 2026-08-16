//! TDD as separate LLM round-trips, each with a minimal declared profile.
//!
//! Monolithic TDD loads ~81% of a repo, because one call needs spec AND tests
//! AND implementation (root V87). Split into round-trips, no single call needs
//! all three -- measured ~1.5k tokens max on this repo against 157k.
//!
//!   1  RED test      spec + existing tests        (NOT the implementation)
//!   1b judge         invariant + the test         (NOT the implementation)
//!   2  GREEN         the test + implementation    (NOT the whole spec)
//!   3  gate          LOCAL, deterministic, ZERO tokens
//!   4  repair        test + impl + failure, capped
//!
//! Step 1b exists for the failure no other guard catches: one model writes a
//! test encoding its own misreading of an invariant, then implements to match,
//! and everything passes. The judge never sees the implementation.

use crate::{fed, ollama, spec};

/// The notation contract: what the symbols in an invariant MEAN.
///
/// Every prompt that must READ a caveman invariant gets this. A slice of
/// `FORMAT.md`, not the whole file -- V82's contract-not-implementation rule
/// applied to our own prompts (B6).
pub const NOTATION: &str = include_str!("notation.txt");
use std::path::Path;
use std::process::Command;

/// What one round-trip cost. Steps without their cost are not evidence.
#[derive(Debug)]
pub struct Step {
    pub label: &'static str,
    pub prompt_tokens: u64,
    pub eval_tokens: u64,
    pub ms: u128,
}

/// Calls a test makes that do not exist yet -- the contract step 2 must fill.
///
/// Deterministic parse, no model (`.:V18`). A run failed when the test called
/// `check_edge_depths(root, &edges)` and step 2 invented a different name, which
/// three repairs could not recover (B12): step 2 was never told what to define.
#[must_use]
pub fn expected_calls(test_src: &str, existing: &str) -> Vec<String> {
    const SKIP: [&str; 18] = [
        "fn",
        "if",
        "for",
        "while",
        "match",
        "let",
        "return",
        "assert",
        "assert_eq",
        "assert_ne",
        "panic",
        "println",
        "format",
        "vec",
        "write",
        "read",
        "Some",
        "Ok",
    ];
    let b = test_src.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !(b[i].is_ascii_alphabetic() || b[i] == b'_') {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
            i += 1
        }
        let name = &test_src[start..i];
        // a call is `name(`; a macro is `name!(`; a method is `.name(`
        if i >= b.len() || b[i] != b'(' {
            continue;
        }
        if start > 0 && (b[start - 1] == b'.' || b[start - 1] == b'!') {
            continue;
        }
        // `fn name(` is a DEFINITION, not a call -- including the test's own
        let mut k = start;
        while k > 0 && (b[k - 1] == b' ' || b[k - 1] == b'\t') {
            k -= 1
        }
        if k >= 2 && &test_src[k - 2..k] == "fn" {
            continue;
        }
        if SKIP.contains(&name) || existing.contains(&format!("fn {name}")) {
            continue;
        }
        // keep the call verbatim, arguments included -- the signature is the point
        let mut depth = 0usize;
        let mut j = i;
        while j < b.len() {
            if b[j] == b'(' {
                depth += 1
            } else if b[j] == b')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            j += 1;
        }
        let call =
            test_src[start..(j + 1).min(test_src.len())].replace('\n', " ");
        let call = call.split_whitespace().collect::<Vec<_>>().join(" ");
        if !out.contains(&call) {
            out.push(call)
        }
    }
    out
}
/// Moved to `crate::spec`, which owns `SPEC.md` structure. It lived here,
/// reachable only from the worker path, while `lens::pack` shipped whole
/// files to every context pack and every budget (`.:B8`).
pub use crate::spec::rule_depth;

/// Split a Rust source file at the `#[cfg(test)]` boundary.
///
/// One definition, because the code ceiling (root V50) needs exactly this
/// split -- code and tests counted separately -- and two readings of one rule
/// is the defect this project exists to end.
#[must_use]
pub fn split_module(src: &str) -> (&str, &str) {
    match src.find("\n#[cfg(test)]") {
        Some(i) => (&src[..i + 1], &src[i + 1..]),
        None => (src, ""),
    }
}

/// The public SURFACE of an implementation: signatures and type shapes, no
/// bodies. Step 1 needs this and must not have the bodies -- it is `§I`, not
/// `§V`. Written after a run where the test author, given only the spec, could
/// not see `Edge`'s fields and reached for the wrong one (B1 here).
#[must_use]
pub fn signatures(impl_src: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    let mut pending: Vec<&str> = Vec::new();
    for line in impl_src.lines() {
        let s = line.trim();
        // Doc comments ARE the semantics. Bare field names cannot tell a judge
        // whether `not_owns` holds a path or prose, and that is precisely the
        // question it has to answer (B4).
        if s.starts_with("///") {
            // Inside a type body a doc belongs to the FIELD below it, so emit
            // it in place; at top level it belongs to the item still to come.
            if depth > 0 {
                out.push_str(line);
                out.push('\n');
            } else {
                pending.push(line);
            }
            continue;
        }
        let is_sig = s.starts_with("pub fn")
            || s.starts_with("pub struct")
            || s.starts_with("pub enum")
            || s.starts_with("pub const");
        if depth > 0 {
            // inside a type body: keep field lines, they are part of the shape
            if s == "}" {
                depth = 0;
                out.push_str("}\n");
            } else if !s.is_empty() {
                out.push_str(line);
                out.push('\n');
            }
            continue;
        }
        if !is_sig {
            pending.clear();
        }
        if is_sig {
            for d in pending.drain(..) {
                out.push_str(d);
                out.push('\n');
            }
            if s.starts_with("pub fn") {
                let sig = s.split('{').next().unwrap_or(s).trim_end();
                out.push_str(sig);
                out.push_str(" { /* ... */ }\n");
            } else {
                out.push_str(line);
                out.push('\n');
                if s.ends_with('{') {
                    depth = 1;
                }
            }
        }
    }
    out
}

/// Append a test into the tests module. The ONLY function that writes there --
/// steps 2 and 4 structurally cannot touch the test, which is the guard
/// against an implementation that games it.
fn insert_test(src: &str, test_fn: &str) -> String {
    let idx = src.trim_end().rfind('}').unwrap_or(src.len());
    format!("{}\n{}\n{}", &src[..idx], test_fn.trim_end(), &src[idx..])
}

/// Append to the implementation region, above `#[cfg(test)]`.
fn insert_impl(src: &str, code: &str) -> String {
    let (impl_r, tests) = split_module(src);
    format!("{}\n{}\n\n{}", impl_r.trim_end(), code.trim(), tests)
}

/// Step 3. Local, deterministic, zero tokens. Reports what RAN, not only what
/// failed (root V48).
/// # Errors
/// The toolchain could not be RUN. That is not a red gate: a gate that did
/// not execute has said nothing, and returning `false` for it made a missing
/// `cargo` indistinguishable from a failing test. In `drive_from` that
/// mattered -- step 1 requires the gate to be RED, so an absent toolchain
/// read as "red as required" and the loop would have written code against a
/// gate that never ran. `.:V48` for a subprocess (B24, tdd B17 recurring).
pub fn gate(root: &Path) -> Result<(bool, String), String> {
    let cargo = std::env::var("BBX_CARGO").unwrap_or_else(|_| "cargo".into());
    // Same strictness as `.githooks/pre-commit`, deliberately: the loop's gate
    // and the commit's gate must be ONE rule. `-D warnings` in BOTH, because
    // `cargo build` does not compile `#[cfg(test)]` code and an unused import
    // in a test module shipped through a gate that never saw it (fed B8).
    let out = Command::new(&cargo)
        .args(["test", "--offline"])
        .env("RUSTFLAGS", "-D warnings")
        .current_dir(root)
        .output();
    let (tests_ok, mut report) = match out {
        Ok(o) => {
            let s = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            (
                o.status.success(),
                format!(
                    "=== cargo test: {} ===\n{}",
                    if o.status.success() { "PASS" } else { "FAIL" },
                    tail(&s, 2500)
                ),
            )
        }
        Err(e) => {
            return Err(format!(
                "the gate could not RUN: `{cargo}` -- {e}. set BBX_CARGO or enter the \
             dev shell. a gate that did not execute is not a gate that passed \
             or failed"
            ));
        }
    };
    // spec::check runs in-process -- no subprocess, no stdout scraping.
    let mut viol = 0;
    let nodes = fed::discover(root);
    for n in &nodes {
        if let Ok(t) = std::fs::read_to_string(n.join("SPEC.md")) {
            viol += spec::check(&t).len();
        }
    }
    report.push_str(&format!(
        "\n=== bbx check: {} === {} nodes examined, {viol} violations\n",
        if viol == 0 { "PASS" } else { "FAIL" },
        nodes.len()
    ));
    // Slice drift, by the same function `bbx slice --check` calls.
    let drift = crate::slice::drifted(root)?;
    report.push_str(&format!(
        "=== slice: {} === {} drifted\n",
        if drift.is_empty() { "PASS" } else { "FAIL" },
        drift.len()
    ));
    Ok((tests_ok && viol == 0 && drift.is_empty(), report))
}

fn tail(s: &str, n: usize) -> &str {
    if s.len() <= n { s } else { &s[s.len() - n..] }
}

fn run(
    prompt: &str,
    label: &'static str,
    log: &mut Vec<Step>,
) -> Result<String, String> {
    run_sampled(prompt, label, ollama::Sampling::DETERMINISTIC, log)
}

fn run_sampled(
    prompt: &str,
    label: &'static str,
    sampling: ollama::Sampling,
    log: &mut Vec<Step>,
) -> Result<String, String> {
    use std::io::Write;
    // Count locally too: the server's number and ours must agree, and a
    // silent divergence means the prompt is not what this code thinks it is.
    let local = crate::tokens::count(prompt);
    // Say what is being sent, and what it should COST, before sending it. A
    // silent 40-90s wait is indistinguishable from a hang (V21), and a
    // prediction is what lets the escalation guards mean anything.
    let eta = ollama::predict_for(label, local.tokens);
    eprintln!(
        "  [{label}] -> {} tok ({:.1} KB) - eta {:.0}s cold / {:.0}s if cached (~{} gen)",
        local.tokens,
        prompt.len() as f64 / 1024.0,
        eta.total_s(),
        eta.cached_s(),
        eta.gen_est
    );
    eprint!("       ");
    let _ = std::io::stderr().flush();
    if ollama::verbose() {
        eprintln!("\n--- prompt [{label}] ---\n{prompt}\n--- end prompt ---");
    }
    let mut n = 0usize;
    let r =
        ollama::generate_sampled(prompt, label, sampling, eta, &mut |chunk| {
            if ollama::verbose() {
                eprint!("{chunk}");
            } else {
                n += 1;
                // One dot per ~25 chunks: visible motion, not a firehose.
                if n.is_multiple_of(25) {
                    eprint!(".");
                }
            }
            let _ = std::io::stderr().flush();
        })?;
    eprintln!();
    ollama::observe_gen(label, r.eval_tokens);
    if ollama::verbose() {
        if !r.thinking.is_empty() {
            eprintln!(
                "\n--- reasoning [{label}] ---\n{}\n--- end reasoning ---",
                r.thinking
            );
        }
        eprintln!("--- end reply [{label}] ---");
    }
    // Prediction against telemetry -- the comparison is the point. A delta
    // that stays large means the pace model is wrong about THIS endpoint.
    let actual = r.ms as f64 / 1000.0;
    let basis = if ollama::last_cached() {
        eta.cached_s()
    } else {
        eta.total_s()
    };
    let delta = (actual - basis) / basis * 100.0;
    eprintln!(
        "  [{label}] <- {} sent · {} gen · {actual:.1}s (eta {:.0}s, {delta:+.0}%){}",
        r.prompt_tokens,
        r.eval_tokens,
        basis,
        if ollama::last_cached() {
            "  [prefix CACHED]"
        } else {
            ""
        }
    );
    if !r.thinking.is_empty() {
        eprintln!(
            "       (+{} reasoning tokens, hidden -- see -v){}",
            r.thinking.len() / 4,
            if ollama::last_load_ms() > 500 {
                format!(
                    "  [endpoint was COLD: {}ms model load]",
                    ollama::last_load_ms()
                )
            } else {
                String::new()
            }
        );
    }
    if r.prompt_tokens.abs_diff(local.tokens) > local.tokens / 10 {
        eprintln!(
            "  [{label}] note: local count {} vs server {} -- >10% apart",
            local.tokens, r.prompt_tokens
        );
    }
    log.push(Step {
        label,
        prompt_tokens: r.prompt_tokens,
        eval_tokens: r.eval_tokens,
        ms: r.ms,
    });
    Ok(r.text)
}

/// The MONOLITH arm of the premise gate (root V60): everything in one call.
///
/// Full spec including §B/§R, full implementation bodies, full tests, asked
/// for test AND implementation together. This is what blackbox claims to beat.
/// Same gate, same node, same invariant -- only the context shape differs.
///
/// # Errors
/// Returns the reason it could not proceed, same as [`drive`].
pub fn oneshot(
    root: &Path,
    node: &Path,
    invariant: &str,
    task: &str,
) -> Result<Vec<Step>, String> {
    let spec_path = node.join("SPEC.md");
    let mod_path = node.join("mod.rs");
    let spec_txt =
        std::fs::read_to_string(&spec_path).map_err(|e| e.to_string())?;
    let original =
        std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
    let (impl_r, tests_r) = split_module(&original);
    let inv = spec_txt
        .lines()
        .find(|l| l.starts_with(&format!("{invariant}:")))
        .ok_or_else(|| format!("{invariant} not declared"))?
        .to_string();
    let mut log = Vec::new();

    let reply = run(
        &format!(
            "{}\n--- spec (complete) ---\n{spec_txt}\n\n\
         --- implementation (complete) ---\n{impl_r}\n\n\
         --- existing tests ---\n{tests_r}\n\n\
         Prove and implement this invariant:\n  {inv}\n\nTask: {task}\n\n\
         Reply with TWO ```rust fenced blocks: first the new `#[test]` function, \
         then the new implementation function(s) to add. The test must fail against \
         the current implementation and pass against your new one.",
            NOTATION
        ),
        "monolith",
        &mut log,
    )?;

    let blocks: Vec<&str> = reply
        .split("```")
        .skip(1)
        .step_by(2)
        .map(|b| b.strip_prefix("rust").unwrap_or(b).trim())
        .collect();
    if blocks.len() < 2 {
        return Err(format!(
            "monolith returned {} code blocks, expected 2",
            blocks.len()
        ));
    }
    std::fs::write(
        &mod_path,
        insert_impl(&insert_test(&original, blocks[0]), blocks[1]),
    )
    .map_err(|e| e.to_string())?;
    let (ok, out) = gate(root)?;
    let sent: u64 = log.iter().map(|s| s.prompt_tokens).sum();
    eprintln!("\n  1 round-trip · {sent} tok sent · max single call {sent}");
    if ok {
        eprintln!("  VERDICT: MERGEABLE -- gates green");
        Ok(log)
    } else {
        eprintln!("{}", tail(&out, 1200));
        Err("NOT mergeable -- gates red".into())
    }
}

/// Drive one invariant from red to green.
///
/// # Errors
/// Returns the reason the loop could not proceed. A rejected test, a test that
/// is already green, or an exhausted repair budget are all reported -- never
/// silently swallowed.
pub fn drive(
    root: &Path,
    node: &Path,
    invariant: &str,
    task: &str,
    max_repair: usize,
) -> Result<Vec<Step>, String> {
    drive_from(root, node, node, invariant, task, max_repair)
}

/// As [`drive`], but the invariant is declared in `owner`, which may be an
/// ancestor. A moved row cites the root invariant it answers to, and that
/// invariant is not in the node's own spec (`.:plan` B6).
/// Restores a module unless the run earns the right to keep it.
///
/// Written after `bbx tdd` exhausted its repair budget and left code that did
/// not COMPILE in the tree (B22). Three other exit paths restored by hand and
/// that one did not -- and a non-compiling module fails every later command in
/// the repo, not just its own node.
///
/// A guard rather than a fourth hand-written restore, because "remember to
/// restore on every error path" is a prompt to my future self, and `?` can
/// return from paths nobody enumerated. Structure over gate over prompt, which
/// is the one encoding rule this repo has actually measured.
///
/// Ten lines rather than `scopeguard`: repair once reached for that crate and
/// this project does not take a dependency to own a `Drop` impl (fed B7).
struct Restore {
    path: std::path::PathBuf,
    original: String,
    armed: bool,
}

impl Restore {
    fn arm(path: &Path, original: &str) -> Self {
        Self {
            path: path.to_path_buf(),
            original: original.to_string(),
            armed: true,
        }
    }
    /// The run earned it. Nothing is restored when this guard drops.
    fn keep(&mut self) {
        self.armed = false;
    }
}

impl Drop for Restore {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::write(&self.path, &self.original);
        }
    }
}

/// One competing implementation, with the evidence about it.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub code: String,
    /// `cargo build` + `cargo test` + `bbx check`, all green.
    pub green: bool,
    /// Mechanical review findings against what this candidate added.
    pub findings: usize,
}

/// The winning candidate, or `None` when none of them earned it.
///
/// An idea meritocracy is not "rank them and take the top one". A red gate or
/// a review finding DISQUALIFIES: those are the two signals that caught real
/// stubs, so a candidate carrying one does not compete on the rest of its
/// merits. If every candidate is disqualified the answer is `None` -- keeping
/// the least-bad of a bad field is how a stub wins by default, and a wrong
/// function is worse than none because it reads as coverage.
///
/// Ties go to the lowest index, which is candidate 0, which is the
/// deterministic call. Merit has to be demonstrated to displace it.
#[must_use]
pub fn best(cands: &[Candidate]) -> Option<usize> {
    cands
        .iter()
        .enumerate()
        .filter(|(_, c)| c.green && c.findings == 0)
        .map(|(i, _)| i)
        .next()
}

/// What a field of candidates earned.
#[derive(Debug, PartialEq, Eq)]
pub enum Pick {
    /// One candidate is green and clean. Keep it.
    Merit(usize),
    /// Nobody finished -- every candidate is red. Not a bad field, an
    /// UNFINISHED one, so it goes to repair like a single attempt would.
    Unfinished(usize),
    /// Something compiled and passed, and still carries a finding. Revert.
    NoWinner,
}

/// Judge the field.
///
/// Measured (T13, N=3): all three candidates came back red with one finding
/// each, and the first cut of this rule disqualified all three and reverted --
/// so asking for MORE candidates removed repair entirely and made the loop
/// strictly worse than N=1. A red gate is not a verdict, it is an unfinished
/// attempt; repair is the step that exists to answer it.
///
/// Green AND carrying a finding is different, and stays fatal. That is the
/// stub signature -- `Vec::new()` compiles, passes, and reads as coverage --
/// and repair does not fix a stub, it polishes one.
#[must_use]
pub fn select(cands: &[Candidate]) -> Pick {
    if let Some(i) = best(cands) {
        return Pick::Merit(i);
    }
    if cands.is_empty() {
        return Pick::NoWinner;
    }
    if cands.iter().any(|c| c.green) {
        return Pick::NoWinner;
    }
    // All red. Candidate 0 is the deterministic call -- repair the same thing
    // a single-candidate run would have repaired, so N>1 can never do worse.
    Pick::Unfinished(0)
}

/// How many implementations compete at step 2. `BBX_CANDIDATES`, default 1.
///
/// Default 1 costs exactly what today costs: candidate 0 IS the deterministic
/// call, so the selection path runs on every step rather than lying dormant
/// until someone opts in.
#[must_use]
pub fn candidate_count() -> usize {
    std::env::var("BBX_CANDIDATES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 5)
}

/// The function a task NAMES, when it names one.
///
/// A backticked identifier followed by `(` -- so "`post_with_retry(&dyn
/// Transport, ...)`" yields `post_with_retry`, while a bare "`generate_via`"
/// mentioned in passing yields nothing. Naming a function with an argument
/// list is how a row says "write THIS"; naming one without is how it refers
/// to something that already exists.
#[must_use]
pub fn named_fn(task: &str) -> Option<String> {
    task.split('`').skip(1).step_by(2).find_map(|seg| {
        let name = seg.split('(').next()?.trim();
        if seg.contains('(')
            && !name.is_empty()
            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !name.starts_with(|c: char| c.is_ascii_digit())
        {
            Some(name.to_string())
        } else {
            None
        }
    })
}

/// A judge's verdict. YES on the first line, or it is not a yes.
///
/// One reading of one rule: both judges parse verdicts the same way, so a
/// change to what counts as assent cannot apply to one and not the other.
#[must_use]
pub fn is_yes(verdict: &str) -> bool {
    verdict
        .trim()
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_uppercase()
        .starts_with("YES")
}

/// The second judge: invariant plus implementation, and **never** the test.
///
/// Not a second opinion -- a second *lens*. Judge 1 holds the invariant and
/// the test and never sees the implementation. This one holds the invariant
/// and the implementation and never sees the test.
///
/// The asymmetry is the whole mechanism. Every stub in §B -- `Vec::new()`
/// under "satisfies the current test suite", `_budget` ignored,
/// `_generate_stub` faking a transport -- passed because the test and the
/// implementation agreed with each other. Agreement between two things is
/// invisible to a reviewer holding both and obvious to two reviewers each
/// holding one. Giving this judge the test would restore exactly the blind
/// spot it exists to cover.
#[must_use]
pub fn blind_prompt(inv: &str, added: &str) -> String {
    format!(
        "{NOTATION}\nInvariant:\n  {inv}\n\n\
         Proposed implementation:\n```rust\n{added}\n```\n\n\
         You are deliberately NOT shown the test. Judge the code against the \
         invariant alone. Answer NO if any of these hold: it returns a constant, \
         an empty collection or a default regardless of its input; it names a \
         parameter it never reads, with or without a leading underscore; it \
         reports a quantity other than the one the invariant is about; or its \
         own comments describe it as a stub, a placeholder, or as satisfying \
         tests. Answer YES only if code that violated the invariant would \
         differ from this. Answer YES or NO on the first line, then one sentence."
    )
}

/// The same judgement with the SCAFFOLDING removed.
///
/// [`blind_prompt`] enumerates the shapes a stub takes -- returns a constant,
/// names a parameter it never reads, reports the wrong quantity -- and every
/// stub in [`RECORDED`] matches one of those clauses almost verbatim. A model
/// scoring full marks there may be matching the checklist rather than reading
/// the code (`.:R38`). This asks the SAME question with none of the help.
///
/// What weakens between tiers is the scaffolding, never the criterion
/// (`.:V103`): a descent that also loosens what counts as correct measures
/// nothing but its own generosity.
#[must_use]
pub fn blind_prompt_bare(inv: &str, added: &str) -> String {
    format!(
        "{NOTATION}\nInvariant:\n  {inv}\n\n\
         Proposed implementation:\n```rust\n{added}\n```\n\n\
         You are deliberately NOT shown the test. Judge the code against the \
         invariant alone. Answer YES or NO on the first line, then one \
         sentence."
    )
}

/// One corpus item: an invariant, a proposed implementation, and whether that
/// implementation actually violates it.
pub struct JudgeItem {
    /// The invariant, as a worker would be given it.
    pub inv: &'static str,
    /// The code under judgement.
    pub code: &'static str,
    /// `true` when the correct verdict is NO.
    pub violates: bool,
}

/// One rung of the judge titration.
pub struct Tier {
    /// Reported name, used in the §R row this produces.
    pub name: &'static str,
    /// Whether the prompt enumerates the stub tells.
    pub tells: bool,
    /// The corpus this rung is judged against.
    pub items: &'static [JudgeItem],
}

impl Tier {
    /// The prompt this rung puts to the judge for one item.
    #[must_use]
    pub fn prompt(&self, it: &JudgeItem) -> String {
        if self.tells {
            blind_prompt(it.inv, it.code)
        } else {
            blind_prompt_bare(it.inv, it.code)
        }
    }
}

/// What one rung measured.
pub struct TierScore {
    /// The rung's name.
    pub name: &'static str,
    /// Items judged as expected, across BOTH arms.
    pub correct: usize,
    /// Items put to the judge.
    pub total: usize,
}

/// Run one rung and score it.
///
/// Scored over both arms together, deliberately: a judge that answers NO to
/// everything scores HALF here, where the stub arm alone would award it full
/// marks. That is the same control the two arm tests split between them
/// (`.:R37`), folded into one number.
///
/// A judge that could not RUN is an error, never a score of zero
/// (`src/tdd:V26`). A rung recorded 0/10 because the endpoint was unreachable
/// reads exactly like a located boundary, and that is the one reading this
/// harness must make impossible.
///
/// # Errors
/// The first judge failure, verbatim.
pub fn titrate_tier(
    tier: &Tier,
    judge: &mut dyn FnMut(&str) -> Result<bool, String>,
) -> Result<TierScore, String> {
    let mut correct = 0;
    for it in tier.items {
        let yes = judge(&tier.prompt(it))?;
        if yes != it.violates {
            correct += 1;
        }
    }
    Ok(TierScore {
        name: tier.name,
        correct,
        total: tier.items.len(),
    })
}

/// The recorded corpus: the five stubs from `§B`, verbatim, and five working
/// functions from this repo. Each carries the invariant it was written
/// against.
///
/// One definition, three readers -- both arm tests and the titration. Three
/// copies of one corpus is the duplication this repo keeps recording as its
/// founding defect.
pub const RECORDED: &[JudgeItem] = &[
    JudgeItem {
        inv: "V9: report every cycle in the federation graph",
        code: "pub fn detect_cycles(_edges: &[Edge]) -> Vec<Vec<String>> {\n    // stub -- satisfies the current test suite\n    Vec::new()\n}",
        violates: true,
    },
    JudgeItem {
        inv: "V8: a node over its ceiling gets a split hint",
        code: "pub fn check_split_hint(root: &Path, _budget: u64) -> Vec<PathBuf> {\n    vec![root.join(\"hint\")]\n}",
        violates: true,
    },
    JudgeItem {
        inv: "V10: report nodes whose declared tokens differ from measured",
        code: "pub fn find_token_mismatches(p: &Path) -> Vec<String> {\n    let n = std::fs::read_to_string(p).unwrap_or_default().len();\n    if n > 0 { vec![format!(\"{n}\")] } else { vec![] }\n}",
        violates: true,
    },
    JudgeItem {
        inv: "V4: retry is driven by the transport, not by a constant",
        code: "fn _generate_stub(_url: &str) -> Result<String, String> {\n    Ok(String::from(\"{\\\"response\\\":\\\"ok\\\"}\"))\n}",
        violates: true,
    },
    JudgeItem {
        inv: "V6: trim old rows of one kind from the state file",
        code: "fn count_kind(_lines: &[String], _kind: &str) -> usize { 0 }",
        violates: true,
    },
    JudgeItem {
        inv: "V46: a budget subtracts entry cost; a negative budget is `does not fit`, not a huge one",
        code: "pub const fn working(window: u64) -> u64 {\n    window.saturating_sub(ENTRY_COST)\n}",
        violates: false,
    },
    JudgeItem {
        inv: "V6: the ceiling for a path is the longest matching prefix, else the default",
        code: "pub fn for_path(&self, path: &str) -> u64 {\n    self.rows.iter()\n        .filter(|(p, _)| path.starts_with(p.as_str()))\n        .max_by_key(|(p, _)| p.len())\n        .map_or(self.default, |(_, v)| *v)\n}",
        violates: false,
    },
    JudgeItem {
        inv: "V14: prefill rate is a function of SIZE, so it is bucketed, not one scalar",
        code: "pub fn bucket(prompt_tokens: u64) -> &'static str {\n    match prompt_tokens {\n        0..=1_999 => \"b0\",\n        2_000..=7_999 => \"b2\",\n        8_000..=31_999 => \"b8\",\n        _ => \"b32\",\n    }\n}",
        violates: false,
    },
    JudgeItem {
        inv: "V4: a verdict states DIRECTION and DISTANCE, never a bare bool",
        code: "pub fn verdict(cost: u64, budget: u64) -> Verdict {\n    if cost <= budget {\n        Verdict::Fits { slack: budget - cost }\n    } else {\n        Verdict::Over { by: cost - budget }\n    }\n}",
        violates: false,
    },
    JudgeItem {
        inv: "V22: a judge's verdict is YES on the first line, or it is not a yes",
        code: "pub fn is_yes(verdict: &str) -> bool {\n    verdict.trim().lines().next().unwrap_or(\"\").trim().to_uppercase().starts_with(\"YES\")\n}",
        violates: false,
    },
];

/// The same ten implementations, with the invariant stated VAGUELY -- the
/// wording a hurried `§V` row actually gets, naming the subject but not the
/// property that decides the verdict.
///
/// The code is identical to [`RECORDED`] on purpose: whatever this rung loses
/// is attributable to the invariant's wording alone.
pub const VAGUE: &[JudgeItem] = &[
    JudgeItem {
        inv: "V9: handle cycles in the graph",
        code: RECORDED[0].code,
        violates: true,
    },
    JudgeItem {
        inv: "V8: deal with nodes that are too big",
        code: RECORDED[1].code,
        violates: true,
    },
    JudgeItem {
        inv: "V10: check the token counts",
        code: RECORDED[2].code,
        violates: true,
    },
    JudgeItem {
        inv: "V4: make the request robust",
        code: RECORDED[3].code,
        violates: true,
    },
    JudgeItem {
        inv: "V6: keep the state file bounded",
        code: RECORDED[4].code,
        violates: true,
    },
    JudgeItem {
        inv: "V46: compute the working budget",
        code: RECORDED[5].code,
        violates: false,
    },
    JudgeItem {
        inv: "V6: look up the ceiling for a path",
        code: RECORDED[6].code,
        violates: false,
    },
    JudgeItem {
        inv: "V14: classify a prompt by size",
        code: RECORDED[7].code,
        violates: false,
    },
    JudgeItem {
        inv: "V4: report whether it fits",
        code: RECORDED[8].code,
        violates: false,
    },
    JudgeItem {
        inv: "V22: read the judge's answer",
        code: RECORDED[9].code,
        violates: false,
    },
];

/// Stubs with the tells REMOVED: each reads its inputs, returns computed
/// data, and carries no comment confessing what it is. Every one still
/// violates its invariant -- by reporting the wrong quantity, covering one
/// case of several, or answering a neighbouring question.
///
/// This is the rung [`blind_prompt`]'s enumerated clauses cannot reach: none
/// of these returns a constant or names an unread parameter, so a checklist
/// match is no longer available and the code has to be read.
pub const SUBTLE: &[JudgeItem] = &[
    JudgeItem {
        inv: "V9: report every cycle in the federation graph",
        code: "pub fn detect_cycles(edges: &[Edge]) -> Vec<Vec<String>> {\n    edges.iter()\n        .filter(|e| e.from == e.to)\n        .map(|e| vec![e.from.clone()])\n        .collect()\n}",
        violates: true,
    },
    JudgeItem {
        inv: "V6: the ceiling for a path is the longest matching prefix, else the default",
        code: "pub fn for_path(&self, path: &str) -> u64 {\n    self.rows.iter()\n        .find(|(p, _)| path.starts_with(p.as_str()))\n        .map_or(self.default, |(_, v)| *v)\n}",
        violates: true,
    },
    JudgeItem {
        inv: "V46: a budget subtracts entry cost; a negative budget is `does not fit`, not a huge one",
        code: "pub fn working(window: u64) -> u64 {\n    (window as i64 - ENTRY_COST as i64) as u64\n}",
        violates: true,
    },
    JudgeItem {
        inv: "V4: a verdict states DIRECTION and DISTANCE, never a bare bool",
        code: "pub fn verdict(cost: u64, budget: u64) -> Verdict {\n    if cost <= budget {\n        Verdict::Fits { slack: budget - cost }\n    } else {\n        Verdict::Over { by: cost }\n    }\n}",
        violates: true,
    },
    JudgeItem {
        inv: "V9: report every cycle in the federation graph",
        code: "pub fn detect_cycles(edges: &[Edge]) -> Vec<Vec<String>> {\n    let mut out = Vec::new();\n    for start in edges.iter().map(|e| &e.from) {\n        let mut seen = vec![start.clone()];\n        let mut at = start;\n        while let Some(n) = edges.iter().find(|e| &e.from == at).map(|e| &e.to) {\n            if seen.contains(n) { out.push(seen.clone()); break; }\n            seen.push(n.clone());\n            at = n;\n        }\n    }\n    out\n}",
        violates: false,
    },
    JudgeItem {
        inv: "V6: the ceiling for a path is the longest matching prefix, else the default",
        code: "pub fn for_path(&self, path: &str) -> u64 {\n    self.rows.iter()\n        .filter(|(p, _)| path.starts_with(p.as_str()))\n        .max_by_key(|(p, _)| p.len())\n        .map_or(self.default, |(_, v)| *v)\n}",
        violates: false,
    },
    JudgeItem {
        inv: "V46: a budget subtracts entry cost; a negative budget is `does not fit`, not a huge one",
        code: "pub fn working(window: u64) -> Option<u64> {\n    window.checked_sub(ENTRY_COST)\n}",
        violates: false,
    },
    JudgeItem {
        inv: "V4: a verdict states DIRECTION and DISTANCE, never a bare bool",
        code: "pub fn verdict(cost: u64, budget: u64) -> Verdict {\n    if cost <= budget {\n        Verdict::Fits { slack: budget - cost }\n    } else {\n        Verdict::Over { by: cost - budget }\n    }\n}",
        violates: false,
    },
];

/// The rungs, easiest first. Only the scaffolding weakens between them.
pub const TIERS: &[Tier] = &[
    Tier {
        name: "0-tells",
        tells: true,
        items: RECORDED,
    },
    Tier {
        name: "1-bare",
        tells: false,
        items: RECORDED,
    },
    Tier {
        name: "2-vague",
        tells: false,
        items: VAGUE,
    },
    Tier {
        name: "3-subtle",
        tells: false,
        items: SUBTLE,
    },
];

///
/// # Errors
/// See [`drive`].
pub fn drive_from(
    root: &Path,
    node: &Path,
    owner: &Path,
    invariant: &str,
    task: &str,
    max_repair: usize,
) -> Result<Vec<Step>, String> {
    let spec_path = node.join("SPEC.md");
    let inv_path = owner.join("SPEC.md");
    let mod_path = node.join("mod.rs");
    let spec_txt = std::fs::read_to_string(&spec_path)
        .map_err(|e| format!("{}: {e}", spec_path.display()))?;
    let original = std::fs::read_to_string(&mod_path)
        .map_err(|e| format!("{}: {e}", mod_path.display()))?;
    let (impl_r, tests_r) = split_module(&original);
    // Armed from here on: every exit below this line restores unless the run
    // ends by earning `keep()`.
    let mut guard = Restore::arm(&mod_path, &original);

    let inv_txt =
        std::fs::read_to_string(&inv_path).unwrap_or_else(|_| spec_txt.clone());
    let inv = inv_txt.lines().find(|l| l.starts_with(&format!("{invariant}:")))
        .ok_or_else(|| format!("{invariant} not declared in {} -- a test for an invariant that does not exist encodes an unstated rule", inv_path.display()))?
        .to_string();
    eprintln!("node {} · {}\n", node.display(), inv.trim());

    let spec_rules = rule_depth(&spec_txt);
    let surface = signatures(impl_r);
    let mut log = Vec::new();

    // 1 -- RED test, with the judge's objection fed back on rejection. The
    // judge's reason is actionable signal; discarding it and hand-tuning the
    // prompt instead is what I did for six configurations before noticing (B10).
    let base = format!(
        "{NOTATION}\n--- spec (rules) ---\n{spec_rules}\n\n\
         --- public surface (signatures only) ---\n{surface}\n\n\
         --- existing tests in this module ---\n{tests_r}\n\n\
         Write ONE new Rust `#[test]` function proving this invariant:\n  {inv}\n\n\
         Task: {task}\n\n\
         It must FAIL against the current implementation, and fail at an assertion -- \
         not by failing to compile. It MUST include data that actually violates the \
         invariant, and assert that the violation is reported. Use only items that \
         already exist, plus the ONE new public function you expect to be written. \
         Reply with a single ```rust fenced block containing only the test function."
    );

    let mut test_fn = String::new();
    let mut accepted = false;
    let mut objection = String::new();
    for attempt in 0..3 {
        let prompt = if attempt == 0 {
            base.clone()
        } else {
            format!(
                "{base}\n\nYour previous attempt was REJECTED by review:\n\
                     ```rust\n{test_fn}\n```\nReason: {objection}\n\
                     Write a corrected test that answers that objection."
            )
        };
        let label: &'static str = if attempt == 0 {
            "1 red-test"
        } else {
            "1 red-retry"
        };
        test_fn = ollama::rust_block(&run(&prompt, label, &mut log)?);

        // Deterministic, before the judge, at zero tokens: if the row names
        // the function to write, the test has to CALL it. Measured -- the row
        // named `post_with_retry`, the test drove `generate_via` instead, the
        // judge said YES, and step 2 then had no new function to write so it
        // rewrote the old one. 10 round-trips, 25,991 tokens, 4 compile errors
        // including a redefinition (B23).
        if let Some(name) = named_fn(task)
            && !test_fn.contains(&format!("{name}("))
        {
            objection = format!(
                "the task names `{name}` and this test never calls it. \
                     Write a test that calls `{name}` directly."
            );
            eprintln!(
                "  contract: test does not call `{name}` -- rejected locally"
            );
            continue;
        }

        let verdict = run(
            &format!(
                "{NOTATION}\n--- data model ---\n{surface}\n\nInvariant:\n  {inv}\n\n\
             Proposed test:\n```rust\n{test_fn}\n```\n\n\
             Answer YES only if BOTH hold: (a) the test exercises the quantity the \
             invariant is actually about -- check the field names against the data model \
             above, a test asserting on the wrong field proves nothing; and (b) an \
             implementation violating the invariant would fail it. If the function \
             DETECTS something, the test MUST include input that should be detected \
             and assert it IS -- a test asserting only that nothing was found is \
             satisfied by a function that always finds nothing. Exhaustiveness is NOT \
             required. Answer YES or NO on the first line, then one sentence."
            ),
            if attempt == 0 {
                "1b judge"
            } else {
                "1b re-judge"
            },
            &mut log,
        )?;
        let first = verdict.trim().lines().next().unwrap_or("").to_string();
        eprintln!("  judge: {}", first.chars().take(78).collect::<String>());
        if is_yes(&verdict) {
            accepted = true;
            break;
        }
        objection = verdict.trim().to_string();
    }
    if !accepted {
        return Err(format!(
            "judge rejected the test 3 times -- last objection: {objection}"
        ));
    }

    std::fs::write(&mod_path, insert_test(&original, &test_fn))
        .map_err(|e| e.to_string())?;
    let (red_ok, red_out) = gate(root)?;
    if red_ok {
        return Err(
            "test passes already -- not a red test, nothing to drive".into()
        );
    }
    eprintln!("  gate: RED as required");

    // 2 -- GREEN. Sees the one test and the implementation, not the whole spec.
    let wanted = expected_calls(&test_fn, &surface);
    let contract = if wanted.is_empty() {
        String::new()
    } else {
        format!(
            "--- the test calls these; define EXACTLY these names and signatures ---\n{}\n\n",
            wanted.join("\n")
        )
    };
    eprintln!(
        "  contract: {}",
        if wanted.is_empty() {
            "(none detected)".into()
        } else {
            wanted.join(", ")
        }
    );
    let green_prompt = format!(
        "--- existing API (signatures; call these, do not reimplement) ---\n{surface}\n\n--- failing test ---\n```rust\n{test_fn}\n```\n\n\
         {contract}\
         --- failure ---\n{}\n\n\
         Write ONLY the new function(s) to ADD to the implementation so this test passes. \
         Do not restate existing code. \
         Do not modify the test. Reply with a single ```rust fenced block.",
        tail(&red_out, 1500)
    );

    // 3 -- the competition. N candidates, each judged on the same evidence,
    // best kept. At N=1 this is exactly the old single call: candidate 0 is
    // the deterministic one, so nothing here is dormant until opted into.
    let n = candidate_count();
    let mut cands: Vec<Candidate> = Vec::new();
    let mut results: Vec<(bool, String)> = Vec::new();
    let with_test =
        std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
    for k in 0..n {
        let label: &'static str =
            if k == 0 { "2 green" } else { "2 green-alt" };
        let code = ollama::rust_block(&run_sampled(
            &green_prompt,
            label,
            ollama::Sampling::candidate(k),
            &mut log,
        )?);
        std::fs::write(&mod_path, insert_impl(&with_test, &code))
            .map_err(|e| e.to_string())?;
        let (g, o) = gate(root)?;
        let added = crate::review::public_fns(&code);
        let cur =
            std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
        let (ci, ct) = split_module(&cur);
        let mut found = crate::review::unwired(ci, ct, &added);
        found.extend(crate::review::negative_only(ct, &added));
        found.extend(crate::review::ignored_input(&code, &added));
        if n > 1 {
            // Name them. Three candidates scoring "1 finding" told me nothing
            // about whether it was one shared defect or three different ones.
            eprintln!(
                "  candidate {k}: {} · {}",
                if g { "gate GREEN" } else { "gate red" },
                if found.is_empty() {
                    "clean".to_string()
                } else {
                    found
                        .iter()
                        .map(|f| f.rule.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            );
        }
        cands.push(Candidate {
            code,
            green: g,
            findings: found.len(),
        });
        results.push((g, o));
    }

    let pick = match select(&cands) {
        Pick::Merit(i) => {
            if n > 1 {
                eprintln!("  merit: candidate {i} of {n} kept");
            }
            i
        }
        Pick::Unfinished(i) => {
            if n > 1 {
                eprintln!("  merit: all {n} red -- repairing candidate {i}");
            }
            i
        }
        Pick::NoWinner => {
            return Err(format!(
                "{n} candidate(s) green but carrying findings -- reverted. repair \
                 polishes a stub, it does not fix one"
            ));
        }
    };
    std::fs::write(&mod_path, insert_impl(&with_test, &cands[pick].code))
        .map_err(|e| e.to_string())?;
    // Track exactly what we added, so repair REPLACES it rather than guessing
    // at a name prefix or rewriting the whole region (B5).
    let mut last_added = cands[pick].code.clone();

    let (mut ok, mut out) = (results[pick].0, results[pick].1.clone());
    // 4 -- repair, capped. On exhaustion, report what was tried.
    for i in 0..max_repair {
        if ok {
            break;
        }
        eprintln!("  gate: FAIL -- repair {}/{}", i + 1, max_repair);
        let cur =
            std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
        let (cur_impl, cur_tests) = split_module(&cur);
        let cur_surface = signatures(cur_impl);
        let label: &'static str =
            if i == 0 { "4 repair-1" } else { "4 repair-n" };
        let fixed = ollama::rust_block(&run(
            &format!(
                "--- existing API (signatures) ---\n{cur_surface}\n\n--- your current attempt ---\n{last_added}\n\n--- test ---\n```rust\n{test_fn}\n```\n\n\
             --- failure ---\n{}\n\n\
             Reply with ONLY the corrected version of the function(s) you previously \
             added, in one ```rust block. Do not restate unrelated code, do not remove \
             module documentation, and do not change the behaviour of functions that \
             already existed. Do not modify the test.",
                tail(&out, 2000)
            ),
            label,
            &mut log,
        )?);
        let replaced = if cur_impl.contains(last_added.trim()) {
            cur_impl.replace(last_added.trim(), fixed.trim())
        } else {
            // Could not find what we added -- refuse to guess. Appending would
            // duplicate the definition, rewriting would destroy unrelated code.
            return Err("repair lost track of the previous insertion -- refusing to                         guess where it went".into());
        };
        last_added = fixed.clone();
        std::fs::write(
            &mod_path,
            format!("{}\n\n{}", replaced.trim_end(), cur_tests),
        )
        .map_err(|e| e.to_string())?;
        let g = gate(root)?;
        ok = g.0;
        out = g.1;
    }

    // 5 -- the second lens. Only when the gate is green: a red gate has already
    // said no, and asking a judge to confirm it costs a call to learn nothing.
    if ok {
        let verdict =
            run(&blind_prompt(&inv, &last_added), "5 blind judge", &mut log)?;
        let first = verdict.trim().lines().next().unwrap_or("").to_string();
        eprintln!("  blind: {}", first.chars().take(78).collect::<String>());
        if !is_yes(&verdict) {
            return Err(format!(
                "gates green, second lens says NO -- reverted. objection: {}",
                verdict.trim()
            ));
        }
    }

    let sent: u64 = log.iter().map(|s| s.prompt_tokens).sum();
    let max = log.iter().map(|s| s.prompt_tokens).max().unwrap_or(0);
    eprintln!(
        "\n  {} round-trips · {sent} tok sent · max single call {max}",
        log.len()
    );
    if ok {
        eprintln!("  VERDICT: MERGEABLE -- gates green + second lens");
        guard.keep();
        Ok(log)
    } else {
        eprintln!("{}", tail(&out, 2000));
        Err(
            "NOT mergeable -- gates red after repair budget, module restored"
                .into(),
        )
    }
}
/// Classify a failure report as a compile‑time error.
///
/// The function returns `true` if the given report looks like a Rust compiler
/// error (e.g., starts with `"error"` or contains an error code such as
/// `"error[E0425]"`).  All other reports, including assertion failures,
/// are considered non‑compile errors and return `false`.
pub fn classify_failure(report: &str) -> bool {
    let s = report.trim_start();
    // Most compiler errors start with "error" or contain an error code in brackets.
    s.starts_with("error") || s.contains("error[")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_task_that_names_a_function_names_it_with_parens() {
        // The row that cost 25,991 tokens: it named the function to write and
        // the test drove a different one.
        assert_eq!(
            named_fn(
                "`post_with_retry(&dyn Transport, url, body)` -- a NEW fn"
            ),
            Some("post_with_retry".into())
        );
        // A function mentioned WITHOUT an argument list is a reference to
        // something that exists, not an instruction to write it. The same row
        // said "do not touch `generate_via`" and that must not become the
        // contract.
        assert_eq!(named_fn("wire `generate_via` to the retry path"), None);
        assert_eq!(named_fn("record per-request template overhead"), None);
        // First named wins, and prose in backticks is not a function.
        assert_eq!(
            named_fn("`§F` rows, then `depth(edges)`"),
            Some("depth".into())
        );
    }

    #[test]
    fn an_unkept_run_restores_the_module() {
        let dir = std::env::temp_dir().join("bbx-restore-test");
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("mod.rs");
        std::fs::write(&f, "fn original() {}\n").unwrap();

        // The exhausted-repair path: generated code written, run gives up.
        {
            let _g = Restore::arm(&f, "fn original() {}\n");
            std::fs::write(&f, "fn generated( {  // does not compile\n")
                .unwrap();
        }
        assert_eq!(
            std::fs::read_to_string(&f).unwrap(),
            "fn original() {}\n",
            "a run that keeps nothing must leave nothing behind"
        );

        // And a run that earns it keeps what it wrote.
        {
            let mut g = Restore::arm(&f, "fn original() {}\n");
            std::fs::write(&f, "fn kept() {}\n").unwrap();
            g.keep();
        }
        assert_eq!(std::fs::read_to_string(&f).unwrap(), "fn kept() {}\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn cand(green: bool, findings: usize) -> Candidate {
        Candidate {
            code: format!("fn c{findings}() {{}}"),
            green,
            findings,
        }
    }

    #[test]
    fn a_bad_field_has_no_winner() {
        // The failure this rule exists to stop: ranking always returns
        // something, so the least-bad stub wins by default. Disqualification
        // does not.
        assert_eq!(
            best(&[cand(false, 0), cand(true, 2), cand(false, 9)]),
            None
        );
        assert_eq!(best(&[]), None);
    }

    #[test]
    fn an_all_red_field_is_unfinished_not_bad() {
        // MEASURED, N=3: all three candidates came back red, the first rule
        // disqualified all three, and asking for more candidates therefore
        // removed repair. More competition made the loop strictly worse.
        assert_eq!(
            select(&[cand(false, 1), cand(false, 1), cand(false, 1)]),
            Pick::Unfinished(0),
            "repair is what answers a red gate"
        );
        // and it repairs candidate 0, so N>1 can never do worse than N=1
        assert_eq!(select(&[cand(false, 9)]), Pick::Unfinished(0));
    }

    #[test]
    fn green_with_a_finding_stays_fatal() {
        // The stub signature: compiles, passes, reads as coverage. Repair
        // polishes one, it does not fix one.
        assert_eq!(select(&[cand(true, 1), cand(false, 0)]), Pick::NoWinner);
        assert_eq!(select(&[cand(true, 0), cand(true, 1)]), Pick::Merit(0));
        assert_eq!(select(&[]), Pick::NoWinner);
    }

    #[test]
    fn merit_must_be_demonstrated_to_displace_the_deterministic_call() {
        // Candidate 0 is the temperature-0 call. A tie leaves it in place.
        assert_eq!(best(&[cand(true, 0), cand(true, 0)]), Some(0));
        // But it does not win by seniority: disqualified is disqualified.
        assert_eq!(best(&[cand(true, 1), cand(true, 0)]), Some(1));
        assert_eq!(best(&[cand(false, 0), cand(true, 0)]), Some(1));
    }

    #[test]
    fn candidate_count_is_bounded_at_both_ends() {
        // Not a correctness rule -- a cost rule. Each candidate is a
        // round-trip plus a full gate run.
        assert_eq!(candidate_count().clamp(1, 5), candidate_count());
    }

    #[test]
    fn the_second_lens_is_never_shown_the_test() {
        // The mechanism IS the blindness. If the test leaks into this prompt,
        // both judges see both sides and test-implementation collusion --
        // every stub in §B -- becomes invisible again.
        let p = blind_prompt(
            "V1: report every dir with no owner",
            "pub fn orphans(_d: &[Dir]) -> Vec<Dir> { Vec::new() }",
        );
        assert!(p.contains("orphans"), "must carry the implementation");
        assert!(
            p.contains("V1: report every dir"),
            "must carry the invariant"
        );
        assert!(
            !p.to_lowercase().contains("#[test]"),
            "the test must not reach the second lens: {p}"
        );
        assert!(!p.contains("assert"), "no test body may leak in: {p}");
    }

    /// V22 is a claim about the endpoint, so it is measured against the
    /// endpoint. `#[ignore]` because the gate stays offline; run with
    /// `cargo test -- --ignored --nocapture blind_lens`.
    ///
    /// Corpus: the stub half of [`RECORDED`] -- the five actually in `§B`,
    /// verbatim, each paired with the invariant it was written against. A NO
    /// on all five is the claim; anything less is the real number.
    #[test]
    #[ignore]
    fn blind_lens_vs_the_recorded_stubs() {
        let rejected = measure_arm(true);
        println!("blind lens rejected {rejected}/5 recorded stubs");
        assert!(
            rejected >= 4,
            "measured {rejected}/5 -- record the real number in §B, \
             do not weaken the corpus"
        );
    }

    /// The control half. A judge that answers NO to everything scores 5/5 on
    /// the stub corpus, which is exactly the vacuous pass the other arm
    /// exists to catch -- so the stub number means nothing without this one.
    ///
    /// Corpus: the working half of [`RECORDED`] -- real functions from this
    /// repo, each with the invariant it was actually written against.
    #[test]
    #[ignore]
    fn blind_lens_vs_working_code() {
        let accepted = measure_arm(false);
        println!("blind lens accepted {accepted}/5 working functions");
        assert!(
            accepted >= 4,
            "measured {accepted}/5 -- a lens that rejects working code is a \
             lens that rejects everything, and its 5/5 on the stub corpus \
             proves nothing"
        );
    }

    /// One arm of [`RECORDED`], scored against the endpoint.
    fn measure_arm(violates: bool) -> usize {
        RECORDED
            .iter()
            .filter(|it| it.violates == violates)
            .filter(|it| {
                let r = crate::ollama::generate(&blind_prompt(it.inv, it.code))
                    .expect("endpoint unreachable -- BBX_ENDPOINT");
                let yes = is_yes(&r.text);
                println!(
                    "{} {} tok · {}",
                    if yes { "ACCEPT" } else { "REJECT" },
                    r.prompt_tokens,
                    r.text.trim().lines().next().unwrap_or("")
                );
                yes != violates
            })
            .count()
    }

    /// THE TITRATION (`.:T74`). Rung 0 is the regression guard and asserts;
    /// rungs 1-3 exist to FAIL, so they report and assert nothing about the
    /// score. A test that demanded success at a rung built to break it would
    /// be flaky by construction, and the first red run would be answered by
    /// weakening the corpus -- which is the one move `.:V103` forbids.
    #[test]
    #[ignore]
    fn blind_lens_titration() {
        let mut judge =
            |p: &str| crate::ollama::generate(p).map(|r| is_yes(&r.text));
        for tier in TIERS {
            let s = titrate_tier(tier, &mut judge).expect(
                "endpoint unreachable -- a rung that did not run is \
                         an error, not a boundary (V26)",
            );
            let pct = s.correct * 100 / s.total;
            println!("tier {} · {}/{} ({pct}%)", s.name, s.correct, s.total);
            if tier.name == "0-tells" {
                assert!(
                    s.correct * 10 >= s.total * 8,
                    "rung 0 is the REGRESSION guard: {}/{} means the baseline \
                     moved, not that a boundary was found",
                    s.correct,
                    s.total
                );
            }
        }
    }

    #[test]
    fn the_bare_prompt_carries_no_tells() {
        let it = &RECORDED[0];
        let bare = blind_prompt_bare(it.inv, it.code);
        for tell in [
            "returns a constant",
            "never reads",
            "quantity other than",
            "placeholder",
        ] {
            assert!(
                !bare.contains(tell),
                "the bare rung must hand over no checklist, found `{tell}`"
            );
        }
        assert!(bare.contains(it.code), "must carry the implementation");
        assert!(bare.contains(it.inv), "must carry the invariant");
    }

    #[test]
    fn the_bare_prompt_never_shows_the_test() {
        let bare = blind_prompt_bare(
            "V1: report every dir with no owner",
            "pub fn orphans(_d: &[Dir]) -> Vec<Dir> { Vec::new() }",
        );
        assert!(
            !bare.to_lowercase().contains("#[test]"),
            "V22: the test must not reach the judge: {bare}"
        );
        assert!(!bare.contains("assert"), "no test body may leak in: {bare}");
    }

    #[test]
    fn every_rung_asks_the_same_question() {
        // V103: what weakens between rungs is the SCAFFOLDING. The judged
        // sentence, the invariant and the code must survive every rung, or
        // the descent is measuring its own generosity.
        for tier in TIERS {
            for it in tier.items {
                let p = tier.prompt(it);
                assert!(
                    p.contains("Judge the code against the invariant alone"),
                    "rung {} dropped the question",
                    tier.name
                );
                assert!(p.contains(it.code), "rung {} dropped code", tier.name);
                assert!(p.contains(it.inv), "rung {} dropped inv", tier.name);
            }
        }
    }

    #[test]
    fn a_rung_that_did_not_run_is_an_error_not_a_zero() {
        // V26. An unreachable endpoint scoring 0/10 reads exactly like a
        // located boundary, which is the reading that must be impossible.
        let mut dead = |_: &str| Err::<bool, String>("endpoint down".into());
        let r = titrate_tier(&TIERS[0], &mut dead);
        assert!(r.is_err(), "a judge that could not run is an ERROR");
    }

    #[test]
    fn a_no_to_everything_scores_half_not_all() {
        // The control, as arithmetic rather than as a second test: scoring
        // both arms together is what makes a vacuous judge visible.
        let mut always_no = |_: &str| Ok(false);
        let s = titrate_tier(&TIERS[0], &mut always_no).unwrap();
        assert_eq!(s.correct * 2, s.total, "NO to everything is half, not all");
    }
    #[test]
    fn a_verdict_is_yes_only_on_the_first_line() {
        assert!(is_yes("YES\nit reads its input"));
        assert!(is_yes("  yes -- fine  "));
        assert!(
            !is_yes("NO\nreturns YES for everything"),
            "a YES in the explanation is not assent"
        );
        assert!(!is_yes(""));
    }

    const SRC: &str = "pub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";

    #[test]
    fn expected_calls_finds_the_undefined_one_only() {
        let t = "#[test]\nfn x() {\n    let e = edges(\"a\");\n    let v = check_edge_depths(root, &e);\n    assert!(v.is_empty());\n    e.len();\n}";
        let existing = "pub fn edges(text: &str) -> Vec<Edge> { }";
        let c = expected_calls(t, existing);
        assert_eq!(c, vec!["check_edge_depths(root, &e)"], "got {c:?}");
    }

    #[test]
    fn expected_calls_skips_macros_and_methods() {
        let t = "assert_eq!(a, b); x.len(); vec![1];";
        assert!(
            expected_calls(t, "").is_empty(),
            "{:?}",
            expected_calls(t, "")
        );
    }

    #[test]
    fn a_worker_prompt_carries_no_supervisor_text() {
        // The supervisor command tells an agent to revert, halt, plant
        // anchors. A 20B asked to write one function must never see it (V13).
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let spec =
            std::fs::read_to_string(root.join("src/fed/SPEC.md")).unwrap();
        let src = std::fs::read_to_string(root.join("src/fed/mod.rs")).unwrap();
        let (impl_r, tests_r) = split_module(&src);
        let prompt = format!(
            "{NOTATION}{}{}{}",
            rule_depth(&spec),
            signatures(impl_r),
            tests_r
        );
        for marker in ["/sit", "git revert", "halt(sit)", "maintenance mode"] {
            assert!(
                !prompt.contains(marker),
                "supervisor instruction `{marker}` reached a worker prompt"
            );
        }
    }

    #[test]
    fn rule_depth_drops_the_archive_sections() {
        let s = "## \u{a7}G GOAL\ngoal\n\n## \u{a7}V INVARIANTS\nV1: a\n\n## \u{a7}B BUGS\nB1|x|cause|fix\n";
        let r = rule_depth(s);
        assert!(r.contains("V1: a"), "rules must survive: {r}");
        assert!(!r.contains("B1|"), "§B must be dropped: {r}");
        let s2 = "## \u{a7}T TASKS\nT1|.|do the thing|V1\n\n## \u{a7}B BUGS\nB1|x|c|f\n";
        assert!(
            rule_depth(s2).contains("T1|"),
            "§T is the plan and must survive"
        );
        assert!(!r.contains("BUGS"), "§B header must be dropped: {r}");
    }

    #[test]
    fn signatures_keep_shape_and_drop_bodies() {
        let src = "/// what it owns\npub struct E {\n    /// a path\n    pub dir: String,\n}\n\n/// does the thing\npub fn go(a: u8) -> bool {\n    secret();\n    true\n}\n";
        let s = signatures(src);
        assert!(
            s.contains("/// a path"),
            "doc comments ARE the semantics: {s}"
        );
        assert!(
            s.contains("/// does the thing"),
            "fn docs must survive: {s}"
        );
        assert!(
            s.contains("pub dir: String"),
            "field shape must survive: {s}"
        );
        assert!(
            s.contains("pub fn go(a: u8) -> bool"),
            "signature must survive: {s}"
        );
        assert!(!s.contains("secret()"), "body must NOT survive: {s}");
    }

    #[test]
    fn split_finds_the_test_boundary() {
        let (i, t) = split_module(SRC);
        assert!(i.contains("pub fn a"));
        assert!(!i.contains("cfg(test)"));
        assert!(t.starts_with("#[cfg(test)]"));
    }

    #[test]
    fn split_of_a_file_with_no_tests_is_all_impl() {
        let (i, t) = split_module("pub fn a() {}\n");
        assert_eq!(i, "pub fn a() {}\n");
        assert_eq!(t, "");
    }

    #[test]
    fn insert_impl_never_touches_the_test_region() {
        let out = insert_impl(SRC, "pub fn b() {}");
        let (_, t) = split_module(&out);
        assert_eq!(
            t,
            split_module(SRC).1,
            "test region must be byte-identical"
        );
        assert!(out.contains("pub fn b"));
    }

    #[test]
    fn insert_test_lands_inside_the_tests_module() {
        let out = insert_test(SRC, "    #[test]\n    fn u() {}");
        assert!(split_module(&out).1.contains("fn u()"));
    }

    #[test]
    fn classify_failure_works() {
        // A typical compiler error – represents a red test that fails to build.
        let compile_report =
            r#"error[E0425]: cannot find value `foo` in this scope"#;

        // An assertion failure message – represents a red test that runs but panics.
        let assert_report = "thread 'main' panicked at 'assertion failed: x == y', src/main.rs:10:5";

        // The new public function we expect to be written:
        //   pub fn classify_failure(report: &str) -> bool
        //
        // It should return true for compile‑time failures and false otherwise.
        assert!(
            classify_failure(compile_report),
            "Compile error should be classified as a compile failure"
        );

        // A red test that merely panics must NOT be treated as a compile failure.
        assert!(
            !classify_failure(assert_report),
            "Assertion failure should not be classified as a compile failure"
        );
    }
}
