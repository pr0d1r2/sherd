#![cfg(feature = "ollama")]
//! Gated with the loop it measures: every titration here needs a live 20B,
//! and the corpora it reads live in `sherd::assay`, which the `ollama` feature
//! carries (`.:B15`).

//! T83, run against a live endpoint: one implementation, graded twice.
//!
//! The implementation is written BLIND in both arms, so the ONLY variable is
//! WHICH tests grade it — hidden ones written before any candidate existed,
//! or one the model wrote itself (`.:V108`). R51: hidden 30/33, own 26/33,
//! and the model's own test caught 0 of 3 wrong implementations while
//! REJECTING 7 of 30 correct ones. `.:V111` is that result as a rule.
//!
//! An INTEGRATION target rather than an inline `#[ignore]` (`.:T96`). The
//! counts and the report live in the lib and are unit-tested there.

#![cfg(feature = "ollama")]

use sherd::assay::{
    GEN_CORPUS, GenItem, Grade, Verdicts, authorship_report, authorship_row,
    gen_prompt, grade_detail, test_prompt,
};

fn ask(prompt: &str) -> Result<String, String> {
    sherd::ollama::generate(prompt).map(|r| sherd::ollama::rust_block(&r.text))
}

/// One implementation, graded twice. Blind in both arms.
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

/// One item, recorded. An endpoint failure is an ERROR line and never a
/// verdict, so a transient cannot look like the model getting it wrong
/// (`src/assay:V1`, and `src/assay:B1` is that mistake costing forty minutes).
fn measure(run: usize, it: &GenItem, vs: &mut Vec<Verdicts>) {
    match one(it) {
        Ok(v) => {
            println!("run {run} · {}", authorship_row(v, it.sig));
            vs.push(v);
        }
        Err(e) => println!("run {run} · ERROR · {e}"),
    }
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
    println!("{}", authorship_report(&vs));
}
