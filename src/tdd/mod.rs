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

/// Step 3. Local, deterministic, zero tokens. Reports what RAN, not only what
/// failed (root V48).
/// # Errors
/// The toolchain could not be RUN. That is not a red gate: a gate that did
/// not execute has said nothing, and returning `false` for it made a missing
/// `cargo` indistinguishable from a failing test. In `drive_from` that
/// mattered -- step 1 requires the gate to be RED, so an absent toolchain
/// read as "red as required" and the loop would have written code against a
/// gate that never ran. `.:V48` for a subprocess (B24, tdd B17 recurring).
/// The toolchain, from `BBX_CARGO` or the default. The EDGES read the env;
/// the loop carries it in `Run` so a test can point at a scripted one without
/// mutating process-global state that other tests share (`V27`).
#[must_use]
pub fn cargo_bin() -> String {
    std::env::var("BBX_CARGO").unwrap_or_else(|_| "cargo".into())
}

/// Run the gate with an explicit toolchain.
///
/// # Errors
/// See [`gate`].
/// The gate, with the toolchain from the environment.
///
/// # Errors
/// The toolchain could not be RUN. That is not a red gate (V26).
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

pub fn gate(root: &Path) -> Result<(bool, String), String> {
    gate_with(root, &cargo_bin())
}

pub fn gate_with(root: &Path, cargo: &str) -> Result<(bool, String), String> {
    // Plain `cargo test`, exactly `hk`'s test step. NOT `RUSTFLAGS=-D
    // warnings`: RUSTFLAGS reaches every path dep, so `itok`'s own two
    // `dead_code` warnings turned this gate red for code blackbox does not
    // own -- and then every candidate and every repair was judged against a
    // gate that could not go green whatever the model wrote (B26).
    //
    // `.:B6` found this and fixed `hk.pkl` by moving `-D warnings` after `--`
    // on the CLIPPY step, where it scopes to this crate. The loop kept the
    // old mechanism, which is `src/fed:B9`: fixing a shared rule must be
    // followed by finding who does not use it.
    //
    // BOUNDED: warnings are now clippy's job and clippy is `hk`'s step, not
    // this one. The loop's gate no longer catches a warnings-only regression;
    // the commit gate still does, and `bbx apply` cannot commit without it.
    let out = Command::new(cargo)
        .args(["test", "--offline"])
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
    // The loop's gate and the commit's gate are ONE rule, which is what the
    // header claims and what B30 measured as false: MERGEABLE was declared
    // for code `hk` refuses on fmt and on the lint ratchet.
    let (fmt, fmt_r) = fmt_ok(root, cargo);
    let (debt, debt_r) = lint_debt_ok(root, cargo);
    report.push_str(&fmt_r);
    report.push_str(&debt_r);
    Ok((
        tests_ok && viol == 0 && drift.is_empty() && fmt && debt,
        report,
    ))
}

/// `cargo fmt --check`, as `hk`'s first step runs it.
///
/// The model's insertion is not formatted -- the generated test landed at
/// column 0 inside a module -- so this refuses a candidate the commit gate
/// would refuse (B30).
fn fmt_ok(root: &Path, cargo: &str) -> (bool, String) {
    let out = Command::new(cargo)
        .args(["fmt", "--check"])
        .current_dir(root)
        .output();
    let ok = out.is_ok_and(|o| o.status.success());
    (
        ok,
        format!("=== fmt: {} ===\n", if ok { "PASS" } else { "FAIL" }),
    )
}

/// THE RATCHET, as `hk` runs it: the count may fall, never rise.
///
/// `.lint-debt` carries the number. Without this the loop called code
/// MERGEABLE that raised the debt 271 -> 276, which `hk` then refuses --
/// so the loop's verdict did not predict the commit (B30).
fn lint_debt_ok(root: &Path, cargo: &str) -> (bool, String) {
    let Some(was) = recorded_debt(root) else {
        return (true, String::new());
    };
    let Some(now) = clippy_warnings(root, cargo) else {
        return (true, String::new());
    };
    let ok = now <= was;
    let word = if ok { "PASS" } else { "ROSE" };
    (
        ok,
        format!("=== lint debt: {word} === {now} (recorded {was})\n"),
    )
}

/// How many warnings clippy reports for THIS crate's own sources.
///
/// `None` when clippy could not run: `.lint-debt` was read first, so there is
/// simply nothing to compare, and refusing would block every candidate on a
/// bench problem. The TEST step is what fails a broken toolchain (V26).
fn clippy_warnings(root: &Path, cargo: &str) -> Option<usize> {
    let o = Command::new(cargo)
        .args(["clippy", "--all-targets", "--message-format=short"])
        .current_dir(root)
        .output()
        .ok()?;
    Some(
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .filter(|l| l.starts_with("src/") && l.contains(": warning"))
            .count(),
    )
}

/// The `total` line of `.lint-debt`, if the file is there.
fn recorded_debt(root: &Path) -> Option<usize> {
    let text = std::fs::read_to_string(root.join(".lint-debt")).ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("total ")?.trim().parse().ok())
}

fn tail(s: &str, n: usize) -> &str {
    if s.len() <= n { s } else { &s[s.len() - n..] }
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
/// for test AND implementation together. This is what blackbox claims to beat.
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
    let (ok, out) = gate_with(r.root, &r.cargo)?;
    let sent: u64 = c.log.iter().map(|s| s.prompt_tokens).sum();
    eprintln!("\n  1 round-trip · {sent} tok sent · max single call {sent}");
    if ok {
        eprintln!("  VERDICT: MERGEABLE -- gates green");
        Ok(c.log)
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
            cargo: cargo_bin(),
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
    /// rather than setting `BBX_CARGO`, which is process-global and shared
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

pub fn drive_run(r: &Run) -> Result<Vec<Step>, String> {
    let (root, node, owner, invariant, task, max_repair) =
        (r.root, r.node, r.owner, r.invariant, r.task, r.max_repair);
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
    // The judge is shown the impl half and told to check names against it,
    // but the test it judges lives in the OTHER half and may legitimately
    // reuse a double declared there. Without these it rejects a good test
    // for referring to something that "does not appear" (B27).
    let in_scope = test_decls(tests_r);
    // A cold endpoint's first call carries a disk load the eta was never
    // taught about, and the 4x ceiling then kills step 1 (`.:ollama:prewarm`).
    ollama::prewarm(r.transport);
    let mut c = Caller::new(r.transport);

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
        test_fn = ollama::rust_block(&c.run(&prompt, label)?);

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

        let red_note = red_note(task);
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
            if attempt == 0 {
                "1b judge"
            } else {
                "1b re-judge"
            },
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
    let (red_ok, red_out) = gate_with(root, &r.cargo)?;
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
        let code = ollama::rust_block(&c.run_sampled(
            &green_prompt,
            label,
            ollama::Sampling::candidate(k),
        )?);
        std::fs::write(&mod_path, insert_impl(&with_test, &code))
            .map_err(|e| e.to_string())?;
        let (g, o) = gate_with(root, &r.cargo)?;
        let added = crate::code::public_fns(&code);
        let cur =
            std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
        let (_ci, ct) = split_module(&cur);
        // NOT `unwired` here. Its own wording is "a `pub fn` called only from
        // tests LANDED but was never wired in" -- the subject is code that
        // shipped and STAYED unwired. A function born in the same breath as
        // its test has landed nothing yet, and at the moment of judgement
        // nothing else can call it: the loop only appends, so EVERY correct
        // run tripped it and V23 made that fatal (V29). The rule is not
        // weakened -- `land::evidence` runs `review::commit` over every
        // commit on the branch, which is where "landed" applies (T19).
        //
        // These three DO belong here: each judges the candidate's own
        // quality, which is complete the moment it is written.
        let mut found = crate::review::negative_only(&code, ct, &added);
        found.extend(crate::review::ignored_input(&code, &added));
        found.extend(crate::review::undocumented(&code, &added));
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
        let fixed = ollama::rust_block(&c.run(
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
        let g = gate_with(root, &r.cargo)?;
        ok = g.0;
        out = g.1;
    }

    // 5 -- the second lens. Only when the gate is green: a red gate has already
    // said no, and asking a judge to confirm it costs a call to learn nothing.
    if ok {
        let verdict =
            c.run(&blind_prompt(&inv, &last_added), "5 blind judge")?;
        let first = verdict.trim().lines().next().unwrap_or("").to_string();
        eprintln!("  blind: {}", first.chars().take(78).collect::<String>());
        if !is_yes(&verdict) {
            return Err(format!(
                "gates green, second lens says NO -- reverted. objection: {}",
                verdict.trim()
            ));
        }
    }

    let sent: u64 = c.log.iter().map(|s| s.prompt_tokens).sum();
    let max = c.log.iter().map(|s| s.prompt_tokens).max().unwrap_or(0);
    eprintln!(
        "\n  {} round-trips · {sent} tok sent · max single call {max}",
        c.log.len()
    );
    if ok {
        eprintln!("  VERDICT: MERGEABLE -- gates green + second lens");
        guard.keep();
        Ok(c.log)
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
    fn a_toolchain_that_cannot_run_fails_fmt_rather_than_passing_it() {
        // V26's shape for this half: a `cargo` that is not there must not
        // read as "formatted clean". Silence would let a candidate through
        // on a broken bench.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let (ok, report) = fmt_ok(root, "definitely-not-a-cargo");
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
        let (ok, report) = lint_debt_ok(root, "definitely-not-a-cargo");
        assert!(ok, "no count means nothing to compare");
        assert!(report.is_empty(), "{report}");
    }

    #[test]
    fn the_gate_reads_the_recorded_lint_debt() {
        // B30: the loop called code MERGEABLE that raised the debt 271 -> 276,
        // which `hk` then refuses -- so its verdict did not predict the
        // commit. The ratchet's number has to be READ for that to change.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let was = recorded_debt(root);
        assert!(was.is_some(), "this repo records a debt total");
        assert!(was.is_some_and(|n| n > 0), "and it is a real count");
    }

    #[test]
    fn a_tree_with_no_lint_debt_file_does_not_fail_the_gate() {
        // A node fixture is not a repo with a ratchet. Absent means "no
        // ratchet here", never "zero allowed" -- which would fail every
        // candidate in every scratch tree.
        let dir = std::env::temp_dir();
        assert_eq!(recorded_debt(&dir.join("definitely-not-a-repo")), None);
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
    fn the_ratchet_passes_when_the_count_holds_and_refuses_when_it_rises() {
        assert_eq!(ratchet_both_ways(), Ok(()));
    }

    fn ratchet_both_ways() -> Result<(), String> {
        let (dir, _n) = scratch("ratchet")?;
        std::fs::write(dir.join(".lint-debt"), "total 2\n")
            .map_err(|e| format!("write: {e}"))?;
        let (ok, r) = lint_debt_ok(&dir, &cargo_with_warnings(&dir, 2)?);
        assert!(ok, "holding at the recorded count PASSES: {r}");
        assert!(r.contains("=== lint debt: PASS === 2 (recorded 2)"), "{r}");
        let (rose, rr) = lint_debt_ok(&dir, &cargo_with_warnings(&dir, 3)?);
        assert!(!rose, "one more warning REFUSES: {rr}");
        assert!(rr.contains("ROSE === 3 (recorded 2)"), "{rr}");
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
        let (fmt, fmt_r) = fmt_ok(&dir, &cargo);
        assert!(fmt, "a scripted toolchain formats clean: {fmt_r}");
        assert!(fmt_r.contains("fmt: PASS"), "{fmt_r}");
        // No `.lint-debt` in a scratch tree: absent is "no ratchet here",
        // never "zero allowed", or every candidate would fail everywhere.
        let (debt, debt_r) = lint_debt_ok(&dir, &cargo);
        assert!(debt, "an absent ratchet does not fail the gate");
        assert!(debt_r.is_empty(), "and says nothing: {debt_r}");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

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

    #[test]
    fn a_gate_that_could_not_run_is_an_error_not_a_verdict() {
        // V26, and it is the distinction the whole harness rests on: a
        // missing toolchain scoring RED is indistinguishable from code that
        // failed its tests, and every titration would read as a located
        // boundary rather than as a broken bench.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let Err(msg) = gate_with(root, "definitely-not-a-cargo-binary") else {
            return;
        };
        assert!(
            msg.contains("could not RUN"),
            "say the gate did not EXECUTE, not that it failed: {msg}"
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

    /// The gate runs `bbx check` and slice drift over ROOT, so the fixture
    /// has to look like a repo and not merely like a node.
    fn repo_fixture(root: &Path) -> Result<(), String> {
        std::fs::write(
            root.join("SPEC.md"),
            "# SPEC\n\n## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n\
             node|the fixture|everything else|-\n",
        )
        .map_err(|e| format!("root spec: {e}"))?;
        std::fs::write(root.join(".bbx-slices"), "# none\n")
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
        // is `.:assay:B1` costing forty minutes.
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
            .join(format!("bbx-loop-{tag}-{}-{n}", std::process::id()));
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
    /// `oneshot` is what blackbox claims to BEAT -- full spec, full bodies,
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
}
