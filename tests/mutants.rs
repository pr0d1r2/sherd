#![cfg(feature = "ollama")]
//! Gated with the loop it measures: every titration here needs a live 20B,
//! and the corpora it reads live in `bbx::assay`, which the `ollama` feature
//! carries (`.:B15`).

//! The mutation sweep, run against a live endpoint.
//!
//! Can a self-authored test kill a known-wrong implementation? `.:V111` says
//! such a test cannot grade its own author; this asks whether a MECHANICAL
//! check would have caught that. R53 answered it — 33 of 33 killed the stub,
//! so the check would have caught NONE of T83's failures and the remedy is
//! REFUTED.
//!
//! An INTEGRATION target rather than an inline `#[ignore]` (`.:T96`): an
//! ignored body counts in the coverage denominator and never runs. The
//! REPORT lives in the lib and is unit-tested there; only the part needing an
//! endpoint is here, so nothing is written twice (`.:B13`).

#![cfg(feature = "ollama")]

use bbx::assay::{
    GEN_CORPUS, GenItem, Grade, Kill, grade_detail, kills_report, stub_for,
    test_prompt,
};
use std::io::Write;

fn one(it: &GenItem) -> Result<Kill, String> {
    let stub = stub_for(it.sig).ok_or("no mutant")?;
    let t = bbx::ollama::generate(&test_prompt(it.sharp, it.sig, it.preamble))
        .map(|r| bbx::ollama::rust_block(&r.text))?;
    // Keep the test itself: T83 discarded its raw material and the next
    // question could not be asked without re-running (`assay:B1`). R54 was
    // read out of this file afterwards, and T97 came out of R54.
    log_test(it.sig, &t);
    let g = grade_detail(stub, it.preamble, &t, "rustc")?;
    Ok(Kill {
        killed: g == Grade::Fail,
        broken: g == Grade::NoCompile,
    })
}

fn log_test(sig: &str, body: &str) {
    let p = std::path::Path::new("target").join("authored-tests.txt");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)
    {
        let _ = writeln!(f, "=== {sig}\n{body}");
    }
}

/// An endpoint failure is an ERROR line, never a verdict -- a transient must
/// not read as a test that failed to discriminate (`assay:V1`).
fn measure(run: usize, it: &GenItem, ks: &mut Vec<Kill>) {
    let name = it.sig.split('(').next().unwrap_or(it.sig).trim();
    match one(it) {
        Ok(k) => {
            println!("run {run} · {} · {name}", k.word());
            ks.push(k);
        }
        Err(e) => println!("run {run} · ERROR · {name} · {e}"),
    }
}

/// Records; asserts nothing about the model.
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
    println!("{}", kills_report(&ks));
}
