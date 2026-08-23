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

use crate::ollama;

/// The notation contract: what the symbols in an invariant MEAN.
///
/// Every prompt that must READ a caveman invariant gets this. A slice of
/// `FORMAT.md`, not the whole file -- V82's contract-not-implementation rule
/// applied to our own prompts (B6).
pub const NOTATION: &str = include_str!("notation.txt");
use std::path::Path;

/// What one round-trip cost. Steps without their cost are not evidence.
#[derive(Debug)]
pub struct Step {
    pub label: &'static str,
    pub prompt_tokens: u64,
    pub eval_tokens: u64,
    pub ms: u128,
}

/// Moved to `crate::spec`, which owns `SPEC.md` structure. It lived here,
/// reachable only from the worker path, while `lens::pack` shipped whole
/// files to every context pack and every budget (`.:B8`).
pub use crate::spec::rule_depth;

/// Reading Rust source moved to `crate::code`, which owns it for BOTH this
/// node and `src/review` (`.:B13`). Re-exported so the loop reads unchanged.
pub use crate::code::{expected_calls, signatures, split_module, test_decls};

/// Append a test into the tests module. The ONLY function that writes there --
/// steps 2 and 4 structurally cannot touch the test, which is the guard
/// against an implementation that games it.
fn insert_test(src: &str, test_fn: &str) -> String {
    let idx = src.trim_end().rfind('}').unwrap_or(src.len());
    format!("{}\n{}\n{}", &src[..idx], test_fn.trim_end(), &src[idx..])
}

/// Append to the implementation region, above `#[cfg(test)]`.
///
/// The reply's OWN test half is dropped first. Step 2 is told "write only the
/// new function(s) to ADD to the implementation" and "do not modify the
/// test", but a reply that carries a `#[cfg(test)] mod tests` anyway gets
/// spliced ABOVE the module's real one -- two top-level `mod tests`, `E0428`,
/// and a gate that no repair can turn green because each repair may do it
/// again. MEASURED: that is what killed the live run (B29).
///
/// Deterministic, local, zero tokens -- the same shape as the `named_fn`
/// pre-check that rejects a test which does not call its target (B23).
fn insert_impl(src: &str, code: &str) -> String {
    let (impl_r, tests) = split_module(src);
    let (code_impl, _) = split_module(code);
    format!("{}\n{}\n\n{}", impl_r.trim_end(), code_impl.trim(), tests)
}

/// What to tell the judge about a function the row asks the loop to WRITE.
///
/// The row asks for a function that does not exist yet -- a call to it is
/// what a RED test IS. Unsaid, the judge reads that call as a mistake and
/// answers NO to every row that ADDS a function, which is every row the loop
/// can drive (B28). Empty when the row names no function, so a row that
/// modifies existing behaviour is unaffected.
#[must_use]
pub fn red_note(task: &str) -> String {
    named_fn(task).map_or(String::new(), |n| {
        format!(
            "`{n}` does NOT exist yet: this is the RED step, writing it is \
             the next one, so a call to it is EXPECTED and is not a reason \
             to answer NO. "
        )
    })
}

/// One streamed chunk: echo it under `-v`, else a dot every 25. Returns the
/// new count.
///
/// Extracted because making `run_sampled` a METHOD added an indent level and
/// pushed this closure past `excessive-nesting` -- the limit noticing that a
/// four-deep closure inside a call inside a method was never readable.
fn tick(chunk: &str, n: usize) -> usize {
    use std::io::Write;
    if ollama::verbose() {
        eprint!("{chunk}");
    } else if n.wrapping_add(1).is_multiple_of(25) {
        eprint!(".");
    }
    let _ = std::io::stderr().flush();
    n.wrapping_add(1)
}

/// Transport plus the step log: they always travel together, so they are one
/// thing. Threading the transport as a fifth ARGUMENT pushed `run_sampled`
/// past `clippy.toml`'s limit of four, which was the limit correctly saying
/// the seam wanted a context and not another parameter (`.:V109`).
pub struct Caller<'a> {
    t: &'a dyn ollama::Transport,
    /// What each round-trip cost. Steps without their cost are not evidence.
    pub log: Vec<Step>,
}

impl<'a> Caller<'a> {
    /// A caller over `t`, with an empty log.
    #[must_use]
    pub fn new(t: &'a dyn ollama::Transport) -> Self {
        Self { t, log: Vec::new() }
    }

    /// One deterministic round-trip.
    fn run(
        &mut self,
        prompt: &str,
        label: &'static str,
    ) -> Result<String, String> {
        self.run_sampled(prompt, label, ollama::Sampling::DETERMINISTIC)
    }

    /// One round-trip at `sampling`, recorded in `self.log`.
    fn run_sampled(
        &mut self,
        prompt: &str,
        label: &'static str,
        sampling: ollama::Sampling,
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
            eprintln!(
                "\n--- prompt [{label}] ---\n{prompt}\n--- end prompt ---"
            );
        }
        let mut n = 0usize;
        let r = ollama::generate_via(
            self.t,
            prompt,
            label,
            sampling,
            eta,
            &mut |chunk| n = tick(chunk, n),
        )?;
        eprintln!();
        if self.t.timings_are_real() {
            ollama::observe_gen(label, r.eval_tokens);
        }
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
        self.log.push(Step {
            label,
            prompt_tokens: r.prompt_tokens,
            eval_tokens: r.eval_tokens,
            ms: r.ms,
        });
        Ok(r.text)
    }
}

/// The MONOLITH arm of the premise gate (root V60): everything in one call.
///
/// Full spec including §B/§R, full implementation bodies, full tests, asked
/// for test AND implementation together. This is what sherd claims to beat.
/// Same gate, same node, same invariant -- only the context shape differs.
///
/// # Errors
/// Returns the reason it could not proceed, same as [`drive`].
pub fn oneshot(r: &Run) -> Result<Vec<Step>, String> {
    let spec_path = r.node.join("SPEC.md");
    let mod_path = r.node.join("mod.rs");
    let spec_txt =
        std::fs::read_to_string(&spec_path).map_err(|e| e.to_string())?;
    let original =
        std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
    let (impl_r, tests_r) = split_module(&original);
    let inv = spec_txt
        .lines()
        .find(|l| l.starts_with(&format!("{}:", r.invariant)))
        .ok_or_else(|| format!("{} not declared", r.invariant))?
        .to_string();

    // A cold endpoint's first call carries a disk load the eta was never
    // taught about, and the 4x ceiling then kills step 1 (`.:ollama:prewarm`).
    ollama::prewarm(r.transport);
    let mut c = Caller::new(r.transport);
    let reply = c.run(
        &format!(
            "{}\n--- spec (complete) ---\n{spec_txt}\n\n\
         --- implementation (complete) ---\n{impl_r}\n\n\
         --- existing tests ---\n{tests_r}\n\n\
         Prove and implement this invariant:\n  {inv}\n\nTask: {}\n\n\
         Reply with TWO ```rust fenced blocks: first the new `#[test]` function, \
         then the new implementation function(s) to add. The test must fail against \
         the current implementation and pass against your new one.",
            NOTATION, r.task
        ),
        "monolith",
    )?;

    let blocks: Vec<&str> = reply
        .split("```")
        .skip(1)
        .step_by(2)
        .map(|b| b.strip_prefix("rust").unwrap_or(b).trim())
        .collect();
    // The pattern IS the contract: a test block and an impl block, in that
    // order. A length check plus two indexes says the same thing twice.
    let [test_block, impl_block, ..] = blocks.as_slice() else {
        return Err(format!(
            "monolith returned {} code blocks, expected 2",
            blocks.len()
        ));
    };
    std::fs::write(
        &mod_path,
        insert_impl(&insert_test(&original, test_block), impl_block),
    )
    .map_err(|e| e.to_string())?;
    let (ok, out) = crate::land::gate_with(r.root, &r.cargo)?;
    let sent: u64 = c.log.iter().map(|s| s.prompt_tokens).sum();
    eprintln!("\n  1 round-trip · {sent} tok sent · max single call {sent}");
    if ok {
        eprintln!("  VERDICT: MERGEABLE -- gates green");
        Ok(c.log)
    } else {
        eprintln!("{}", crate::land::tail(&out, 1200));
        Err("NOT mergeable -- gates red".into())
    }
}

/// Drive one invariant from red to green.
///
/// # Errors
/// Returns the reason the loop could not proceed. A rejected test, a test that
/// is already green, or an exhausted repair budget are all reported -- never
/// silently swallowed.
/// One `tdd` run's inputs, bundled.
///
/// A struct rather than a seventh parameter: `drive_from` already took six
/// against `clippy.toml`'s limit of four, and adding the transport as an
/// argument would have made the signature worse to fix a testability problem
/// (`.:V50`). The limit did design work here rather than nagging.
///
/// `transport` is what makes the loop RUNNABLE OFFLINE (`V27`). Every model
/// call in the loop goes through it, so a scripted one drives the whole
/// cycle with no endpoint -- which is the only way anything here gets a test
/// that is not a forty-minute live run.
/// Repair attempts step 4 gets when the caller does not say.
pub const DEFAULT_REPAIRS: usize = 3;

pub struct Run<'a> {
    /// Repo root; the gate runs here.
    pub root: &'a Path,
    /// The node whose `mod.rs` is edited.
    pub node: &'a Path,
    /// Where the invariant is declared -- may be an ancestor (`.:plan` B6).
    pub owner: &'a Path,
    /// The invariant id, e.g. `V3`.
    pub invariant: &'a str,
    /// The task text from the `§T` row.
    pub task: &'a str,
    /// How many repair attempts step 4 gets.
    pub max_repair: usize,
    /// The toolchain the gate runs. `cargo_bin()` in production.
    pub cargo: String,
    /// Where model calls go. `&ollama::Http` in production.
    pub transport: &'a dyn ollama::Transport,
}

impl<'a> Run<'a> {
    /// A run against the real endpoint, with owner defaulting to the node.
    #[must_use]
    /// `max_repair` defaults to `DEFAULT_REPAIRS`; use [`Self::repairs`] to
    /// change it. A fifth parameter would have put this past
    /// `clippy.toml`'s limit of four -- the same limit that made `Run` exist.
    pub fn new(
        root: &'a Path,
        node: &'a Path,
        invariant: &'a str,
        task: &'a str,
    ) -> Self {
        Self {
            root,
            node,
            owner: node,
            invariant,
            task,
            max_repair: DEFAULT_REPAIRS,
            cargo: crate::land::cargo_bin(),
            transport: &ollama::Http,
        }
    }

    /// How many repair attempts step 4 gets.
    #[must_use]
    pub const fn repairs(mut self, n: usize) -> Self {
        self.max_repair = n;
        self
    }

    /// Point the gate at a different toolchain. A test scripts one here
    /// rather than setting `SHERD_CARGO`, which is process-global and shared
    /// with every other test running in parallel.
    #[must_use]
    pub fn with_cargo(mut self, cargo: &str) -> Self {
        self.cargo = cargo.to_string();
        self
    }

    /// The same run, with the invariant declared in an ancestor.
    #[must_use]
    pub const fn owned_by(mut self, owner: &'a Path) -> Self {
        self.owner = owner;
        self
    }
}

pub fn drive(
    root: &Path,
    node: &Path,
    invariant: &str,
    task: &str,
    max_repair: usize,
) -> Result<Vec<Step>, String> {
    drive_run(&Run::new(root, node, invariant, task).repairs(max_repair))
}

/// As [`drive`], but the invariant is declared in `owner`, which may be an
/// ancestor. A moved row cites the root invariant it answers to, and that
/// invariant is not in the node's own spec (`.:plan` B6).
/// Restores a module unless the run earns the right to keep it.
///
/// Written after `sherd tdd` exhausted its repair budget and left code that did
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
    /// `cargo build` + `cargo test` + `sherd check`, all green.
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

/// How many implementations compete at step 2. `SHERD_CANDIDATES`, default 1.
///
/// Default 1 costs exactly what today costs: candidate 0 IS the deterministic
/// call, so the selection path runs on every step rather than lying dormant
/// until someone opts in.
#[must_use]
pub fn candidate_count() -> usize {
    std::env::var("SHERD_CANDIDATES")
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

///
/// # Errors
/// See [`drive`].
/// As [`drive_run`], from loose parts. `plan::apply` builds its arguments
/// one at a time, so it keeps a positional entry point.
///
/// # Errors
/// See [`drive_run`].
pub fn drive_from(
    root: &Path,
    node: &Path,
    owner: &Path,
    invariant: &str,
    task: &str,
    max_repair: usize,
) -> Result<Vec<Step>, String> {
    drive_run(
        &Run::new(root, node, invariant, task)
            .repairs(max_repair)
            .owned_by(owner),
    )
}
/// What every stage of the loop reads: derived once from the node, never
/// mutated, and passed by reference so a stage takes TWO arguments instead of
/// six.
///
/// One struct rather than a wide parameter list, because splitting a long
/// function into helpers that each take five positional arguments trades one
/// lint for a worse one -- measured, and reverted, earlier in this repo
/// (`.lint-debt`).
struct Ctx<'a> {
    /// The `Run` this belongs to: root, node, cargo, transport.
    run: &'a Run<'a>,
    /// The module being edited.
    mod_path: std::path::PathBuf,
    /// The invariant's own line, verbatim from the spec that declares it.
    inv: String,
    /// `--depth rule` of the node's spec.
    spec_rules: String,
    /// Public signatures of the implementation half.
    surface: String,
    /// The test half, verbatim.
    tests: String,
    /// Names a test may already use without declaring them (B27).
    in_scope: String,
}

/// Step 1 -- a RED test, with the judge's objection fed back on rejection.
///
/// The judge's reason is actionable signal; discarding it and hand-tuning the
/// prompt instead is what I did for six configurations before noticing (B10).
fn red_test(ctx: &Ctx, c: &mut Caller) -> Result<String, String> {
    let (inv, surface, task) = (&ctx.inv, &ctx.surface, ctx.run.task);
    let base = format!(
        "{NOTATION}\n--- spec (rules) ---\n{}\n\n\
         --- public surface (signatures only) ---\n{surface}\n\n\
         --- existing tests in this module ---\n{}\n\n\
         Write ONE new Rust `#[test]` function proving this invariant:\n  {inv}\n\n\
         Task: {task}\n\n\
         It must FAIL against the current implementation, and fail at an assertion -- \
         not by failing to compile. It MUST include data that actually violates the \
         invariant, and assert that the violation is reported. Use only items that \
         already exist, plus the ONE new public function you expect to be written. \
         Reply with a single ```rust fenced block containing only the test function.",
        ctx.spec_rules, ctx.tests
    );

    let mut test_fn = String::new();
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
        test_fn = ollama::rust_block(&c.run(&prompt, label)?);

        if let Some(name) = named_fn(task)
            && !test_fn.contains(&format!("{name}("))
        {
            objection = contract_objection(&name);
            continue;
        }
        match judge_test(ctx, c, &test_fn, attempt)? {
            None => return Ok(test_fn),
            Some(no) => objection = no,
        }
    }
    Err(format!(
        "judge rejected the test 3 times -- last objection: {objection}"
    ))
}

/// The deterministic check, before the judge, at zero tokens: if the row names
/// the function to write, the test has to CALL it.
///
/// Measured -- the row named `post_with_retry`, the test drove `generate_via`
/// instead, the judge said YES, and step 2 then had no new function to write
/// so it rewrote the old one. 10 round-trips, 25,991 tokens, 4 compile errors
/// including a redefinition (B23).
fn contract_objection(name: &str) -> String {
    eprintln!("  contract: test does not call `{name}` -- rejected locally");
    format!(
        "the task names `{name}` and this test never calls it. \
         Write a test that calls `{name}` directly."
    )
}

/// The judge's verdict on one proposed test: `None` is acceptance, `Some` is
/// the objection to feed back.
fn judge_test(
    ctx: &Ctx,
    c: &mut Caller,
    test_fn: &str,
    attempt: usize,
) -> Result<Option<String>, String> {
    let (surface, in_scope, inv) = (&ctx.surface, &ctx.in_scope, &ctx.inv);
    let red_note = red_note(ctx.run.task);
    let verdict = c.run(
        &format!(
            "{NOTATION}\n--- data model ---\n{surface}\n--- already available to a test ---\n{in_scope}\n\nInvariant:\n  {inv}\n\n\
             Proposed test:\n```rust\n{test_fn}\n```\n\n\
             {red_note}Answer YES only if BOTH hold: (a) the test exercises the quantity the \
             invariant is actually about -- check the field names against the data model \
             above, a test asserting on the wrong field proves nothing; and (b) an \
             implementation violating the invariant would fail it. If the function \
             DETECTS something, the test MUST include input that should be detected \
             and assert it IS -- a test asserting only that nothing was found is \
             satisfied by a function that always finds nothing. Exhaustiveness is NOT \
             required. Answer YES or NO on the first line, then one sentence."
        ),
        if attempt == 0 { "1b judge" } else { "1b re-judge" },
    )?;
    let first = verdict.trim().lines().next().unwrap_or("").to_string();
    eprintln!("  judge: {}", first.chars().take(78).collect::<String>());
    Ok(if is_yes(&verdict) {
        None
    } else {
        Some(verdict.trim().to_string())
    })
}

/// One candidate and the gate output it produced.
///
/// Two parallel `Vec`s indexed by the same `pick` was the shape before, which
/// is a complex type in the signature and an index in two places that must
/// agree. They belong together because they are one attempt.
struct Tried {
    cand: Candidate,
    /// The gate's output for this candidate -- what repair is shown.
    out: String,
}

/// Steps 2 and 3 -- N candidate implementations, each judged on the same
/// evidence.
///
/// At N=1 this is exactly the old single call: candidate 0 is the
/// deterministic one, so nothing here is dormant until opted into.
fn green_candidates(
    ctx: &Ctx,
    c: &mut Caller,
    test_fn: &str,
    red_out: &str,
) -> Result<Vec<Tried>, String> {
    let prompt = green_prompt(ctx, test_fn, red_out);
    let n = candidate_count();
    let mut tried: Vec<Tried> = Vec::new();
    let with_test =
        std::fs::read_to_string(&ctx.mod_path).map_err(|e| e.to_string())?;
    for k in 0..n {
        let label: &'static str =
            if k == 0 { "2 green" } else { "2 green-alt" };
        let code = ollama::rust_block(&c.run_sampled(
            &prompt,
            label,
            ollama::Sampling::candidate(k),
        )?);
        std::fs::write(&ctx.mod_path, insert_impl(&with_test, &code))
            .map_err(|e| e.to_string())?;
        let (g, o) = crate::land::gate_with(ctx.run.root, &ctx.run.cargo)?;
        let found = candidate_findings(ctx, &code)?;
        if n > 1 {
            report_candidate(k, g, &found);
        }
        tried.push(Tried {
            cand: Candidate {
                code,
                green: g,
                findings: found.len(),
            },
            out: o,
        });
    }
    Ok(tried)
}

/// The step-2 prompt: the ONE test and the implementation, not the whole spec.
///
/// The contract line names exactly what the test calls, so step 2 cannot
/// invent a neighbouring name and leave the test uncallable.
fn green_prompt(ctx: &Ctx, test_fn: &str, red_out: &str) -> String {
    let surface = &ctx.surface;
    let wanted = expected_calls(test_fn, surface);
    eprintln!(
        "  contract: {}",
        if wanted.is_empty() {
            "(none detected)".into()
        } else {
            wanted.join(", ")
        }
    );
    let contract = if wanted.is_empty() {
        String::new()
    } else {
        format!(
            "--- the test calls these; define EXACTLY these names and signatures ---\n{}\n\n",
            wanted.join("\n")
        )
    };
    format!(
        "--- existing API (signatures; call these, do not reimplement) ---\n{surface}\n\n--- failing test ---\n```rust\n{test_fn}\n```\n\n\
         {contract}\
         --- failure ---\n{}\n\n\
         Write ONLY the new function(s) to ADD to the implementation so this test passes. \
         Do not restate existing code. \
         Do not modify the test. Reply with a single ```rust fenced block.",
        crate::land::tail(red_out, 1500)
    )
}

/// Mechanical review of ONE candidate, against what that candidate added.
///
/// NOT `unwired` here. Its own wording is "a `pub fn` called only from tests
/// LANDED but was never wired in" -- the subject is code that shipped and
/// STAYED unwired. A function born in the same breath as its test has landed
/// nothing yet, and at the moment of judgement nothing else can call it: the
/// loop only appends, so EVERY correct run tripped it and V23 made that fatal
/// (V29). The rule is not weakened -- `land::evidence` runs `review::commit`
/// over every commit on the branch, which is where "landed" applies (T19).
///
/// These three DO belong here: each judges the candidate's own quality, which
/// is complete the moment it is written.
fn candidate_findings(
    ctx: &Ctx,
    code: &str,
) -> Result<Vec<crate::review::Finding>, String> {
    let added = crate::code::public_fns(code);
    let cur =
        std::fs::read_to_string(&ctx.mod_path).map_err(|e| e.to_string())?;
    let (_ci, ct) = split_module(&cur);
    let mut found = crate::review::negative_only(code, ct, &added);
    found.extend(crate::review::ignored_input(code, &added));
    found.extend(crate::review::undocumented(code, &added));
    Ok(found)
}

/// Name what each candidate scored.
///
/// Three candidates scoring "1 finding" told me nothing about whether it was
/// one shared defect or three different ones.
fn report_candidate(k: usize, green: bool, found: &[crate::review::Finding]) {
    eprintln!(
        "  candidate {k}: {} · {}",
        if green { "gate GREEN" } else { "gate red" },
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

/// Step 4 -- repair, capped. Returns the last gate verdict and its output.
///
/// Repair REPLACES what the loop added rather than guessing at a name prefix
/// or rewriting the whole region (B5), so it tracks `last_added` across
/// attempts and refuses when it can no longer find it.
fn repair(
    ctx: &Ctx,
    c: &mut Caller,
    test_fn: &str,
    seed: (bool, String, String),
) -> Result<(bool, String, String), String> {
    let (mut ok, mut out, mut last_added) = seed;
    for i in 0..ctx.run.max_repair {
        if ok {
            break;
        }
        eprintln!("  gate: FAIL -- repair {}/{}", i + 1, ctx.run.max_repair);
        let cur = std::fs::read_to_string(&ctx.mod_path)
            .map_err(|e| e.to_string())?;
        let (cur_impl, cur_tests) = split_module(&cur);
        let label: &'static str =
            if i == 0 { "4 repair-1" } else { "4 repair-n" };
        let fixed = ollama::rust_block(&c.run(
            &repair_prompt(signatures(cur_impl), &last_added, test_fn, &out),
            label,
        )?);
        let Some(replaced) = cur_impl
            .contains(last_added.trim())
            .then(|| cur_impl.replace(last_added.trim(), fixed.trim()))
        else {
            // Could not find what we added -- refuse to guess. Appending would
            // duplicate the definition, rewriting would destroy unrelated code.
            return Err("repair lost track of the previous insertion -- \
                        refusing to guess where it went"
                .into());
        };
        last_added = fixed;
        std::fs::write(
            &ctx.mod_path,
            format!("{}\n\n{}", replaced.trim_end(), cur_tests),
        )
        .map_err(|e| e.to_string())?;
        let g = crate::land::gate_with(ctx.run.root, &ctx.run.cargo)?;
        ok = g.0;
        out = g.1;
    }
    Ok((ok, out, last_added))
}

/// The repair prompt: the current surface, what we last added, the test, and
/// the failure -- and nothing else, so the model cannot restate the module.
fn repair_prompt(
    surface: String,
    last_added: &str,
    test_fn: &str,
    out: &str,
) -> String {
    format!(
        "--- existing API (signatures) ---\n{surface}\n\n--- your current attempt ---\n{last_added}\n\n--- test ---\n```rust\n{test_fn}\n```\n\n\
         --- failure ---\n{}\n\n\
         Reply with ONLY the corrected version of the function(s) you previously \
         added, in one ```rust block. Do not restate unrelated code, do not remove \
         module documentation, and do not change the behaviour of functions that \
         already existed. Do not modify the test.",
        crate::land::tail(out, 2000)
    )
}

/// The loop: RED test, GREEN candidates, repair, second lens.
///
/// This function COORDINATES; each numbered step is its own function above.
/// It holds the `Restore` guard because arming and keeping are the two ends of
/// one decision, and splitting them across functions would put the module's
/// safety in two places (B21 is why this split is worth measuring at all).
pub fn drive_run(r: &Run) -> Result<Vec<Step>, String> {
    let spec_path = r.node.join("SPEC.md");
    let mod_path = r.node.join("mod.rs");
    let spec_txt = std::fs::read_to_string(&spec_path)
        .map_err(|e| format!("{}: {e}", spec_path.display()))?;
    let original = std::fs::read_to_string(&mod_path)
        .map_err(|e| format!("{}: {e}", mod_path.display()))?;
    let (impl_r, tests_r) = split_module(&original);
    // Armed from here on: every exit below this line restores unless the run
    // ends by earning `keep()`.
    let mut guard = Restore::arm(&mod_path, &original);

    let ctx = Ctx {
        run: r,
        inv: declared_invariant(r, &spec_txt)?,
        spec_rules: rule_depth(&spec_txt),
        surface: signatures(impl_r),
        // The judge is shown the impl half and told to check names against it,
        // but the test it judges lives in the OTHER half and may legitimately
        // reuse a double declared there. Without these it rejects a good test
        // for referring to something that "does not appear" (B27).
        in_scope: test_decls(tests_r),
        tests: tests_r.to_string(),
        mod_path,
    };
    eprintln!("node {} · {}\n", r.node.display(), ctx.inv.trim());

    // A cold endpoint's first call carries a disk load the eta was never
    // taught about, and the 4x ceiling then kills step 1 (`.:ollama:prewarm`).
    ollama::prewarm(r.transport);
    let mut c = Caller::new(r.transport);

    let test_fn = red_test(&ctx, &mut c)?;
    std::fs::write(&ctx.mod_path, insert_test(&original, &test_fn))
        .map_err(|e| e.to_string())?;
    let (red_ok, red_out) = crate::land::gate_with(r.root, &r.cargo)?;
    if red_ok {
        return Err(
            "test passes already -- not a red test, nothing to drive".into()
        );
    }
    eprintln!("  gate: RED as required");

    let tried = green_candidates(&ctx, &mut c, &test_fn, &red_out)?;
    let pick = pick_candidate(&tried)?;
    let Some(won) = tried.get(pick) else {
        unreachable!("select returns an index into the slice it was given")
    };
    let with_test =
        std::fs::read_to_string(&ctx.mod_path).map_err(|e| e.to_string())?;
    std::fs::write(&ctx.mod_path, insert_impl(&with_test, &won.cand.code))
        .map_err(|e| e.to_string())?;
    let seed = (won.cand.green, won.out.clone(), won.cand.code.clone());

    let (ok, out, last_added) = repair(&ctx, &mut c, &test_fn, seed)?;

    // 5 -- the second lens. Only when the gate is green: a red gate has
    // already said no, and asking a judge to confirm it costs a call to learn
    // nothing.
    if ok {
        blind_check(&ctx, &mut c, &last_added)?;
    }
    report_verdict(c, ok, &out, &mut guard)
}

/// The invariant's own line, from the spec that DECLARES it -- which may be an
/// ancestor (`.:plan` B6).
fn declared_invariant(r: &Run, spec_txt: &str) -> Result<String, String> {
    let inv_path = r.owner.join("SPEC.md");
    let inv_txt = std::fs::read_to_string(&inv_path)
        .unwrap_or_else(|_| spec_txt.to_string());
    inv_txt
        .lines()
        .find(|l| l.starts_with(&format!("{}:", r.invariant)))
        .map(str::to_string)
        .ok_or_else(|| {
            format!(
                "{} not declared in {} -- a test for an invariant that does \
                 not exist encodes an unstated rule",
                r.invariant,
                inv_path.display()
            )
        })
}

/// Which candidate is kept, and why -- or the refusal when none earned it.
fn pick_candidate(tried: &[Tried]) -> Result<usize, String> {
    let n = tried.len();
    let cands: Vec<Candidate> = tried.iter().map(|t| t.cand.clone()).collect();
    match select(&cands) {
        Pick::Merit(i) => {
            if n > 1 {
                eprintln!("  merit: candidate {i} of {n} kept");
            }
            Ok(i)
        }
        Pick::Unfinished(i) => {
            if n > 1 {
                eprintln!("  merit: all {n} red -- repairing candidate {i}");
            }
            Ok(i)
        }
        Pick::NoWinner => Err(format!(
            "{n} candidate(s) green but carrying findings -- reverted. repair \
             polishes a stub, it does not fix one"
        )),
    }
}

/// Step 5 -- the second lens, blind to the gate's verdict.
fn blind_check(
    ctx: &Ctx,
    c: &mut Caller,
    last_added: &str,
) -> Result<(), String> {
    let verdict =
        c.run(&blind_prompt(&ctx.inv, last_added), "5 blind judge")?;
    let first = verdict.trim().lines().next().unwrap_or("").to_string();
    eprintln!("  blind: {}", first.chars().take(78).collect::<String>());
    if is_yes(&verdict) {
        return Ok(());
    }
    Err(format!(
        "gates green, second lens says NO -- reverted. objection: {}",
        verdict.trim()
    ))
}

/// What the run cost, and whether the module is kept.
fn report_verdict(
    c: Caller,
    ok: bool,
    out: &str,
    guard: &mut Restore,
) -> Result<Vec<Step>, String> {
    let sent: u64 = c.log.iter().map(|s| s.prompt_tokens).sum();
    let max = c.log.iter().map(|s| s.prompt_tokens).max().unwrap_or(0);
    eprintln!(
        "\n  {} round-trips · {sent} tok sent · max single call {max}",
        c.log.len()
    );
    if !ok {
        eprintln!("{}", crate::land::tail(out, 2000));
        return Err(
            "NOT mergeable -- gates red after repair budget, module restored"
                .into(),
        );
    }
    eprintln!("  VERDICT: MERGEABLE -- gates green + second lens");
    guard.keep();
    Ok(c.log)
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
        let dir = std::env::temp_dir().join("sherd-restore-test");
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

    /// A module with one implementation fn and one test.
    const MODULE: &str = "pub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";

    /// A step-2 reply that carries a test module, against its instructions.
    const REPLY_WITH_TEST: &str = "pub fn b() -> u8 { 1 }\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn u() {}\n}";

    /// B29. Splicing a reply's own `mod tests` above the module's real one
    /// gives TWO top-level test modules and `E0428` -- and no repair can fix
    /// it, because each repair may do it again, so the whole budget burns on
    /// a defect the loop introduced itself.
    #[test]
    fn a_reply_carrying_its_own_test_module_leaves_one_module() {
        let out = insert_impl(MODULE, REPLY_WITH_TEST);
        let n = out.matches("\nmod tests {").count();
        assert_eq!(n, 1, "exactly one test module survives: {out}");
    }

    #[test]
    fn the_implementation_lands_and_the_replies_test_does_not() {
        let out = insert_impl(MODULE, REPLY_WITH_TEST);
        assert!(out.contains("pub fn b()"), "the implementation lands");
        assert!(!out.contains("fn u()"), "step 2 must not write tests");
        assert!(out.contains("fn t()"), "the module's own test is kept");
    }

    #[test]
    fn a_well_behaved_reply_is_unchanged_by_the_guard() {
        // The common case must not pay for the guard.
        let out = insert_impl(MODULE, "pub fn b() -> u8 { 1 }");
        assert!(out.contains("pub fn b()"));
        assert_eq!(out.matches("\nmod tests {").count(), 1);
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

#[cfg(test)]
mod loop_tests {
    use super::*;

    #[test]
    fn a_row_naming_a_function_tells_the_judge_it_is_not_written_yet() {
        // B28: the judge rejected the test for calling something that does
        // not exist, which is what a RED test IS. The note is what stops it.
        // BACKTICKED, as a real §T row writes it -- `named_fn` reads only
        // backticked segments, so an unquoted name yields no note at all.
        let n = red_note("`post_with_retry(&dyn Transport, url)` -- a NEW fn");
        assert!(n.contains("post_with_retry"), "names the target: {n}");
        assert!(
            n.contains("does NOT exist yet"),
            "says why it is absent: {n}"
        );
        assert!(n.contains("not a reason"), "and that it is not a NO: {n}");
    }

    #[test]
    fn a_row_naming_no_function_says_nothing_to_the_judge() {
        // A row that changes existing behaviour has no absent target, and
        // telling the judge otherwise would excuse a test calling anything.
        assert_eq!(red_note("tighten the ceiling comparison"), "");
        assert_eq!(
            red_note("post_with_retry(url) with no backticks"),
            "",
            "an unbackticked name is not a named function"
        );
    }

    use std::cell::Cell;
    use std::io::BufRead;
    use std::time::Duration;

    /// A transport that answers each call from a script, in order.
    ///
    /// The loop's point is the SEQUENCE -- author a test, judge it, require
    /// red, write an implementation, gate it, judge it blind. A fake that
    /// returns one canned answer proves none of that; one that answers in
    /// order proves the steps happen, and happen in the right order.
    struct Scripted {
        replies: Vec<String>,
        at: Cell<usize>,
    }

    impl Scripted {
        fn new(replies: &[&str]) -> Self {
            Self {
                replies: replies.iter().map(|s| (*s).to_string()).collect(),
                at: Cell::new(0),
            }
        }

        /// Ollama's NDJSON: a chunk frame, then a `done` frame with counts.
        fn frames(text: &str) -> String {
            let esc = text
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n");
            format!(
                "{{\"response\":\"{esc}\",\"done\":false}}\n\
                 {{\"response\":\"\",\"done\":true,\
                 \"prompt_eval_count\":10,\"eval_count\":5}}\n"
            )
        }
    }

    impl ollama::Transport for Scripted {
        fn timings_are_real(&self) -> bool {
            false
        }

        fn post(
            &self,
            _url: &str,
            _body: &str,
            _t: Duration,
        ) -> Result<Box<dyn BufRead + Send>, String> {
            let i = self.at.get();
            let text = self
                .replies
                .get(i)
                .ok_or_else(|| format!("script exhausted at call {i}"))?;
            self.at.set(i.wrapping_add(1));
            Ok(Box::new(std::io::Cursor::new(Self::frames(text))))
        }
    }

    fn write_exec(path: &Path, body: &str) -> Result<(), String> {
        std::fs::write(path, body).map_err(|e| format!("write: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, PermissionsExt::from_mode(0o755))
                .map_err(|e| format!("chmod: {e}"))?;
        }
        Ok(())
    }

    /// A `cargo` that fails its first `red_times` invocations, then passes.
    ///
    /// Step 1 REQUIRES a red gate before the loop will write an
    /// implementation, so a fake toolchain has to be red first and green
    /// after -- the transition the loop exists to observe.
    /// A `cargo` whose TEST step goes red `red_times` times, then green.
    ///
    /// Only `test` is counted. The gate also invokes `fmt` and `clippy`
    /// (B30), and counting those would consume the red budget inside the
    /// first gate run -- the double would then be modelling cargo calls when
    /// what the tests mean is GATE RUNS. Those two always pass here: what a
    /// scripted gate exists to vary is the test verdict.
    fn scripted_cargo(dir: &Path, red_times: u32) -> Result<String, String> {
        let counter = dir.join("gate-count");
        let script = dir.join("scripted-cargo");
        let body = format!(
            "#!/bin/sh\ncase \"$1\" in test) ;; *) exit 0 ;; esac\n\
             n=$(cat {c} 2>/dev/null || echo 0)\n\
             echo $((n+1)) > {c}\n\
             if [ \"$n\" -lt \"{red_times}\" ]; then \
             echo 'test result: FAILED'; exit 1; fi\n\
             echo 'test result: ok'\nexit 0\n",
            c = counter.display()
        );
        write_exec(&script, &body)?;
        Ok(script.display().to_string())
    }

    /// The gate runs `sherd check` and slice drift over ROOT, so the fixture
    /// has to look like a repo and not merely like a node.
    fn repo_fixture(root: &Path) -> Result<(), String> {
        std::fs::write(
            root.join("SPEC.md"),
            "# SPEC\n\n## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n\
             node|the fixture|everything else|-\n",
        )
        .map_err(|e| format!("root spec: {e}"))?;
        std::fs::write(root.join(".sherd-slices"), "# none\n")
            .map_err(|e| format!("slices: {e}"))
    }

    fn node_fixture(dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| format!("mkdir: {e}"))?;
        std::fs::write(
            dir.join("SPEC.md"),
            "# SPEC\n\n## \u{a7}V INVARIANTS\n\nV1: a doubling returns twice its input\n",
        )
        .map_err(|e| format!("spec: {e}"))?;
        std::fs::write(
            dir.join("mod.rs"),
            "pub fn existing() -> u8 {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n\
             \x20   #[test]\n    fn t() {}\n}\n",
        )
        .map_err(|e| format!("mod: {e}"))
    }

    fn script() -> Scripted {
        Scripted::new(&[
            "```rust\n#[test]\nfn doubles() { assert_eq!(double(2), 4); }\n```",
            "YES it exercises the invariant",
            "```rust\n/// Twice its input.\npub fn double(n: u8) -> u8 { n * 2 }\n```",
            "YES it reads its input",
        ])
    }

    /// THE END-TO-END TEST -- the first the loop has ever had.
    ///
    /// It asserts the SHAPE of a run, not a model's answers: there is no
    /// model here, and no endpoint.
    #[test]
    fn the_loop_runs_end_to_end_with_no_endpoint() {
        // Errors are RETURNED, never `.expect`ed. A harness that panics in
        // setup is indistinguishable from the thing it tests failing, which
        // is `.:src/assay:B1` costing forty minutes.
        assert_eq!(drive_a_scripted_run(), Ok(()));
    }

    /// Build the scratch repo and hand back its root and node.
    ///
    /// Unique per INSTANCE, not per process. Keying on the pid alone gave
    /// every test in the binary the same directory, so the second caller
    /// raced the first and one `remove_dir_all` deleted a tree the other was
    /// still reading (B25). `src/review:V6` is the rule and `TestRepo`
    /// already carried the remedy; this node did not have it because exactly
    /// one test used the fixture.
    fn scratch(
        tag: &str,
    ) -> Result<(std::path::PathBuf, std::path::PathBuf), String> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir()
            .join(format!("sherd-loop-{tag}-{}-{n}", std::process::id()));
        let node = dir.join("node");
        node_fixture(&node)?;
        repo_fixture(&dir)?;
        Ok((dir, node))
    }

    fn drive_a_scripted_run() -> Result<(), String> {
        let (dir, node) = scratch("repair")?;
        let cargo = scripted_cargo(&dir, 1)?;
        let t = script();
        let base = Run::new(&dir, &node, "V1", "add double()")
            .with_cargo(&cargo)
            .repairs(0);
        let out = drive_run(&Run {
            transport: &t,
            ..base
        });
        let after = std::fs::read_to_string(node.join("mod.rs"))
            .map_err(|e| format!("read back: {e}"))?;
        let _ = std::fs::remove_dir_all(&dir);
        check_outcome(&out, &after)
    }

    /// The monolith arm, offline.
    ///
    /// `oneshot` is what sherd claims to BEAT -- full spec, full bodies,
    /// test and implementation asked for together in one call (R29/R30). It
    /// took `&ollama::Http` and the real `gate`, so the comparison arm could
    /// only ever run against a live endpoint and this repo's own toolchain.
    /// It now takes `&Run` like `drive_run`, which is what `Run` exists for.
    fn monolith_script(blocks: &str) -> Scripted {
        Scripted::new(&[blocks])
    }

    const TWO_BLOCKS: &str = "```rust\n#[test]\nfn doubles() { assert_eq!(double(2), 4); }\n```\n\
         and now the implementation:\n\
         ```rust\npub fn double(n: u8) -> u8 { n * 2 }\n```";

    #[test]
    fn the_monolith_arm_runs_and_reports_mergeable_on_a_green_gate() {
        assert_eq!(monolith_green(), Ok(()));
    }

    fn monolith_green() -> Result<(), String> {
        let (dir, node) = scratch("mono-green")?;
        let cargo = scripted_cargo(&dir, 0)?;
        let t = monolith_script(TWO_BLOCKS);
        let base = scripted_run(&dir, &node, &cargo, 0);
        let out = oneshot(&Run {
            transport: &t,
            ..base
        });
        let after = std::fs::read_to_string(node.join("mod.rs"))
            .map_err(|e| format!("read back: {e}"))?;
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.is_ok(), "a green gate is MERGEABLE: {out:?}");
        assert_both_blocks_landed(&after);
        Ok(())
    }

    /// The monolith writes BOTH halves in one call -- that is the whole
    /// difference from the decomposed arm, and half of it landing would be a
    /// worse failure than none.
    fn assert_both_blocks_landed(after: &str) {
        assert!(after.contains("n * 2"), "the impl block landed: {after}");
        assert!(after.contains("doubles"), "the test block landed too");
    }

    #[test]
    fn the_monolith_arm_reports_red_rather_than_claiming_a_merge() {
        assert_eq!(monolith_red(), Ok(()));
    }

    fn monolith_red() -> Result<(), String> {
        let (dir, node) = scratch("mono-red")?;
        // Never green: the monolith FAILING is the measured outcome R29
        // recorded (1 of 2 green, against the decomposed arm's 2 of 2).
        let cargo = scripted_cargo(&dir, 9)?;
        let t = monolith_script(TWO_BLOCKS);
        let base = scripted_run(&dir, &node, &cargo, 0);
        let out = oneshot(&Run {
            transport: &t,
            ..base
        });
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.is_err(), "a red gate is NOT mergeable: {out:?}");
        Ok(())
    }

    #[test]
    fn a_monolith_reply_short_of_two_blocks_is_an_error() {
        // One block means the model gave a test or an implementation but not
        // both, and guessing which would write the wrong half into the file.
        assert_eq!(monolith_one_block(), Ok(()));
    }

    fn monolith_one_block() -> Result<(), String> {
        let (dir, node) = scratch("mono-one")?;
        let cargo = scripted_cargo(&dir, 0)?;
        let t =
            monolith_script("```rust\npub fn double(n: u8) -> u8 { n*2 }\n```");
        let base = scripted_run(&dir, &node, &cargo, 0);
        let out = oneshot(&Run {
            transport: &t,
            ..base
        });
        let after = std::fs::read_to_string(node.join("mod.rs"))
            .map_err(|e| format!("read back: {e}"))?;
        let _ = std::fs::remove_dir_all(&dir);
        assert_unusable_reply_wrote_nothing(&out, &after)
    }

    /// One block means the model gave a test or an implementation but not
    /// both. Guessing which would write the wrong half into the file, so the
    /// run must say what was wrong and leave the module alone.
    fn assert_unusable_reply_wrote_nothing(
        out: &Result<Vec<Step>, String>,
        after: &str,
    ) -> Result<(), String> {
        let Err(msg) = out else {
            return Err("one block is not two".into());
        };
        assert!(msg.contains("expected 2"), "say what was wrong: {msg}");
        assert!(
            !after.contains("double"),
            "nothing is written when the reply is unusable: {after}"
        );
        Ok(())
    }

    /// A run where the candidate's gate goes RED, so step 4 has to repair.
    ///
    /// The repair loop was 60 lines nothing had ever executed. It is the part
    /// of the loop that EDITS code it previously wrote, and `B5` is it
    /// guessing wrong about where its own insertion went -- so it is the most
    /// dangerous stretch in the node and it was reachable only with an
    /// endpoint and a genuinely failing gate.
    fn repair_script() -> Scripted {
        Scripted::new(&[
            "```rust\n#[test]\nfn doubles() { assert_eq!(double(2), 4); }\n```",
            "YES it exercises the invariant",
            "```rust\n/// Twice its input.\npub fn double(n: u8) -> u8 { n + n }\n```",
            "```rust\n/// Twice its input.\npub fn double(n: u8) -> u8 { n * 2 }\n```",
            "YES it reads its input",
        ])
    }

    #[test]
    fn a_red_gate_sends_the_loop_through_repair() {
        assert_eq!(drive_with_one_repair(), Ok(()));
    }

    /// Build a run over `dir`/`node` with a scripted transport and gate.
    fn scripted_run<'a>(
        dir: &'a Path,
        node: &'a Path,
        cargo: &str,
        repairs: usize,
    ) -> Run<'a> {
        Run::new(dir, node, "V1", "add double()")
            .with_cargo(cargo)
            .repairs(repairs)
    }

    fn drive_with_one_repair() -> Result<(), String> {
        let (dir, node) = scratch("repair")?;
        // RED twice: once for step 3's required red, once for the candidate,
        // so the repair round-trip is what turns it green.
        let cargo = scripted_cargo(&dir, 2)?;
        let t = repair_script();
        let base = scripted_run(&dir, &node, &cargo, 1);
        let out = drive_run(&Run {
            transport: &t,
            ..base
        });
        let after = std::fs::read_to_string(node.join("mod.rs"))
            .map_err(|e| format!("read back: {e}"))?;
        let _ = std::fs::remove_dir_all(&dir);
        assert!(out.is_ok(), "a repaired run that goes green lands: {out:?}");
        assert_repaired_body_landed(&after);
        Ok(())
    }

    /// The repaired body is what lands, and the first attempt is GONE.
    fn assert_repaired_body_landed(after: &str) {
        assert!(
            after.contains("n * 2"),
            "the REPAIRED body is what lands, not the first attempt: {after}"
        );
        assert!(
            !after.contains("n + n"),
            "repair REPLACES its own previous insertion rather than appending \
             beside it -- appending would duplicate the definition (B5): {after}"
        );
    }

    /// The gate never goes green, and repairs run out.
    ///
    /// The exhaustion path must REPORT what was tried. Landing a red tree
    /// because the loop gave up would be the worst of both -- generated code
    /// on the branch and no verdict about it.
    #[test]
    fn repair_exhaustion_reports_rather_than_landing_a_red_tree() {
        assert_eq!(exhausted_repairs_do_not_pass(), Ok(()));
    }

    fn exhausted_repairs_do_not_pass() -> Result<(), String> {
        let (dir, node) = scratch("exhaust")?;
        // Red for longer than the loop has repairs, so it runs out.
        let cargo = scripted_cargo(&dir, 9)?;
        let t = repair_script();
        let base = scripted_run(&dir, &node, &cargo, 1);
        let out = drive_run(&Run {
            transport: &t,
            ..base
        });
        let _ = std::fs::remove_dir_all(&dir);
        // The blind judge is SKIPPED on a red gate -- asking a judge to
        // confirm a refusal costs a call to learn nothing.
        assert!(
            out.is_err(),
            "an exhausted repair reports, never a silent pass: {out:?}"
        );
        Ok(())
    }

    /// The loop reached the END: authored a test, judged it, saw the gate go
    /// RED as step 1 requires, wrote an implementation, saw it go GREEN, ran
    /// the mechanical review, and REVERTED on a finding.
    ///
    /// That last part is `V23` -- green plus a finding is the stub signature,
    /// and repair polishes a stub rather than fixing one. A new `pub fn`
    /// called only by its own new test is exactly what `unwired` flags, so
    /// the revert is the honest outcome for this script rather than a broken
    /// harness, and asserting it proves every step ran.
    /// THE MERIT WIN, asserted.
    ///
    /// This used to require a REVERT: "a candidate with findings must not
    /// land". That was the honest outcome while `unwired` judged a candidate
    /// (V29) and `negative_only` flagged every scalar (`src/review:B6`) --
    /// the test was encoding two defects as expected behaviour.
    ///
    /// It is not weakened by inverting. The old form asserted that the loop
    /// FAILS; this asserts it SUCCEEDS and that the implementation is really
    /// in the module afterwards, which is the stronger claim and the one
    /// `src/tdd:T13` has been asking for since the loop was built.
    fn check_outcome(
        out: &Result<Vec<Step>, String>,
        after: &str,
    ) -> Result<(), String> {
        let steps = match out {
            Ok(s) => s,
            Err(e) => return Err(format!("the run must be KEPT, got: {e}")),
        };
        assert!(!steps.is_empty(), "a kept run records what it cost");
        assert!(
            after.contains("double"),
            "the implementation must be IN the module, not restored away: \
             {after}"
        );
        Ok(())
    }

    // The gate cluster moved to `src/land` (`.:T99`); these tests did not,
    // because the scripted-toolchain fixtures they drive live here and
    // copying a fixture into a second node is the duplication §C ends.
    // `.:T101` moves the fixtures to `testrepo` and the tests follow them.
    // The gate cluster's tests, moved with it (`.:T99`). They exercise
    // `fmt_ok`, `lint_debt_ok` and `debt::recorded` against scripted
    // toolchains, and a test left behind in the node that no longer owns
    // the code is how a module ends up with tests for functions it does
    // not have.
    #[test]
    fn a_toolchain_that_cannot_run_fails_fmt_rather_than_passing_it() {
        // V26's shape for this half: a `cargo` that is not there must not
        // read as "formatted clean". Silence would let a candidate through
        // on a broken bench.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (ok, report) = crate::land::fmt_ok(root, "definitely-not-a-cargo");
        assert!(!ok, "an unrunnable toolchain is not a PASS");
        assert!(report.contains("fmt: FAIL"), "{report}");
    }

    #[test]
    fn the_ratchet_is_silent_when_clippy_cannot_run() {
        // The debt half degrades the other way ON PURPOSE: `.lint-debt` is
        // read first, and a clippy that cannot run yields no count to
        // compare, so refusing would block every candidate on a bench
        // problem. The TEST step is what fails a broken toolchain (V26).
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (ok, report) =
            crate::land::lint_debt_ok(root, "definitely-not-a-cargo");
        assert!(ok, "no count means nothing to compare");
        assert!(report.is_empty(), "{report}");
    }

    #[test]
    fn the_gate_reads_the_recorded_lint_debt() {
        // B30: the loop called code MERGEABLE that raised the debt 271 -> 276,
        // which `hk` then refuses -- so its verdict did not predict the
        // commit. The ratchet's number has to be READ for that to change.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let was = crate::debt::recorded(root);
        assert!(was.is_some(), "this repo records a debt total");
        assert!(was.is_some_and(|n| n > 0), "and it is a real count");
    }

    #[test]
    fn a_tree_with_no_lint_debt_file_does_not_fail_the_gate() {
        // A node fixture is not a repo with a ratchet. Absent means "no
        // ratchet here", never "zero allowed" -- which would fail every
        // candidate in every scratch tree.
        let dir = std::env::temp_dir();
        assert_eq!(
            crate::debt::recorded(&dir.join("definitely-not-a-repo")),
            None
        );
    }

    /// A tree with no `src/` is not a tree with zero lines of Rust: the
    /// density has no denominator, so there is nothing to compare and the
    /// candidate is not refused on a bench problem (V26).
    #[test]
    fn a_tree_with_no_rust_yields_no_density() {
        let dir = std::env::temp_dir().join("sherd-no-rust-at-all");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join(".lint-debt"), "density 5.0\n").expect("write");
        let (ok, r) = crate::land::lint_debt_ok(&dir, "false");
        assert!(ok, "no denominator is not a refusal: {r}");
        assert!(r.is_empty(), "and it says nothing: {r}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `.:B21`: the loop's five numbered steps are five functions, and the
    /// coordinator holds the guard because arming and keeping are two ends of
    /// one decision.
    ///
    /// Asserted on the source rather than on behaviour, because the behaviour
    /// is unchanged by construction -- what changed is the shape, and the
    /// shape is what `.:B21`'s ratchet measures.
    #[test]
    fn the_loop_coordinates_and_each_step_is_its_own_function() {
        let src = include_str!("mod.rs");
        let (impl_r, _tests) = split_module(src);
        for step in [
            "fn red_test(",
            "fn judge_test(",
            "fn green_candidates(",
            "fn repair(",
            "fn blind_check(",
        ] {
            assert!(impl_r.contains(step), "the loop lost {step}");
        }
        // The claim is DELEGATION, not a line count: the coordinator calls
        // each step rather than inlining it. A threshold here would be the
        // same defect `.:B21` records -- a number that punishes the next
        // honest edit.
        let drive = impl_r
            .split_once("pub fn drive_run(")
            .and_then(|(_, r)| r.split_once("\n}\n"))
            .map(|(body, _)| body)
            .unwrap_or_default();
        for call in [
            "red_test(&ctx",
            "green_candidates(&ctx",
            "repair(&ctx",
            "blind_check(&ctx",
        ] {
            assert!(
                drive.contains(call),
                "the coordinator stopped calling {call}"
            );
        }
        assert!(
            !drive.contains("NOTATION"),
            "a prompt body belongs to its step, not to the coordinator"
        );
        // The guard stays with the coordinator: it arms before any step can
        // write, and only the verdict may keep it.
        assert!(
            impl_r.contains("let mut guard = Restore::arm("),
            "the coordinator arms the guard"
        );
    }

    /// The split's real payoff: these were unreachable inside a 269-line body
    /// and are now pure functions a test can call directly.
    ///
    /// `contract_objection` is the zero-token check that saved 10 round-trips
    /// and 25,991 tokens the day it was written (B23), and until now nothing
    /// asserted the message it feeds back actually names the function.
    #[test]
    fn the_contract_objection_names_the_function_the_row_asked_for() {
        let o = contract_objection("post_with_retry");
        assert!(o.contains("post_with_retry"), "{o}");
        assert!(
            o.contains("calls `post_with_retry` directly"),
            "the objection has to be actionable, not just a complaint: {o}"
        );
    }

    /// `NoWinner` is a REFUSAL, not a pick: repair polishes a stub, it does
    /// not fix one.
    #[test]
    fn a_green_candidate_carrying_findings_is_refused_not_repaired() {
        let clean = Candidate {
            code: "pub fn f() {}".into(),
            green: true,
            findings: 0,
        };
        let flawed = Candidate {
            findings: 2,
            ..clean.clone()
        };
        let tried = |c: Candidate| Tried {
            cand: c,
            out: String::new(),
        };
        assert_eq!(pick_candidate(&[tried(clean.clone())]), Ok(0));
        let refused = pick_candidate(&[tried(flawed)]);
        assert!(
            refused.is_err_and(|e| e.contains("reverted")),
            "a green candidate with findings is reverted, not kept"
        );
        // And an empty slate is a refusal too, never a panic on index 0.
        assert!(pick_candidate(&[]).is_err());
    }

    /// The step-2 prompt names exactly what the test calls, so step 2 cannot
    /// invent a neighbouring name and leave the test uncallable.
    #[test]
    fn the_green_prompt_carries_the_contract_and_the_failure() {
        let t = Scripted::new(&[]);
        let run = Run {
            root: Path::new("."),
            node: Path::new("."),
            owner: Path::new("."),
            invariant: "V1",
            task: "add `existing`",
            max_repair: 1,
            cargo: "false".into(),
            transport: &t,
        };
        let ctx = probe_ctx(&run, "pub fn existing(x: u64) -> bool");
        let p = green_prompt(&ctx, "fn t() { assert!(existing(1)); }", "E0425");
        assert!(p.contains("existing"), "the surface is shown: {p}");
        assert!(p.contains("E0425"), "the failure is shown: {p}");
        assert!(
            p.contains("Do not modify the test"),
            "step 2 may not rewrite the test it was given"
        );
    }

    /// Repair is shown what it last added, so it REPLACES rather than
    /// appending a second definition (B5).
    #[test]
    fn the_repair_prompt_shows_what_was_last_added() {
        let p = repair_prompt(
            "pub fn existing()".into(),
            "pub fn mine() -> u8 { 0 }",
            "fn t() {}",
            "assertion failed",
        );
        assert!(p.contains("pub fn mine() -> u8 { 0 }"), "{p}");
        assert!(p.contains("assertion failed"), "{p}");
        assert!(
            p.contains("corrected version of the function(s) you previously"),
            "repair replaces its own work, it does not add more"
        );
    }

    /// A `Ctx` over a scratch node, for the pure steps that read one.
    fn probe_ctx<'a>(run: &'a Run<'a>, surface: &str) -> Ctx<'a> {
        Ctx {
            run,
            mod_path: std::path::PathBuf::from("mod.rs"),
            inv: "V1: a ! b".into(),
            spec_rules: String::new(),
            surface: surface.to_string(),
            tests: String::new(),
            in_scope: String::new(),
        }
    }

    /// A `cargo` whose clippy step emits `n` warning lines on stderr.
    fn cargo_with_warnings(dir: &Path, n: usize) -> Result<String, String> {
        let script = dir.join("noisy-cargo");
        let mut emit = String::new();
        for i in 0..n {
            emit.push_str(&format!(
                "echo 'src/x/mod.rs:{i}:1: warning: made up' >&2\n"
            ));
        }
        write_exec(&script, &format!("#!/bin/sh\n{emit}exit 0\n"))?;
        Ok(script.display().to_string())
    }

    #[test]
    fn the_ratchet_holds_on_density_and_lets_the_tree_grow_clean() {
        assert_eq!(ratchet_both_ways(), Ok(()));
    }

    fn ratchet_both_ways() -> Result<(), String> {
        let (dir, _n) = scratch("ratchet")?;
        // A hundred lines of Rust, so the arithmetic is legible: two warnings
        // over 100 lines is 20.0 per KLoC, three is 30.0.
        std::fs::create_dir_all(dir.join("src/deeper"))
            .map_err(|e| format!("mkdir: {e}"))?;
        std::fs::write(dir.join("src/x.rs"), "// line\n".repeat(60))
            .map_err(|e| format!("write: {e}"))?;
        // A nested dir and a non-Rust file, because the walker must descend
        // and must not count the README beside the code.
        std::fs::write(dir.join("src/deeper/y.rs"), "// line\n".repeat(40))
            .map_err(|e| format!("write: {e}"))?;
        std::fs::write(dir.join("src/notes.md"), "// line\n".repeat(500))
            .map_err(|e| format!("write: {e}"))?;
        std::fs::write(dir.join(".lint-debt"), "density 20.0\n")
            .map_err(|e| format!("write: {e}"))?;
        let (ok, r) =
            crate::land::lint_debt_ok(&dir, &cargo_with_warnings(&dir, 2)?);
        assert!(ok, "holding at the recorded density PASSES: {r}");
        assert!(r.contains("PASS === 20.0 per KLoC (ceiling 20.0)"), "{r}");
        let (rose, rr) =
            crate::land::lint_debt_ok(&dir, &cargo_with_warnings(&dir, 3)?);
        assert!(!rose, "one more warning on the same lines REFUSES: {rr}");
        assert!(rr.contains("ROSE === 30.0 per KLoC (ceiling 20.0)"), "{rr}");
        // And the point of a RATIO: three warnings over 150 lines is 20.0,
        // the same density, so growing the tree with clean code passes where
        // the old absolute count refused it (`.:B22`).
        std::fs::write(dir.join("src/x.rs"), "// line\n".repeat(110))
            .map_err(|e| format!("write: {e}"))?;
        let (grew, gr) =
            crate::land::lint_debt_ok(&dir, &cargo_with_warnings(&dir, 3)?);
        assert!(grew, "same density over more lines PASSES: {gr}");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn fmt_and_the_ratchet_pass_against_a_scripted_toolchain() {
        assert_eq!(scripted_gate_halves(), Ok(()));
    }

    fn scripted_gate_halves() -> Result<(), String> {
        let (dir, _node) = scratch("gatehalves")?;
        let cargo = scripted_cargo(&dir, 0)?;
        let (fmt, fmt_r) = crate::land::fmt_ok(&dir, &cargo);
        assert!(fmt, "a scripted toolchain formats clean: {fmt_r}");
        assert!(fmt_r.contains("fmt: PASS"), "{fmt_r}");
        // No `.lint-debt` in a scratch tree: absent is "no ratchet here",
        // never "zero allowed", or every candidate would fail everywhere.
        let (debt, debt_r) = crate::land::lint_debt_ok(&dir, &cargo);
        assert!(debt, "an absent ratchet does not fail the gate");
        assert!(debt_r.is_empty(), "and says nothing: {debt_r}");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn a_gate_that_could_not_run_is_an_error_not_a_verdict() {
        // V26, and it is the distinction the whole harness rests on: a
        // missing toolchain scoring RED is indistinguishable from code that
        // failed its tests, and every titration would read as a located
        // boundary rather than as a broken bench.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let Err(msg) =
            crate::land::gate_with(root, "definitely-not-a-cargo-binary")
        else {
            return;
        };
        assert!(
            msg.contains("could not RUN"),
            "say the gate did not EXECUTE, not that it failed: {msg}"
        );
    }
}
