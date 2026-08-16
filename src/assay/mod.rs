//! The assay: a corpus, and a way to grade a model against it.
//!
//! An assay tests a sample against what it is CLAIMED to be and reports when
//! the claim is false. That is what a known-answer corpus plus a compiler
//! grader do, and it is a different activity from writing code with a model,
//! which is `crate::tdd`.
//!
//! Promoted from `src/tdd` where it was the LARGEST cluster -- larger than
//! the loop's own helpers -- and where its 460 lines of corpora inflated a
//! node whose subject is the loop (`.:R48`). A SIBLING at depth 2, so it
//! deepens no chain (`.:V110`).
//!
//! The grader is `rustc`, never a model. A model grader would confound every
//! result twice: `.:R40` measured the judge as itself precision-sensitive,
//! and `src/tdd:B2` is a judge loosening under pressure.
//!
//! `NOTATION` still comes from `crate::tdd`, which is the wrong direction --
//! a measurement node depending on the loop for a string constant. It is a
//! caveman-reading primer and belongs with `crate::spec`, which owns
//! `SPEC.md` structure. Recorded as T2 here rather than smuggled into this
//! move.

// `blind_prompt` is the LOOP's step-5 judge prompt, so it lives with the
// loop; this node MEASURES that judge and therefore depends on it, which is
// the right direction. `blind_prompt_bare` is assay-only -- it exists to strip
// the tells and see whether the checklist was doing the work (`.:R40`).
use crate::tdd::{NOTATION, blind_prompt};
use std::process::Command;
/// Which channel carries an invariant to the writer (`.:R43`, `.:V107`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    /// The type and signature already encode the rule, so wording is free.
    /// Predicts PASS at both sharp and vague.
    Type,
    /// The rule is a magic number or a boundary set no signature carries, so
    /// prose is the only channel. Predicts sharp PASS, vague FAIL.
    Prose,
    /// Predicted beyond the frontier: FAIL at either wording.
    Beyond,
}

impl Channel {
    /// What this class predicts for `(sharp, vague)`.
    #[must_use]
    pub const fn predicts(self) -> (bool, bool) {
        match self {
            Self::Type => (true, true),
            Self::Prose => (true, false),
            Self::Beyond => (false, false),
        }
    }
}

/// One generation item: the same function asked for twice, once from a sharp
/// invariant and once from a vague one.
///
/// `tests` are HIDDEN from the writer. A writer shown the test can satisfy it
/// without reading the invariant, which is the stub path recorded three times
/// in `§B` -- and it would measure the wrong thing entirely, since the
/// question is whether invariant PRECISION drives writing (`.:R40`).
pub struct GenItem {
    /// Which channel carries this rule, PREDICTED before the run (`.:R43`).
    ///
    /// Registered up front so the three-way split is falsifiable. A class
    /// assigned after seeing the scores would fit any result, which is how a
    /// story survives a measurement that should have killed it.
    pub predicted: Channel,
    /// The invariant as a careful `§V` row states it.
    pub sharp: &'static str,
    /// The same rule as a hurried row states it: subject named, deciding
    /// property left out.
    pub vague: &'static str,
    /// The signature the writer must fill.
    pub sig: &'static str,
    /// Types and constants both the candidate and the tests need.
    pub preamble: &'static str,
    /// The grader. Written before any candidate existed.
    pub tests: &'static str,
}

/// Ask for an implementation from an invariant and a signature, nothing else.
#[must_use]
pub fn gen_prompt(inv: &str, sig: &str, preamble: &str) -> String {
    format!(
        "{NOTATION}\nInvariant:\n  {inv}\n\n\
         In scope already:\n```rust\n{preamble}\n```\n\n\
         Write the body of exactly this function so that it satisfies the \
         invariant:\n```rust\n{sig}\n```\n\n\
         Reply with the complete function and nothing else. No tests, no \
         explanation, no `mod`."
    )
}

/// Ask for a TEST, never an implementation.
///
/// T83, variable 2 of 3 (`.:V108`). The writer is blind here exactly as in
/// [`gen_prompt`] -- same invariant, same signature, no implementation to
/// read -- so the only thing differing between the two arms is WHICH TESTS
/// grade the candidate: hidden ones written before any candidate existed, or
/// one the model wrote itself.
///
/// `src/tdd:B2` and `B12` are both a model writing a test that agrees with
/// its own wrong implementation, which makes self-authored tests the live
/// suspect for `src/tdd:T13`'s zero merit wins.
#[must_use]
pub fn test_prompt(inv: &str, sig: &str, preamble: &str) -> String {
    format!(
        "{NOTATION}\nInvariant:\n  {inv}\n\n\
         In scope already:\n```rust\n{preamble}\n```\n\n\
         Write a `#[test]` function that PROVES this invariant holds for:\n\
         ```rust\n{sig}\n```\n\n\
         An implementation that violated the invariant must FAIL your test. \
         Reply with one ```rust block containing `#[cfg(test)] mod t {{ .. }}` \
         and nothing else. No implementation."
    )
}

/// Ask for an implementation with NO signature given.
///
/// T84, variable 3 of 3 (`.:V108`). `.:R44` handed the writer a signature;
/// `bbx tdd` makes it invent one, and `src/tdd:B12` is that going wrong -- a
/// test calling `check_edge_depths` while step 2 defined a different name,
/// unrecoverable by three repairs.
///
/// Graded against the SAME hidden tests, so a name or arity the tests cannot
/// call shows up as `Grade::NoCompile` rather than as a wrong answer. That
/// distinction is the whole measurement.
#[must_use]
pub fn gen_prompt_no_sig(inv: &str, preamble: &str) -> String {
    format!(
        "{NOTATION}\nInvariant:\n  {inv}\n\n\
         In scope already:\n```rust\n{preamble}\n```\n\n\
         Write the public function that satisfies this invariant. Choose its \
         name and signature yourself.\n\n\
         Reply with the complete function and nothing else. No tests, no \
         explanation, no `mod`."
    )
}

/// A deliberately WRONG implementation that still compiles.
///
/// The mutant a test must kill. `.:V111` says a self-authored test cannot
/// grade its own author; this is how that gets a mechanical remedy instead of
/// a warning -- run the test against a known-wrong body and require it to
/// FAIL. A test that passes this measured nothing, which is `src/fed:B6`
/// (`detect_cycles` returning `Vec::new()` under a comment reading "satisfies
/// the current test suite") caught before it lands rather than after.
///
/// Each is the plausible-stub shape: compiles, reads its inputs or ignores
/// them quietly, returns a fixed or passthrough value.
///
/// Returns `None` for an unknown signature rather than a guess -- a mutant
/// nobody chose would make the measurement meaningless.
#[must_use]
pub fn stub_for(sig: &str) -> Option<&'static str> {
    let name = sig.split('(').next().unwrap_or("").trim();
    STUBS.iter().find(|(n, _)| *n == name).map(|(_, s)| *s)
}

/// The mutants, as data. One per corpus signature.
const STUBS: &[(&str, &str)] = &[
    ("pub fn working", "pub fn working(_w: u64) -> u64 { 0 }"),
    (
        "pub fn bucket",
        "pub fn bucket(_n: u64) -> &'static str { \"b0\" }",
    ),
    ("pub fn is_yes", "pub fn is_yes(_v: &str) -> bool { true }"),
    (
        "pub fn verdict",
        "pub fn verdict(_c: u64, _b: u64) -> Verdict { Verdict::Fits { slack: 0 } }",
    ),
    (
        "pub fn for_path",
        "pub fn for_path(_r: &[(String, u64)], default: u64, _p: &str) -> u64 { default }",
    ),
    (
        "pub fn checked_working",
        "pub fn checked_working(_w: u64) -> Option<u64> { Some(0) }",
    ),
    ("pub fn sign", "pub fn sign(_n: i64) -> Sign { Sign::Zero }"),
    (
        "pub fn abort_budget_ms",
        "pub fn abort_budget_ms(eta_ms: u64) -> u64 { eta_ms }",
    ),
    (
        "pub fn is_cached",
        "pub fn is_cached(_t: u64, _m: u64) -> bool { false }",
    ),
    (
        "pub fn parse_limit",
        "pub fn parse_limit(_l: &str) -> Option<(String, u64)> { None }",
    ),
    (
        "pub fn escape_cell",
        "pub fn escape_cell(s: &str) -> String { s.to_string() }",
    ),
];

/// The same request, prefixed with a real node's lens pack.
///
/// T82, variable 1 of 3 (`.:V108`). R44 measured writing from a ~500 token
/// prompt; `bbx tdd` sends the node's whole chain, ~10k after T41. R15 and
/// R16 measured what a fat pack COSTS in wall clock. Whether it makes the
/// model WORSE at writing is a different question, and the one the `§G`
/// TARGET line rests on -- federation is only worth having if the context it
/// assembles does not degrade the work.
///
/// Everything after the pack is byte-identical to [`gen_prompt`], so the pack
/// is the only variable.
#[must_use]
pub fn gen_prompt_in_context(
    pack: &str,
    inv: &str,
    sig: &str,
    preamble: &str,
) -> String {
    format!(
        "Here is the specification of the module you are working in.\n\n\
         {pack}\n\n---\n\n{}",
        gen_prompt(inv, sig, preamble)
    )
}

/// Four outcomes, not two.
///
/// A test that will not COMPILE graded nothing, and folding that into `Fail`
/// would credit an unusable test with a correct rejection -- `src/tdd:V26`
/// one level finer. T83 needs it: a model whose own test does not build has
/// caught nothing, and must not be scored as if it had.
///
/// `Hung` is `V6`, and it is the same rule a third time: the MODEL writes
/// what gets run, so termination is not assumable, and a run that never ends
/// decided nothing either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grade {
    /// Compiled, every test passed.
    Pass,
    /// Compiled, a test failed.
    Fail,
    /// Did not compile.
    NoCompile,
    /// Compiled, ran, and was still running at [`GRADE_TIMEOUT`].
    Hung,
}

/// How long a graded child may run before it is killed and called `Hung`.
///
/// The whole corpus compiles and runs in under a second, so this is ~10x
/// headroom rather than a tuned number, and it bounds a 33-row sweep at five
/// and a half minutes in the worst case. The number matters far less than
/// the existence of a bound: without one the wait is unbounded, which is
/// `B2` -- fifty-five minutes on a single `while eta <= u64::MAX / 4`.
pub const GRADE_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(10);

/// How often the child is checked. Small enough to be invisible against a
/// sub-second run, large enough not to spin a core.
const POLL: std::time::Duration = std::time::Duration::from_millis(25);

/// Compile `candidate` against `tests` and run them, under a clock.
///
/// The grader is `rustc`, never a model. A model grader would confound this
/// twice: `.:R40` measured the judge as itself precision-sensitive, and
/// `src/tdd:B2` is a judge loosening under pressure.
///
/// # Errors
/// The compiler could not be executed, or the scratch file could not be
/// written. Both are ERRORS, never verdicts.
pub fn grade_detail(
    candidate: &str,
    preamble: &str,
    tests: &str,
    rustc: &str,
) -> Result<Grade, String> {
    let (src, bin) = scratch_paths();
    std::fs::write(&src, format!("{preamble}\n{candidate}\n{tests}\n"))
        .map_err(|e| format!("scratch write: {e}"))?;
    let built = compile(rustc, &src, &bin)?;
    let g = if built {
        run_bounded(&bin)?
    } else {
        Grade::NoCompile
    };
    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&bin);
    Ok(g)
}

/// A unique source and binary path per call, so concurrent grades cannot
/// overwrite each other's scratch.
fn scratch_paths() -> (std::path::PathBuf, std::path::PathBuf) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    let pid = std::process::id();
    (
        dir.join(format!("bbx_gen_{pid}_{n}.rs")),
        dir.join(format!("bbx_gen_{pid}_{n}")),
    )
}

/// `rustc` itself is trusted to terminate -- it is not what the model wrote.
fn compile(
    rustc: &str,
    src: &std::path::Path,
    bin: &std::path::Path,
) -> Result<bool, String> {
    let out = Command::new(rustc)
        .args(["--test", "--edition", "2021", "-A", "warnings"])
        .arg(src)
        .arg("-o")
        .arg(bin)
        .output()
        .map_err(|e| format!("{rustc} could not run: {e}"))?;
    Ok(out.status.success())
}

/// Run the compiled tests, and stop waiting at [`GRADE_TIMEOUT`].
///
/// `Stdio::null()`, deliberately: a piped stream nobody drains DEADLOCKS
/// once the child fills the buffer, which would reintroduce `B2` through the
/// other door. Only the exit status was ever read.
///
/// A child still alive at the deadline is killed and reported `Hung`. It is
/// NOT reported `Fail`: a killed process exits non-zero, so folding the two
/// would record a wrong answer about code that never gave one -- which for
/// the ambiguity detector means a DISAGREEMENT about a row nothing
/// disagreed about (`V6`).
fn run_bounded(bin: &std::path::Path) -> Result<Grade, String> {
    let mut child = Command::new(bin)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("compiled binary could not run: {e}"))?;
    let deadline = std::time::Instant::now().checked_add(GRADE_TIMEOUT);
    while deadline.is_none_or(|d| std::time::Instant::now() < d) {
        match child.try_wait() {
            Err(e) => return Err(format!("waiting on the child: {e}")),
            Ok(Some(s)) if s.success() => return Ok(Grade::Pass),
            Ok(Some(_)) => return Ok(Grade::Fail),
            Ok(None) => std::thread::sleep(POLL),
        }
    }
    reap(&mut child);
    Ok(Grade::Hung)
}

/// Kill the child and WAIT for it, so the sweep does not accumulate zombies
/// across thirty-three rows.
fn reap(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

/// What a test and an implementation, each written BLIND from the SAME `§V`
/// row, said about each other.
///
/// `.:V112`. Two calls and a compile, with no reference answer anywhere: the
/// question is not whether either half is RIGHT, it is whether they read the
/// row the same way. R51 measured them disagreeing 7 of 33, and R54 read the
/// disagreements back -- every one was a rule the row left unstated and the
/// two halves filled in differently. So a disagreement is evidence about the
/// ROW, which is what makes this an instrument for the spec rather than for
/// the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reading {
    /// The blind test passes the blind implementation. One reading, twice.
    Agree,
    /// The blind test REJECTS the blind implementation. Two readings of one
    /// row, and the gap between them is in the row.
    Disagree,
    /// The test could not be compiled against the implementation at all.
    /// It graded NOTHING, so it is not a disagreement.
    ///
    /// `src/tdd:B12`'s name-or-arity mismatch is one cause and was the one
    /// assumed here; `.:R55` measured the other and it dominates -- `sign`
    /// failed 3/3 on `let samples: [i64; 10]` holding nine elements. An
    /// ordinary compile error in the model's test, not a naming problem.
    Uncallable,
    /// The pair compiled and then never terminated. `V6`, and `B2` is
    /// fifty-five minutes of it. Killed at [`GRADE_TIMEOUT`] and reported
    /// apart: a run that never ended decided nothing about the row either.
    Hung,
}

impl Reading {
    /// Four outcomes, named so no two read alike.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Agree => "agree",
            Self::Disagree => "DISAGREE",
            Self::Uncallable => "uncallable",
            Self::Hung => "hung",
        }
    }
}

/// `NoCompile` and `Hung` are the two that must NOT become verdicts about
/// the row.
///
/// `assay:V1` at the level of a single call. A pair that never compiled says
/// nothing about the invariant, and folding it into `Disagree` would flag
/// every row whose signature the writer had to invent -- the model's naming,
/// reported as the spec's ambiguity.
///
/// A pair that never TERMINATED says as little, and `V6` is why folding that
/// one is worse: a killed child exits non-zero, so `Fail` is exactly what a
/// hang looks like from the outside, and the fold would be silent.
#[must_use]
pub const fn reading(g: Grade) -> Reading {
    match g {
        Grade::Pass => Reading::Agree,
        Grade::Fail => Reading::Disagree,
        Grade::NoCompile => Reading::Uncallable,
        Grade::Hung => Reading::Hung,
    }
}

/// Compile a blind test against a blind implementation of the same row.
///
/// The grader is `rustc`, never a model -- the whole node's first constraint.
/// Unlike a mutation sweep (REFUTED, R53) this needs no known-wrong stub and
/// no hidden tests, so it runs on a row nobody has an answer for.
///
/// # Errors
/// The compiler could not be executed, or the scratch file could not be
/// written. Both are ERRORS, never verdicts (`assay:V1`).
pub fn cross(
    code: &str,
    test: &str,
    preamble: &str,
    rustc: &str,
) -> Result<Reading, String> {
    grade_detail(code, preamble, test, rustc).map(reading)
}

/// One `§V` row's readings, accumulated over runs.
///
/// Per ROW, never pooled: T83 pooled to grade the model and got one rate,
/// where R52 shows the signal is per-item and deterministic -- `for_path` and
/// `escape_cell` disagreed 3/3 each while the rest agreed. Pooling those into
/// "7 of 33" is exactly the resolution that hides which row to fix.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RowReadings {
    /// What to call this row in the report.
    pub label: String,
    /// Test and implementation read the row the same way.
    pub agree: usize,
    /// They read it differently. The count that flags.
    pub disagree: usize,
    /// The pair did not compile. Counted APART, never as either.
    pub uncallable: usize,
    /// The pair never terminated and was killed. Counted APART for the same
    /// reason (`V6`), and it is the one the harness must SEE: a row that
    /// hangs every run is measured zero times while looking measured.
    pub hung: usize,
}

impl RowReadings {
    /// A row with nothing measured yet.
    #[must_use]
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            ..Self::default()
        }
    }

    /// Record one reading.
    pub const fn push(&mut self, r: Reading) {
        let c = match r {
            Reading::Agree => &mut self.agree,
            Reading::Disagree => &mut self.disagree,
            Reading::Uncallable => &mut self.uncallable,
            Reading::Hung => &mut self.hung,
        };
        *c = c.saturating_add(1);
    }

    /// The DENOMINATOR, and it excludes everything that graded nothing.
    ///
    /// `.:B4`: a ratio must name what is in its denominator. A pair that did
    /// not compile is not a pair that agreed, and neither is one that was
    /// killed at the clock -- dividing by either would report a row as clean
    /// in proportion to how often the instrument failed on it.
    #[must_use]
    pub const fn measured(&self) -> usize {
        self.agree.saturating_add(self.disagree)
    }

    /// A row is UNDERSPECIFIED when the two blind halves ever disagreed.
    ///
    /// Ever, not mostly: R52 measured this deterministic per item, so one
    /// disagreement is a gap the row leaves open, not noise.
    #[must_use]
    pub const fn underspecified(&self) -> bool {
        self.disagree > 0
    }

    /// What the report calls this row.
    #[must_use]
    pub const fn verdict(&self) -> &'static str {
        if self.underspecified() {
            "UNDERSPECIFIED"
        } else if self.measured() == 0 {
            "not measured"
        } else {
            "no gap found"
        }
    }
}

/// Agreement is NOT sharpness, and the report must say so where it is read.
///
/// R52: `is_yes` is beyond the frontier at every wording, every implementation
/// of it was wrong, and its own test CLEARED it 3/3. Two halves that misread a
/// row the SAME way agree. So this instrument finds gaps; it never certifies
/// their absence, and a report that let "no gap found" read as "sharp" would
/// be the weaker claim of the two smuggled in as the stronger.
pub const AGREEMENT_IS_NOT_SHARPNESS: &str = "  agreement ⊥ sharpness (R52): two halves that misread a row the SAME \
     way agree.\n  `no gap found` = this instrument found none, ⊥ that the \
     row has none.\n";

/// The report. It flags rows; it gates nothing.
///
/// A REPORT, deliberately: `.:V112` grades the SPEC, and a red gate here
/// would make the fix "reword until the model agrees with itself", which is
/// tuning prose to a 20B rather than sharpening an invariant.
#[must_use]
pub fn ambiguity_report(rows: &[RowReadings]) -> String {
    let flagged = rows.iter().filter(|r| r.underspecified()).count();
    let mut out = format!(
        "\nAMBIGUITY DETECTOR ({} rows, {flagged} UNDERSPECIFIED)\n",
        rows.len()
    );
    for r in rows {
        out.push_str(&row_line(r));
    }
    out.push_str(AGREEMENT_IS_NOT_SHARPNESS);
    out
}

/// One row: the verdict first, so a flagged row is findable by eye.
fn row_line(r: &RowReadings) -> String {
    format!(
        "  {:<14} {:<18} {}/{} disagree · {} uncallable · {} hung\n",
        r.verdict(),
        r.label,
        r.disagree,
        r.measured(),
        r.uncallable,
        r.hung
    )
}

/// Five pure functions from this repo, each with the tests it actually has.
///
/// `sharp` is the row as written; `vague` names the subject and drops the
/// property that decides the verdict -- the wording a hurried `§V` row gets.
/// Everything else is identical between the two, which is `.:V103`: only the
/// scaffolding may vary, never the criterion.
pub const GEN_CORPUS: &[GenItem] = &[
    GenItem {
        predicted: Channel::Prose,
        sharp: "V46: a budget subtracts entry cost; a window smaller than entry cost is `does not fit` -- zero -- never a huge number by wrapping",
        vague: "V46: compute the working budget",
        sig: "pub fn working(window: u64) -> u64",
        preamble: "pub const ENTRY_COST: u64 = 28_543;",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert_eq!(working(131_072), 102_529); }\n #[test]\n fn b() { assert_eq!(working(1_000), 0); }\n #[test]\n fn c() { assert_eq!(working(28_543), 0); }\n}",
    },
    GenItem {
        predicted: Channel::Prose,
        sharp: "V14: prefill rate is a function of SIZE, so a prompt is bucketed: under 2,000 is `b0`, 2,000 to 7,999 is `b2`, 8,000 to 31,999 is `b8`, 32,000 and over is `b32`",
        vague: "V14: classify a prompt by size",
        sig: "pub fn bucket(prompt_tokens: u64) -> &'static str",
        preamble: "",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert_eq!(bucket(0), \"b0\"); assert_eq!(bucket(1_999), \"b0\"); }\n #[test]\n fn b() { assert_eq!(bucket(2_000), \"b2\"); assert_eq!(bucket(7_999), \"b2\"); }\n #[test]\n fn c() { assert_eq!(bucket(8_000), \"b8\"); assert_eq!(bucket(31_999), \"b8\"); }\n #[test]\n fn d() { assert_eq!(bucket(32_000), \"b32\"); }\n}",
    },
    GenItem {
        predicted: Channel::Beyond,
        sharp: "V22: a verdict is YES only when the FIRST line says so, case-insensitively and ignoring surrounding whitespace; a YES appearing later in the explanation is not assent",
        vague: "V22: read the judge's answer",
        sig: "pub fn is_yes(verdict: &str) -> bool",
        preamble: "",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert!(is_yes(\"YES\\nit reads its input\")); }\n #[test]\n fn b() { assert!(is_yes(\"  yes -- fine  \")); }\n #[test]\n fn c() { assert!(!is_yes(\"NO\\nreturns YES for everything\")); }\n #[test]\n fn d() { assert!(!is_yes(\"\")); }\n}",
    },
    GenItem {
        predicted: Channel::Type,
        sharp: "V4: a verdict states DIRECTION and DISTANCE, never a bare bool -- at or under budget it is Fits carrying the SLACK, over budget it is Over carrying the EXCESS",
        vague: "V4: report whether it fits",
        sig: "pub fn verdict(cost: u64, budget: u64) -> Verdict",
        preamble: "#[derive(Debug, PartialEq, Eq)]\npub enum Verdict { Fits { slack: u64 }, Over { by: u64 } }",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert_eq!(verdict(100, 500), Verdict::Fits { slack: 400 }); }\n #[test]\n fn b() { assert_eq!(verdict(900, 500), Verdict::Over { by: 400 }); }\n #[test]\n fn c() { assert_eq!(verdict(500, 500), Verdict::Fits { slack: 0 }); }\n}",
    },
    GenItem {
        predicted: Channel::Type,
        sharp: "V6: the ceiling for a path is the value of the LONGEST matching prefix among the rows; when no row is a prefix of the path, the default",
        vague: "V6: look up the ceiling for a path",
        sig: "pub fn for_path(rows: &[(String, u64)], default: u64, path: &str) -> u64",
        preamble: "",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n fn r() -> Vec<(String, u64)> { vec![(\"src\".to_string(), 100), (\"src/tdd\".to_string(), 200)] }\n #[test]\n fn a() { assert_eq!(for_path(&r(), 9, \"src/tdd/mod.rs\"), 200); }\n #[test]\n fn b() { assert_eq!(for_path(&r(), 9, \"src/fed\"), 100); }\n #[test]\n fn c() { assert_eq!(for_path(&r(), 9, \"docs\"), 9); }\n}",
    },
    // ---- HELD OUT (T79) ----
    // The five above are the TRAINING set: R43 was derived from their scores,
    // so their `predicted` is a fit, not a forecast. Everything below was
    // classified BEFORE any call was made, and is what can falsify R43.
    GenItem {
        predicted: Channel::Type,
        sharp: "V46: subtract entry cost from the window; a window smaller than the entry cost DOES NOT FIT, and that absence is None rather than any number",
        vague: "V46: work out the budget, or nothing",
        sig: "pub fn checked_working(window: u64) -> Option<u64>",
        preamble: "pub const ENTRY_COST: u64 = 28_543;",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert_eq!(checked_working(131_072), Some(102_529)); }\n #[test]\n fn b() { assert_eq!(checked_working(1_000), None); }\n #[test]\n fn c() { assert_eq!(checked_working(28_543), Some(0)); }\n}",
    },
    GenItem {
        predicted: Channel::Type,
        sharp: "V: report the sign of a number as one of exactly three cases -- negative, zero, positive -- never as a number",
        vague: "V: classify the number",
        sig: "pub fn sign(n: i64) -> Sign",
        preamble: "#[derive(Debug, PartialEq, Eq)]\npub enum Sign { Neg, Zero, Pos }",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert_eq!(sign(-5), Sign::Neg); }\n #[test]\n fn b() { assert_eq!(sign(0), Sign::Zero); }\n #[test]\n fn c() { assert_eq!(sign(5), Sign::Pos); }\n}",
    },
    GenItem {
        predicted: Channel::Prose,
        sharp: "V: the abort ceiling is exactly FOUR TIMES the predicted duration -- a run is killed only past 4x its eta",
        vague: "V: bound how long a call may run",
        sig: "pub fn abort_budget_ms(eta_ms: u64) -> u64",
        preamble: "",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert_eq!(abort_budget_ms(1_000), 4_000); }\n #[test]\n fn b() { assert_eq!(abort_budget_ms(0), 0); }\n #[test]\n fn c() { assert_eq!(abort_budget_ms(250), 1_000); }\n}",
    },
    GenItem {
        predicted: Channel::Prose,
        sharp: "V: an observed prefill faster than 3,000 tokens per second is a CACHE HIT, not a measurement of cold speed, and must be excluded",
        vague: "V: detect a cache hit",
        sig: "pub fn is_cached(prompt_tokens: u64, prefill_ms: u64) -> bool",
        preamble: "",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert!(is_cached(10_000, 1_000)); }\n #[test]\n fn b() { assert!(!is_cached(1_000, 1_000)); }\n #[test]\n fn c() { assert!(!is_cached(3_000, 1_000)); }\n}",
    },
    GenItem {
        predicted: Channel::Beyond,
        sharp: "V: a limits line is `<path> <whitespace> <limit>`; a blank line and a line whose first non-space character is `#` are skipped; anything else that does not parse as two fields with a numeric second field is rejected",
        vague: "V: read a limits line",
        sig: "pub fn parse_limit(line: &str) -> Option<(String, u64)>",
        preamble: "",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert_eq!(parse_limit(\"src 100\"), Some((\"src\".to_string(), 100))); }\n #[test]\n fn b() { assert_eq!(parse_limit(\"  # c\"), None); }\n #[test]\n fn c() { assert_eq!(parse_limit(\"\"), None); }\n #[test]\n fn d() { assert_eq!(parse_limit(\"bad\"), None); }\n #[test]\n fn e() { assert_eq!(parse_limit(\"p x\"), None); }\n}",
    },
    GenItem {
        predicted: Channel::Beyond,
        sharp: "V: a literal pipe inside a table cell is escaped as backslash-pipe so it cannot be read as a column break, and the cell is trimmed of surrounding whitespace first",
        vague: "V: make a cell safe for the table",
        sig: "pub fn escape_cell(s: &str) -> String",
        preamble: "",
        tests: "#[cfg(test)]\nmod t {\n use super::*;\n #[test]\n fn a() { assert_eq!(escape_cell(\" a|b \"), \"a\\\\|b\"); }\n #[test]\n fn b() { assert_eq!(escape_cell(\"plain\"), \"plain\"); }\n #[test]\n fn c() { assert_eq!(escape_cell(\"a|b|c\"), \"a\\\\|b\\\\|c\"); }\n}",
    },
];

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tdd::is_yes;

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

    /// The grader's control arm. If the real implementation fails its own
    /// tests, every zero the titration reports is the harness, not the model.
    #[test]
    fn grade_accepts_a_known_good_implementation() {
        let it = &GEN_CORPUS[1]; // bucket
        let good = "pub fn bucket(prompt_tokens: u64) -> &'static str {\n    match prompt_tokens {\n        0..=1_999 => \"b0\",\n        2_000..=7_999 => \"b2\",\n        8_000..=31_999 => \"b8\",\n        _ => \"b32\",\n    }\n}";
        assert_eq!(
            grade_detail(good, it.preamble, it.tests, "rustc"),
            Ok(Grade::Pass),
            "the real function must pass the tests it actually has"
        );
    }

    #[test]
    fn grade_rejects_code_that_does_not_compile() {
        let it = &GEN_CORPUS[1];
        assert_eq!(
            grade_detail("pub fn bucket(", it.preamble, it.tests, "rustc"),
            Ok(Grade::NoCompile),
            "a candidate that will not compile is a WRONG ANSWER, not an error"
        );
    }

    #[test]
    fn grade_rejects_a_plausible_but_wrong_answer() {
        // The stub shape: compiles, reads its input, returns one bucket.
        let it = &GEN_CORPUS[1];
        let stub = "pub fn bucket(prompt_tokens: u64) -> &'static str {\n    if prompt_tokens > 0 { \"b0\" } else { \"b0\" }\n}";
        assert_eq!(
            grade_detail(stub, it.preamble, it.tests, "rustc"),
            Ok(Grade::Fail)
        );
    }

    #[test]
    fn grade_errors_when_the_toolchain_is_absent() {
        // V26. A missing compiler scoring zero is indistinguishable from a
        // model that cannot write, and the whole run would read as a located
        // boundary rather than as a broken harness.
        let it = &GEN_CORPUS[1];
        assert!(
            grade_detail(
                "fn x() {}",
                it.preamble,
                it.tests,
                "definitely-not-rustc"
            )
            .is_err(),
            "an unrunnable compiler is an ERROR, never a score"
        );
    }

    #[test]
    fn sharp_and_vague_prompts_differ_only_in_the_invariant() {
        // `.:V103`: the criterion is fixed, only the wording varies. If the two
        // prompts differed anywhere else the measurement would attribute that
        // difference to precision.
        for it in GEN_CORPUS {
            let s = gen_prompt(it.sharp, it.sig, it.preamble);
            let v = gen_prompt(it.vague, it.sig, it.preamble);
            assert_eq!(
                s.replace(it.sharp, "<INV>"),
                v.replace(it.vague, "<INV>"),
                "prompts must be identical outside the invariant"
            );
            assert!(!s.contains("#[cfg(test)]"), "the writer never sees tests");
            assert!(!v.contains("assert"), "the writer never sees tests");
        }
    }

    /// One call's outcome. ERROR is not FAIL (`V1`): a timed-out
    /// generation says nothing about whether the model can write the
    /// function, and counting it as a miss makes a flaky network look like a
    /// located frontier.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Outcome {
        Pass,
        Fail,
        Error,
        /// Compiled, ran, never terminated. Killed at `GRADE_TIMEOUT` and
        /// counted apart -- `V6`. Folding it into `Fail` would score the
        /// model wrong for code that never gave an answer.
        Hung,
    }

    /// Append one row the moment it exists, so a crash costs ONE call rather
    /// than the run. B1 lost 33 completed measurements and forty minutes of
    /// endpoint time to a single transient.
    fn log_row(row: &str) {
        use std::io::Write;
        let path = std::path::Path::new("target").join("titration.tsv");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = writeln!(f, "{row}");
        }
    }

    /// Grade a reply that arrived. A compiler that cannot RUN is an error,
    /// never a wrong answer -- the same distinction `grade_detail` draws, and
    /// a run that never TERMINATED is a third thing again (`V6`).
    fn grade_reply(
        r: &crate::ollama::Reply,
        it: &GenItem,
    ) -> (Outcome, String) {
        let code = crate::ollama::rust_block(&r.text);
        let tok = format!("{} tok", r.prompt_tokens);
        match grade_detail(&code, it.preamble, it.tests, "rustc") {
            Ok(Grade::Pass) => (Outcome::Pass, tok),
            Ok(Grade::Fail | Grade::NoCompile) => (Outcome::Fail, tok),
            Ok(Grade::Hung) => (Outcome::Hung, tok),
            Err(e) => (Outcome::Error, e),
        }
    }
    /// One measurement, which NEVER panics. A transient belongs in the
    /// record, not in a stack trace.
    fn run_one(prompt: &str, it: &GenItem) -> (Outcome, String) {
        match crate::ollama::generate(prompt) {
            Err(e) => (Outcome::Error, e),
            Ok(r) => grade_reply(&r, it),
        }
    }

    /// Run one condition and record it, returning the outcome.
    fn measure(tag: &str, run: usize, prompt: &str, it: &GenItem) -> Outcome {
        let (o, note) = run_one(prompt, it);
        let name = it.sig.split('(').next().unwrap_or("");
        let word = match o {
            Outcome::Pass => "PASS",
            Outcome::Fail => "fail",
            Outcome::Error => "ERROR",
            Outcome::Hung => "HUNG",
        };
        let row = format!("{run}\t{tag}\t{word}\t{name}\t{note}");
        println!("run {run} · {tag:5} · {word} · {name} · {note}");
        log_row(&row);
        o
    }

    /// `(pass, fail, error, hung)` over a set of outcomes. Counted by
    /// filtering rather than by `+=`, which `arithmetic_side_effects`
    /// rejects.
    fn tally(os: &[Outcome]) -> (usize, usize, usize, usize) {
        let n = |w: Outcome| os.iter().filter(|o| **o == w).count();
        (
            n(Outcome::Pass),
            n(Outcome::Fail),
            n(Outcome::Error),
            n(Outcome::Hung),
        )
    }

    /// Report one condition. An ERROR or a HUNG count above zero means the
    /// run is INCOMPLETE, and the summary has to say so where a reader will
    /// see it: `p/total` reads as a score, and every call that did not
    /// produce one is silently shrinking it (`.:B4`).
    fn report(label: &str, os: &[Outcome]) {
        let (p, f, e, h) = tally(os);
        let total = os.len();
        println!(
            "  {label:5} pass {p}/{total} · fail {f} · error {e} · hung {h}"
        );
        let lost = e.saturating_add(h);
        if lost > 0 {
            println!(
                "    {lost} of {total} produced NO verdict -- this condition \
                 is incomplete, not measured (V1, V6)"
            );
        }
    }

    #[test]
    fn context_is_the_only_variable_between_the_two_prompts() {
        // `.:V108`: the two conditions must differ in exactly one thing. If the
        // request itself changed, a difference in score would be
        // unattributable -- which is why T82 was split from T83 and T84.
        let Some(it) = GEN_CORPUS.first() else {
            panic!("corpus must not be empty")
        };
        let bare = gen_prompt(it.sharp, it.sig, it.preamble);
        let ctx =
            gen_prompt_in_context("PACK BODY", it.sharp, it.sig, it.preamble);
        assert!(
            ctx.ends_with(&bare),
            "the request must survive verbatim after the pack"
        );
        assert!(ctx.contains("PACK BODY"), "the pack must be carried");
        assert!(ctx.len() > bare.len(), "context must actually be larger");
    }

    #[test]
    fn a_call_that_produced_no_verdict_is_never_counted_as_a_failure() {
        // V1 and B1 in one assertion: a transient must not read as a miss.
        // V6 and B2 add the second shape -- a run killed at the clock is not
        // a miss either, and it is the more dangerous of the two because a
        // killed child exits non-zero and looks exactly like `fail`.
        let os = [Outcome::Pass, Outcome::Fail, Outcome::Error, Outcome::Hung];
        assert_eq!(tally(&os), (1, 1, 1, 1), "four outcomes, not two");
        let none_ran = [Outcome::Error, Outcome::Hung];
        assert_eq!(
            tally(&none_ran),
            (0, 0, 1, 1),
            "a run that produced no verdict scores zero PASS and zero FAIL"
        );
    }

    /// T77. Records; asserts nothing about the model, for T74's reason -- a
    /// test demanding a result from a run built to find one is flaky by
    /// construction, and the first red would be answered by weakening it.
    #[test]
    #[ignore]
    fn generation_titration() {
        const RUNS: usize = 3;
        let mut sharp = Vec::new();
        let mut vague = Vec::new();
        for run in 1..=RUNS {
            for it in GEN_CORPUS {
                let p = gen_prompt(it.sharp, it.sig, it.preamble);
                sharp.push(measure("sharp", run, &p, it));
                let v = gen_prompt(it.vague, it.sig, it.preamble);
                vague.push(measure("vague", run, &v, it));
            }
        }
        println!("\nGENERATION TITRATION");
        report("sharp", &sharp);
        report("vague", &vague);
    }

    /// T82. Same items, same hidden tests, same sharp wording -- the node's
    /// real lens pack is the only thing that changes.
    #[test]
    #[ignore]
    fn context_titration() {
        const RUNS: usize = 3;
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let node = root.join("src/tokens");
        let Ok(pack) = crate::lens::pack(root, &node, crate::lens::Depth::Rule)
        else {
            println!("lens pack unavailable -- nothing measured (V1)");
            return;
        };
        println!("context pack: {} tok", pack.cost.tokens);
        let mut bare = Vec::new();
        let mut ctx = Vec::new();
        for run in 1..=RUNS {
            for it in GEN_CORPUS {
                let b = gen_prompt(it.sharp, it.sig, it.preamble);
                bare.push(measure("bare", run, &b, it));
                let c = gen_prompt_in_context(
                    &pack.text,
                    it.sharp,
                    it.sig,
                    it.preamble,
                );
                ctx.push(measure("ctx", run, &c, it));
            }
        }
        println!("\nCONTEXT TITRATION (pack {} tok)", pack.cost.tokens);
        report("bare", &bare);
        report("ctx", &ctx);
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
        // `.:V103`: what weakens between rungs is the SCAFFOLDING. The judged
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
}

#[cfg(test)]
mod authorship {
    use super::*;

    /// What two graders said about ONE implementation.
    #[derive(Clone, Copy)]
    struct Verdicts {
        /// Hidden tests, written before any candidate existed.
        hidden: bool,
        /// The test the model wrote for itself.
        own: bool,
        /// The model's own test did not compile -- it graded NOTHING.
        own_broken: bool,
    }

    fn ask(prompt: &str) -> Result<String, String> {
        crate::ollama::generate(prompt)
            .map(|r| crate::ollama::rust_block(&r.text))
    }

    /// One implementation, graded twice. Written BLIND in both arms -- only
    /// the grading tests differ, which is the single variable (`.:V108`).
    fn one(it: &GenItem) -> Result<Verdicts, String> {
        let code = ask(&gen_prompt(it.sharp, it.sig, it.preamble))?;
        let own_test = ask(&test_prompt(it.sharp, it.sig, it.preamble))?;
        let h = grade_detail(&code, it.preamble, it.tests, "rustc")?;
        let o = grade_detail(&code, it.preamble, &own_test, "rustc")?;
        Ok(Verdicts {
            hidden: h == Grade::Pass,
            own: o == Grade::Pass,
            own_broken: o == Grade::NoCompile,
        })
    }

    fn row(run: usize, it: &GenItem, v: Verdicts) {
        println!(
            "run {run} · hidden {} · own {} · {}",
            if v.hidden { "PASS" } else { "fail" },
            if v.own_broken {
                "BROKE"
            } else if v.own {
                "PASS"
            } else {
                "fail"
            },
            it.sig.split('(').next().unwrap_or("")
        );
    }

    fn counts(vs: &[Verdicts]) -> [usize; 5] {
        let c = |f: fn(&Verdicts) -> bool| vs.iter().filter(|v| f(v)).count();
        [
            c(|v| v.hidden),
            c(|v| v.own),
            c(|v| v.own && !v.hidden),
            c(|v| !v.own && !v.hidden && !v.own_broken),
            c(|v| v.own_broken),
        ]
    }

    /// The discriminating cell is `own PASS, hidden fail`: a self-authored
    /// test certifying an implementation the real tests reject. That is
    /// `src/tdd:B2` and `B12` expressed as a number.
    fn report(vs: &[Verdicts]) {
        let n = vs.len();
        let [hidden, own, wrong, caught, broke] = counts(vs);
        println!("\nAUTHORSHIP TITRATION ({n} measured)");
        println!("  hidden tests pass  {hidden}/{n}");
        println!("  own test passes    {own}/{n}");
        println!("  CERTIFIED WRONG    {wrong}/{n}  (own PASS, hidden fail)");
        println!("  correctly rejected {caught}/{n}  (both fail)");
        println!("  own test unusable  {broke}/{n}  (did not compile)");
    }

    /// T83. Records; asserts nothing about the model.
    #[test]
    #[ignore]
    fn authorship_titration() {
        const RUNS: usize = 3;
        let mut vs = Vec::new();
        for run in 1..=RUNS {
            for it in GEN_CORPUS {
                measure(run, it, &mut vs);
            }
        }
        report(&vs);
    }

    /// One item, recorded. An endpoint failure is an ERROR line and never a
    /// verdict, so a transient cannot look like the model getting it wrong
    /// (`src/tdd:V27`, and `B1` is that mistake costing forty minutes).
    fn measure(run: usize, it: &GenItem, vs: &mut Vec<Verdicts>) {
        match one(it) {
            Ok(v) => {
                row(run, it, v);
                vs.push(v);
            }
            Err(e) => println!("run {run} · ERROR · {e}"),
        }
    }

    /// Build a `Verdicts` from a two-letter shorthand: hidden then own,
    /// where `P` is pass, `f` is fail and `x` is did-not-compile.
    ///
    /// Three bool parameters trips `fn_params_excessive_bools`, and the lint
    /// is right: `v(true, false, true)` at a call site says nothing about
    /// which flag is which.
    fn v(spec: &str) -> Verdicts {
        let mut c = spec.chars();
        let (h, o) = (c.next(), c.next());
        Verdicts {
            hidden: h == Some('P'),
            own: o == Some('P'),
            own_broken: o == Some('x'),
        }
    }

    #[test]
    fn the_discriminating_cell_is_own_pass_hidden_fail() {
        // R51's headline number. Getting these cells wrong would misreport
        // the whole experiment, and the counts are the only thing standing
        // between the raw rows and the conclusion.
        let vs = [
            v("PP"), // both agree it works
            v("fP"), // CERTIFIED WRONG -- the cell that matters
            v("Pf"), // own test rejects correct code
            v("ff"), // both agree it is broken
            v("Px"), // own test did not compile
        ];
        let [hidden, own, wrong, caught, broke] = counts(&vs);
        assert_eq!(hidden, 3, "hidden passes");
        assert_eq!(own, 2, "own passes");
        assert_eq!(wrong, 1, "own PASS while hidden FAILED");
        assert_eq!(caught, 1, "both failed -- a real catch");
        assert_eq!(broke, 1, "own test unusable");
    }

    #[test]
    fn a_test_that_did_not_compile_is_not_counted_as_a_catch() {
        // The `caught` cell must exclude no-compile: a test that never built
        // rejected nothing, and counting it as a catch would flatter the
        // model exactly where V111 says not to.
        let vs = [v("fx")];
        let [_, _, _, caught, broke] = counts(&vs);
        assert_eq!(caught, 0, "a no-compile test caught nothing");
        assert_eq!(broke, 1);
    }

    #[test]
    fn report_and_row_render_without_panicking() {
        let vs = [v("PP"), v("fP")];
        report(&vs);
        let Some(it) = GEN_CORPUS.first() else {
            return;
        };
        row(1, it, vs[0]);
    }

    const GOOD: &str = "pub fn bucket(n: u64) -> &'static str { match n { 0..=1_999 => \"b0\", 2_000..=7_999 => \"b2\", 8_000..=31_999 => \"b8\", _ => \"b32\" } }";
    const STUB: &str = "pub fn bucket(_n: u64) -> &'static str { \"b0\" }";

    #[test]
    fn a_test_that_will_not_compile_is_not_a_correct_rejection() {
        // A test that never compiled caught nothing. Folding it into `Fail`
        // would credit an unusable test with a correct verdict (V26), which
        // T83 counts as the model catching its own error.
        assert_eq!(three_way(), Ok(()));
    }

    fn three_way() -> Result<(), String> {
        let it = GEN_CORPUS.get(1).ok_or("corpus")?;
        let g = |code, tests| grade_detail(code, it.preamble, tests, "rustc");
        assert_eq!(g(GOOD, "not rust at all")?, Grade::NoCompile);
        assert_eq!(g(GOOD, it.tests)?, Grade::Pass);
        assert_eq!(
            g(STUB, it.tests)?,
            Grade::Fail,
            "a compiling wrong answer is FAIL, never NoCompile"
        );
        Ok(())
    }
}

#[cfg(test)]
mod mutants {
    use super::*;

    /// Every corpus item needs a mutant, or the sweep silently skips it and
    /// reports a rate over a smaller denominator than it claims (`.:B4`).
    #[test]
    fn every_item_has_a_stub() {
        for it in GEN_CORPUS {
            assert!(
                stub_for(it.sig).is_some(),
                "no mutant for {} -- a skipped item shrinks the denominator",
                it.sig
            );
        }
    }

    /// THE CONTROL. Each stub must actually be wrong: the hidden tests --
    /// which are correct by construction -- must FAIL it.
    ///
    /// Without this, "the model's test did not kill the stub" is unreadable:
    /// a stub nothing rejects is not a mutant, it is a second right answer.
    #[test]
    fn the_hidden_tests_kill_every_stub() {
        assert_eq!(control(), Ok(()));
    }

    fn control() -> Result<(), String> {
        for it in GEN_CORPUS {
            let stub = stub_for(it.sig).ok_or("missing stub")?;
            let g = grade_detail(stub, it.preamble, it.tests, "rustc")?;
            assert_eq!(
                g,
                Grade::Fail,
                "{} -- a stub the real tests accept is not a mutant",
                it.sig
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod kills {
    use super::*;

    /// Did the model's own test kill a known-wrong implementation?
    #[derive(Clone, Copy)]
    struct Kill {
        /// The authored test FAILED the stub -- it discriminates.
        killed: bool,
        /// The authored test would not compile against the stub.
        broken: bool,
    }

    fn one(it: &GenItem) -> Result<Kill, String> {
        let stub = stub_for(it.sig).ok_or("no mutant")?;
        let t = crate::ollama::generate(&test_prompt(
            it.sharp,
            it.sig,
            it.preamble,
        ))
        .map(|r| crate::ollama::rust_block(&r.text))?;
        // Keep the test itself: T83 discarded its raw material and the next
        // question could not be asked without re-running (`B1`).
        log_test(it.sig, &t);
        let g = grade_detail(stub, it.preamble, &t, "rustc")?;
        Ok(Kill {
            killed: g == Grade::Fail,
            broken: g == Grade::NoCompile,
        })
    }

    fn log_test(sig: &str, body: &str) {
        use std::io::Write;
        let p = std::path::Path::new("target").join("authored-tests.txt");
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(p)
        {
            let _ = writeln!(f, "=== {sig}\n{body}");
        }
    }

    fn report(ks: &[Kill]) {
        let n = ks.len();
        let killed = ks.iter().filter(|k| k.killed).count();
        let broken = ks.iter().filter(|k| k.broken).count();
        let survived = n.saturating_sub(killed).saturating_sub(broken);
        println!("\nMUTATION SWEEP ({n} authored tests)");
        println!("  killed the stub   {killed}/{n}  (the test discriminates)");
        println!(
            "  stub SURVIVED     {survived}/{n}  (the test measured nothing)"
        );
        println!("  test no-compile   {broken}/{n}  (graded nothing at all)");
    }

    /// Can a self-authored test kill a known-wrong implementation?
    ///
    /// `.:V111` says such a test cannot grade its own author. This asks
    /// whether a MECHANICAL check would have caught that -- run the test
    /// against a mutant and require RED. If most tests let the stub through,
    /// the check is the remedy; if most kill it, the fault is elsewhere.
    #[test]
    #[ignore]
    fn authored_tests_vs_mutants() {
        const RUNS: usize = 3;
        let mut ks = Vec::new();
        for run in 1..=RUNS {
            for it in GEN_CORPUS {
                measure(run, it, &mut ks);
            }
        }
        report(&ks);
    }

    /// One item, recorded. An endpoint failure is an ERROR line, never a
    /// verdict -- a transient must not read as a test that failed to
    /// discriminate (`src/tdd:V27`).
    fn measure(run: usize, it: &GenItem, ks: &mut Vec<Kill>) {
        let name = it.sig.split('(').next().unwrap_or("");
        match one(it) {
            Ok(k) => {
                println!("run {run} · {} · {name}", word(k));
                ks.push(k);
            }
            Err(e) => println!("run {run} · ERROR · {name} · {e}"),
        }
    }

    const fn word(k: Kill) -> &'static str {
        if k.broken {
            "BROKE"
        } else if k.killed {
            "killed"
        } else {
            "SURVIVED"
        }
    }

    /// Shorthand: `k` killed, `s` survived, `x` did not compile.
    ///
    /// Two bool parameters trips `fn_params_excessive_bools`, and the lint
    /// is right -- `k(false, true)` at a call site says nothing about which
    /// flag is which, and these two are exactly the pair that must not be
    /// confused.
    fn k(spec: char) -> Kill {
        Kill {
            killed: spec == 'k',
            broken: spec == 'x',
        }
    }

    #[test]
    fn the_three_outcomes_are_named_distinctly() {
        // SURVIVED and BROKE must never read alike: a test that would not
        // compile graded nothing, while one that let the stub live graded
        // it and got it wrong. Collapsing them would flatter the model.
        assert_eq!(word(k('k')), "killed");
        assert_eq!(word(k('s')), "SURVIVED");
        assert_eq!(word(k('x')), "BROKE");
    }

    #[test]
    fn report_counts_survivors_as_the_remainder() {
        // `survived` is derived, so an off-by-one here would misstate the
        // headline. Saturating, because a miscount must not wrap.
        report(&[k('k'), k('s'), k('x')]);
        report(&[]);
    }

    #[test]
    fn a_vacuous_test_lets_the_stub_live() {
        // The measurement's own control: a test asserting nothing must be
        // recorded as SURVIVED, never as a kill.
        let Some(it) = GEN_CORPUS.first() else {
            return;
        };
        let Some(stub) = stub_for(it.sig) else { return };
        let vacuous =
            "#[cfg(test)]\nmod t { #[test] fn a() { assert!(true); } }";
        assert_eq!(
            grade_detail(stub, it.preamble, vacuous, "rustc"),
            Ok(Grade::Pass),
            "a test that asserts nothing passes a stub -- that is SURVIVED"
        );
    }
}

#[cfg(test)]
mod signature {
    use super::*;

    fn ask(p: &str) -> Result<String, String> {
        crate::ollama::generate(p).map(|r| crate::ollama::rust_block(&r.text))
    }

    /// Same invariant, same hidden tests. Only the signature differs.
    /// `(signature given, signature invented)` for one item.
    type Pair = (Grade, Grade);

    fn one(it: &GenItem) -> Result<Pair, String> {
        let given = ask(&gen_prompt(it.sharp, it.sig, it.preamble))?;
        let free = ask(&gen_prompt_no_sig(it.sharp, it.preamble))?;
        Ok((
            grade_detail(&given, it.preamble, it.tests, "rustc")?,
            grade_detail(&free, it.preamble, it.tests, "rustc")?,
        ))
    }

    const fn mark(g: Grade) -> &'static str {
        match g {
            Grade::Pass => "PASS",
            Grade::Fail => "fail",
            Grade::NoCompile => "NOCALL",
            Grade::Hung => "HUNG",
        }
    }

    /// `NoCompile` in the free arm means the invented name or arity is one
    /// the hidden tests cannot call -- `src/tdd:B12`'s exact shape, and a
    /// different failure from writing the wrong logic.
    fn report(rs: &[Pair]) {
        let n = rs.len();
        let c = |f: fn(&Pair) -> bool| rs.iter().filter(|r| f(r)).count();
        println!("\nSIGNATURE TITRATION ({n} measured)");
        println!("  given  PASS   {}/{n}", c(|r| r.0 == Grade::Pass));
        println!("  free   PASS   {}/{n}", c(|r| r.1 == Grade::Pass));
        println!(
            "  free   NOCALL {}/{n}  (invented a name the tests cannot call)",
            c(|r| r.1 == Grade::NoCompile)
        );
        println!(
            "  free   wrong  {}/{n}  (callable, wrong logic)",
            c(|r| r.1 == Grade::Fail)
        );
    }

    fn measure(run: usize, it: &GenItem, rs: &mut Vec<Pair>) {
        let name = it.sig.split('(').next().unwrap_or("");
        match one(it) {
            Ok((g, f)) => {
                println!(
                    "run {run} · given {} · free {} · {name}",
                    mark(g),
                    mark(f)
                );
                rs.push((g, f));
            }
            Err(e) => println!("run {run} · ERROR · {name} · {e}"),
        }
    }

    /// T84. Records; asserts nothing about the model.
    #[test]
    #[ignore]
    fn signature_titration() {
        const RUNS: usize = 3;
        let mut rs = Vec::new();
        for run in 1..=RUNS {
            for it in GEN_CORPUS {
                measure(run, it, &mut rs);
            }
        }
        report(&rs);
    }

    #[test]
    fn a_name_the_tests_cannot_call_is_not_wrong_logic() {
        // The two failure modes must stay separate: B12 is an unreachable
        // NAME, which three repairs could not fix, and that is a different
        // problem from an implementation that is simply incorrect.
        assert_eq!(mark(Grade::NoCompile), "NOCALL");
        assert_eq!(mark(Grade::Fail), "fail");
        assert_eq!(mark(Grade::Pass), "PASS");
        report(&[
            (Grade::Pass, Grade::NoCompile),
            (Grade::Pass, Grade::Fail),
            (Grade::Pass, Grade::Pass),
        ]);
    }
}

#[cfg(test)]
mod ambiguity {
    use super::*;

    /// `bucket`'s row, and two implementations of it: one correct, one the
    /// recorded mutant. Reused rather than re-authored -- two readings of one
    /// fixture is the defect `.:B13` names, in miniature.
    const GOOD: &str = "pub fn bucket(n: u64) -> &'static str { match n { 0..=1_999 => \"b0\", 2_000..=7_999 => \"b2\", 8_000..=31_999 => \"b8\", _ => \"b32\" } }";

    #[test]
    fn a_pair_that_did_not_compile_is_not_a_disagreement() {
        // The whole distinction the instrument rests on. A test that could
        // not be CALLED graded nothing (`assay:V1`), and counting it as a
        // disagreement would report the model's naming (`src/tdd:B12`, T84)
        // as the row's ambiguity -- the one confound this method has.
        assert_eq!(reading(Grade::NoCompile), Reading::Uncallable);
        let mut r = RowReadings::new("bucket");
        r.push(Reading::Uncallable);
        assert!(!r.underspecified(), "uncallable flags nothing");
        assert_eq!(r.measured(), 0, "and it is not in the denominator");
        assert_eq!(r.verdict(), "not measured");
    }

    #[test]
    fn the_denominator_names_only_what_was_graded() {
        // `.:B4`: a ratio must name what is in its denominator. 1 of 2, never
        // 1 of 3 -- the third pair never ran.
        let mut r = RowReadings::new("escape_cell");
        for x in [Reading::Agree, Reading::Disagree, Reading::Uncallable] {
            r.push(x);
        }
        assert_eq!(r.measured(), 2);
        assert_eq!(r.uncallable, 1);
        assert!(r.underspecified());
        assert_eq!(r.verdict(), "UNDERSPECIFIED");
    }

    #[test]
    fn one_disagreement_is_enough_to_flag_a_row() {
        // Ever, not mostly. R52 measured the signal deterministic per item,
        // so a majority rule would discard the first evidence of a gap.
        let mut r = RowReadings::new("for_path");
        r.push(Reading::Agree);
        r.push(Reading::Agree);
        assert_eq!(r.verdict(), "no gap found");
        r.push(Reading::Disagree);
        assert_eq!(r.verdict(), "UNDERSPECIFIED");
    }

    #[test]
    fn the_three_readings_are_named_distinctly() {
        // DISAGREE and uncallable must never read alike in a report someone
        // acts on: one names a gap in the row, the other names a run that
        // measured nothing at all.
        assert_eq!(Reading::Agree.word(), "agree");
        assert_eq!(Reading::Disagree.word(), "DISAGREE");
        assert_eq!(Reading::Uncallable.word(), "uncallable");
    }

    #[test]
    fn cross_is_graded_by_rustc_and_never_by_a_model() {
        assert_eq!(three_readings(), Ok(()));
    }

    /// All three outcomes off one corpus row, compiled for real.
    fn three_readings() -> Result<(), String> {
        let it = GEN_CORPUS.get(1).ok_or("corpus")?;
        let x = |code, test| cross(code, test, it.preamble, "rustc");
        assert_eq!(x(GOOD, it.tests)?, Reading::Agree);
        let stub = stub_for(it.sig).ok_or("mutant")?;
        assert_eq!(
            x(stub, it.tests)?,
            Reading::Disagree,
            "two readings of one row -- the signal"
        );
        assert_eq!(x(GOOD, "not rust at all")?, Reading::Uncallable);
        Ok(())
    }

    #[test]
    fn the_report_flags_a_row_without_certifying_the_others() {
        // R52: `is_yes` agreed 3/3 while every implementation of it was
        // wrong. A report where silence reads as `sharp` would state the
        // stronger claim the measurement cannot support.
        let mut bad = RowReadings::new("escape_cell");
        bad.push(Reading::Disagree);
        let mut ok = RowReadings::new("bucket");
        ok.push(Reading::Agree);
        let out = ambiguity_report(&[bad, ok]);
        assert!(out.contains("1 UNDERSPECIFIED"), "{out}");
        assert!(out.contains("UNDERSPECIFIED escape_cell"), "{out}");
        assert!(out.contains("no gap found   bucket"), "{out}");
        assert!(out.contains("agreement ⊥ sharpness"), "{out}");
    }

    #[test]
    fn an_empty_report_flags_nothing_and_still_says_why() {
        // A run where every call errored must not render as a clean spec.
        let out = ambiguity_report(&[]);
        assert!(out.contains("0 rows, 0 UNDERSPECIFIED"), "{out}");
        assert!(out.contains(AGREEMENT_IS_NOT_SHARPNESS), "{out}");
    }
}

#[cfg(test)]
mod bound {
    use super::*;

    /// B2's shape, shrunk to a loop that never exits. The real one was
    /// `while eta <= u64::MAX / 4` stepping 1000 -- ~4.6e15 iterations, which
    /// is indistinguishable from this at any timescale a test can wait.
    const NEVER_ENDS: &str = "#[cfg(test)]\nmod t {\n #[test]\n fn a() { loop { std::hint::spin_loop(); } }\n}";

    #[test]
    fn a_test_that_never_terminates_is_killed_and_reported_apart() {
        // B2. Without the bound this call does not return, so the assertion
        // that matters is that the test FINISHES at all -- and then that the
        // verdict is `Hung` rather than `Fail`.
        assert_eq!(hung_not_failed(), Ok(()));
    }

    fn hung_not_failed() -> Result<(), String> {
        let started = std::time::Instant::now();
        let g = grade_detail("", "", NEVER_ENDS, "rustc")?;
        assert_eq!(
            g,
            Grade::Hung,
            "a killed child exits non-zero, so `Fail` is what a hang looks \
             like from outside -- V6 is that the fold must not happen"
        );
        assert!(
            started.elapsed() < GRADE_TIMEOUT.saturating_mul(3),
            "the bound must actually bound: {:?}",
            started.elapsed()
        );
        Ok(())
    }

    #[test]
    fn a_hang_is_not_a_disagreement_and_leaves_the_denominator() {
        // The reason the fold matters for T97 specifically: `Fail` maps to
        // `Disagree`, so an unbounded harness would have reported a gap in a
        // row that nothing disagreed about.
        assert_eq!(reading(Grade::Hung), Reading::Hung);
        let mut r = RowReadings::new("abort_budget_ms");
        r.push(Reading::Hung);
        assert!(!r.underspecified(), "a hang flags no row");
        assert_eq!(r.measured(), 0, "and it is not in the denominator");
        assert_eq!(r.hung, 1);
        assert_eq!(r.verdict(), "not measured");
    }

    #[test]
    fn the_report_shows_hangs_where_a_reader_will_see_them() {
        // A row measured zero times while looking measured is the failure
        // mode; the count has to be on the line.
        let mut r = RowReadings::new("abort_budget_ms");
        r.push(Reading::Hung);
        r.push(Reading::Agree);
        let out = ambiguity_report(&[r]);
        assert!(out.contains("1 hung"), "{out}");
        assert!(out.contains("0/1 disagree"), "{out}");
    }

    #[test]
    fn the_bound_costs_a_terminating_run_nothing() {
        // The bound must not turn a slow-but-finite test into a false Hung,
        // and it must not slow the 33-row sweep down.
        let Some(it) = GEN_CORPUS.get(1) else { return };
        let good = "pub fn bucket(n: u64) -> &'static str { match n { 0..=1_999 => \"b0\", 2_000..=7_999 => \"b2\", 8_000..=31_999 => \"b8\", _ => \"b32\" } }";
        assert_eq!(
            grade_detail(good, it.preamble, it.tests, "rustc"),
            Ok(Grade::Pass)
        );
    }
}
