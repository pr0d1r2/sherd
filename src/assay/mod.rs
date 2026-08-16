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

/// Compile a candidate against tests it never saw, and run them.
///
/// The grader is `rustc`, never a model. A model grader would confound this
/// twice: `.:R40` measured the judge as itself precision-sensitive, and
/// `src/tdd:B2` is a judge loosening under pressure.
///
/// Code that does not COMPILE is a wrong answer, so `Ok(false)`. A toolchain
/// that could not RUN is an error, so `Err` -- `.:V26`: a missing compiler
/// scoring zero is indistinguishable from a model that cannot write, and the
/// whole measurement would read as a located boundary.
///
/// # Errors
/// The compiler could not be executed, or the scratch file could not be
/// written.
pub fn grade(
    candidate: &str,
    preamble: &str,
    tests: &str,
    rustc: &str,
) -> Result<bool, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir();
    let src = dir.join(format!("bbx_gen_{}_{n}.rs", std::process::id()));
    let bin = dir.join(format!("bbx_gen_{}_{n}", std::process::id()));
    std::fs::write(&src, format!("{preamble}\n{candidate}\n{tests}\n"))
        .map_err(|e| format!("scratch write: {e}"))?;
    let out = Command::new(rustc)
        .args(["--test", "--edition", "2021", "-A", "warnings"])
        .arg(&src)
        .arg("-o")
        .arg(&bin)
        .output()
        .map_err(|e| format!("{rustc} could not run: {e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&src);
        return Ok(false);
    }
    let run = Command::new(&bin)
        .output()
        .map_err(|e| format!("compiled binary could not run: {e}"))?;
    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&bin);
    Ok(run.status.success())
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
            grade(good, it.preamble, it.tests, "rustc"),
            Ok(true),
            "the real function must pass the tests it actually has"
        );
    }

    #[test]
    fn grade_rejects_code_that_does_not_compile() {
        let it = &GEN_CORPUS[1];
        assert_eq!(
            grade("pub fn bucket(", it.preamble, it.tests, "rustc"),
            Ok(false),
            "a candidate that will not compile is a WRONG ANSWER, not an error"
        );
    }

    #[test]
    fn grade_rejects_a_plausible_but_wrong_answer() {
        // The stub shape: compiles, reads its input, returns one bucket.
        let it = &GEN_CORPUS[1];
        let stub = "pub fn bucket(prompt_tokens: u64) -> &'static str {\n    if prompt_tokens > 0 { \"b0\" } else { \"b0\" }\n}";
        assert_eq!(grade(stub, it.preamble, it.tests, "rustc"), Ok(false));
    }

    #[test]
    fn grade_errors_when_the_toolchain_is_absent() {
        // V26. A missing compiler scoring zero is indistinguishable from a
        // model that cannot write, and the whole run would read as a located
        // boundary rather than as a broken harness.
        let it = &GEN_CORPUS[1];
        assert!(
            grade("fn x() {}", it.preamble, it.tests, "definitely-not-rustc")
                .is_err(),
            "an unrunnable compiler is an ERROR, never a score"
        );
    }

    #[test]
    fn sharp_and_vague_prompts_differ_only_in_the_invariant() {
        // V103: the criterion is fixed, only the wording varies. If the two
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

    /// One call's outcome. ERROR is not FAIL (`src/tdd:V27`): a timed-out
    /// generation says nothing about whether the model can write the
    /// function, and counting it as a miss makes a flaky network look like a
    /// located frontier.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Outcome {
        Pass,
        Fail,
        Error,
    }

    /// Append one row the moment it exists, so a crash costs ONE call rather
    /// than the run. B25 lost 33 completed measurements and forty minutes of
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
    /// never a wrong answer -- the same distinction `grade` already draws.
    fn grade_reply(
        r: &crate::ollama::Reply,
        it: &GenItem,
    ) -> (Outcome, String) {
        let code = crate::ollama::rust_block(&r.text);
        match grade(&code, it.preamble, it.tests, "rustc") {
            Ok(true) => (Outcome::Pass, format!("{} tok", r.prompt_tokens)),
            Ok(false) => (Outcome::Fail, format!("{} tok", r.prompt_tokens)),
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
        };
        let row = format!("{run}\t{tag}\t{word}\t{name}\t{note}");
        println!("run {run} · {tag:5} · {word} · {name} · {note}");
        log_row(&row);
        o
    }

    /// `(pass, fail, error)` over a set of outcomes. Counted by filtering
    /// rather than by `+=`, which `arithmetic_side_effects` rejects.
    fn tally(os: &[Outcome]) -> (usize, usize, usize) {
        let n = |w: Outcome| os.iter().filter(|o| **o == w).count();
        (n(Outcome::Pass), n(Outcome::Fail), n(Outcome::Error))
    }

    /// Report one condition. An ERROR count above zero means the run is
    /// INCOMPLETE, and the summary has to say so where a reader will see it.
    fn report(label: &str, os: &[Outcome]) {
        let (p, f, e) = tally(os);
        let total = os.len();
        println!("  {label:5} pass {p}/{total} · fail {f} · error {e}");
        if e > 0 {
            println!(
                "    {e} of {total} did not RUN -- this condition is \
                 incomplete, not measured (V27)"
            );
        }
    }

    #[test]
    fn context_is_the_only_variable_between_the_two_prompts() {
        // V108: the two conditions must differ in exactly one thing. If the
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
    fn an_error_is_never_counted_as_a_failure() {
        // V27, and B25 in one assertion: a transient must not read as a miss.
        let os = [Outcome::Pass, Outcome::Fail, Outcome::Error];
        assert_eq!(tally(&os), (1, 1, 1), "three outcomes, not two");
        let all_err = [Outcome::Error, Outcome::Error];
        assert_eq!(
            tally(&all_err),
            (0, 0, 2),
            "a run that never ran scores zero PASS and zero FAIL"
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
            println!("lens pack unavailable -- nothing measured (V27)");
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
}
