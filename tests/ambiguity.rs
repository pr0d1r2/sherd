//! T97. The ambiguity detector, run against a live endpoint.
//!
//! `.:V112`: ask for a TEST and an IMPLEMENTATION blind from ONE `§V` row,
//! compile them against each other, and report a disagreement as a gap in the
//! ROW. Two calls and a compile, with no reference answer -- which is what
//! separates it from the mutation gate R53 refuted, and from T83, which needed
//! hidden tests because it was grading the model.
//!
//! An INTEGRATION target, not an inline test, and that is deliberate. The
//! titrations live inline and are `#[ignore]`d, so their bodies count in the
//! coverage denominator and never run: `.coverage` records three drops for
//! that one cause and says the next experiment must not add a fourth. `.:T96`
//! moves the rest here; this one starts here.

#![cfg(feature = "ollama")]

use bbx::assay::{
    GEN_CORPUS, GenItem, Reading, RowReadings, ambiguity_report, cross,
    gen_prompt, test_prompt,
};
use std::io::Write;

fn ask(prompt: &str) -> Result<String, String> {
    bbx::ollama::generate(prompt).map(|r| bbx::ollama::rust_block(&r.text))
}

/// One row, read twice, blind both times.
///
/// Both halves see the SAME text and neither sees the other, so the only
/// thing that can differ between them is how they read the row -- which is
/// `.:V108`, one variable, and the reason the disagreement is attributable
/// to the spec at all.
fn one(it: &GenItem) -> Result<Reading, String> {
    let code = ask(&gen_prompt(it.sharp, it.sig, it.preamble))?;
    let test = ask(&test_prompt(it.sharp, it.sig, it.preamble))?;
    log_pair(it.sig, &code, &test);
    cross(&code, &test, it.preamble, "rustc")
}

/// Keep the raw material. R54 -- the finding that produced this task -- was
/// read out of `target/authored-tests.txt` AFTER the run; T83 discarded its
/// own and the next question could not be asked without re-running (`B1`).
fn log_pair(sig: &str, code: &str, test: &str) {
    let p = std::path::Path::new("target").join("ambiguity.txt");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
    {
        let _ = writeln!(f, "=== {sig}\n--- impl\n{code}\n--- test\n{test}");
    }
}

/// An endpoint failure is an ERROR line and never a verdict (`assay:V1`): a
/// transient must not read as a row that left something unstated.
fn measure(run: usize, it: &GenItem, row: &mut RowReadings) {
    match one(it) {
        Ok(r) => {
            println!("run {run} · {} · {}", r.word(), row.label);
            row.push(r);
        }
        Err(e) => println!("run {run} · ERROR · {} · {e}", row.label),
    }
}

fn name(sig: &str) -> &str {
    sig.split('(').next().unwrap_or(sig).trim()
}

/// T97. Reports on the SPEC; asserts nothing about the model.
///
/// `#[ignore]` because it needs `BBX_ENDPOINT`. 2 calls per row per run.
///
/// THREE runs, though `assay:R1` says runs buy nothing and items buy
/// everything -- because what R1 measured was a POOLED rate, and the verdict
/// here is per ROW. "Ever, not mostly" is only sound if a row that leaves a
/// gap leaves it every time, which is R52's claim and this is the check on
/// it. If the third run agrees with the first, the next version drops to one
/// and spends the calls on more ROWS.
#[test]
#[ignore]
fn ambiguity_detector() {
    const RUNS: usize = 3;
    let mut rows: Vec<RowReadings> = GEN_CORPUS
        .iter()
        .map(|it| RowReadings::new(name(it.sig)))
        .collect();
    for run in 1..=RUNS {
        for (row, it) in rows.iter_mut().zip(GEN_CORPUS) {
            measure(run, it, row);
        }
    }
    println!("{}", ambiguity_report(&rows));
}
