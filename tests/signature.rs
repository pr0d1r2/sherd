#![cfg(feature = "ollama")]
//! Gated with the loop it measures: every titration here needs a live 20B,
//! and the corpora it reads live in `bbx::assay`, which the `ollama` feature
//! carries (`.:B15`).

//! T84, run against a live endpoint: the same invariant and the same hidden
//! tests, with the signature GIVEN and with it INVENTED.
//!
//! An INTEGRATION target rather than an inline `#[ignore]`, which is `.:T96`.
//! An ignored body counts in the coverage denominator and never runs, so
//! every experiment lowered the number while making the instrument better --
//! `.coverage` records three drops for that one cause. `tests/ambiguity.rs`
//! measured the fix twice: an integration target does not appear in the
//! report at all.
//!
//! The REPORT lives in the lib, unit-tested there. Only the part that needs
//! an endpoint lives here, so nothing is written twice (`.:B13`).

#![cfg(feature = "ollama")]

use bbx::assay::{
    GEN_CORPUS, GenItem, SignaturePair, gen_prompt, gen_prompt_no_sig,
    grade_detail, signature_mark, signature_report,
};

fn ask(p: &str) -> Result<String, String> {
    bbx::ollama::generate(p).map(|r| bbx::ollama::rust_block(&r.text))
}

/// Same invariant, same hidden tests. Only the signature differs -- one
/// variable, which is `.:V108`.
fn one(it: &GenItem) -> Result<SignaturePair, String> {
    let given = ask(&gen_prompt(it.sharp, it.sig, it.preamble))?;
    let free = ask(&gen_prompt_no_sig(it.sharp, it.preamble))?;
    Ok((
        grade_detail(&given, it.preamble, it.tests, "rustc")?,
        grade_detail(&free, it.preamble, it.tests, "rustc")?,
    ))
}

/// An endpoint failure is an ERROR line and never a verdict (`assay:V1`).
fn measure(run: usize, it: &GenItem, rs: &mut Vec<SignaturePair>) {
    let name = it.sig.split('(').next().unwrap_or(it.sig).trim();
    match one(it) {
        Ok((g, f)) => {
            println!(
                "run {run} · given {} · free {} · {name}",
                signature_mark(g),
                signature_mark(f)
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
    let mut rs: Vec<SignaturePair> = Vec::new();
    for run in 1..=RUNS {
        for it in GEN_CORPUS {
            measure(run, it, &mut rs);
        }
    }
    println!("{}", signature_report(&rs));
}
