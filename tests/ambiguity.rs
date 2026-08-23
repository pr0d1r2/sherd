#![cfg(feature = "ollama")]
//! Gated with the loop it measures: every titration here needs a live 20B,
//! and the corpora it reads live in `sherd::assay`, which the `ollama` feature
//! carries (`.:B15`).

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

use sherd::assay::{
    GEN_CORPUS, GenItem, Reading, RowReadings, ambiguity_report, cross,
    gen_prompt, test_prompt,
};
use std::io::Write;

fn ask(prompt: &str) -> Result<String, String> {
    sherd::ollama::generate(prompt).map(|r| sherd::ollama::rust_block(&r.text))
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
/// `#[ignore]` because it needs `SHERD_ENDPOINT`. 2 calls per row per run.
///
/// ONE run, and that is a measured decision rather than the obvious default.
/// The first sweep ran three and every row came back identical (`.:R55`),
/// which is what "ever, not mostly" needed: a row that leaves a gap leaves it
/// every time. `assay:R1` had already measured runs buying nothing and items
/// buying everything, pooled; this is the same result per ROW. So the calls
/// go to more rows instead.
#[test]
#[ignore]
fn ambiguity_detector() {
    const RUNS: usize = 1;
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
