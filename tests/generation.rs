#![cfg(feature = "ollama")]
//! Gated with the loop it measures: every titration here needs a live 20B,
//! and the corpora it reads live in `sherd::assay`, which the `ollama` feature
//! carries (`.:B15`).

//! T77 and T82, run against a live endpoint.
//!
//! `generation_titration` — does invariant PRECISION drive writing? R42 and
//! R44: sharp 30/33, vague 9/33 at the extended corpus, graded by `rustc` on
//! tests the writer never saw.
//!
//! `context_titration` — does a node's whole lens pack make the model WORSE?
//! R49: bare 30/33, ctx 30/33 at a 10,009-token pack. A true null, and the
//! same single item failed 3/3 in BOTH arms.
//!
//! INTEGRATION targets rather than inline `#[ignore]`s (`.:T96`). `Outcome`,
//! the tally and the report live in the lib and are unit-tested there; only
//! the endpoint half is here, so nothing is written twice (`.:B13`).

#![cfg(feature = "ollama")]

use sherd::assay::{
    GEN_CORPUS, GenItem, Grade, Outcome, gen_prompt, gen_prompt_in_context,
    grade_detail, titration_report,
};
use std::io::Write;

/// Append one row the moment it exists, so a crash costs ONE call rather
/// than the run. `src/assay:B1` lost 33 completed measurements and forty minutes
/// of endpoint time to a single transient.
fn log_row(row: &str) {
    let path = std::path::Path::new("target").join("titration.tsv");
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{row}");
    }
}

/// Grade a reply that arrived. A compiler that cannot RUN is an error, never
/// a wrong answer, and a run that never TERMINATED is a third thing (`V6`).
fn grade_reply(r: &sherd::ollama::Reply, it: &GenItem) -> (Outcome, String) {
    let code = sherd::ollama::rust_block(&r.text);
    let tok = format!("{} tok", r.prompt_tokens);
    match grade_detail(&code, it.preamble, it.tests, "rustc") {
        Ok(Grade::Pass) => (Outcome::Pass, tok),
        Ok(Grade::Fail | Grade::NoCompile) => (Outcome::Fail, tok),
        Ok(Grade::Hung) => (Outcome::Hung, tok),
        Err(e) => (Outcome::Error, e),
    }
}

/// One measurement, which NEVER panics. A transient belongs in the record,
/// not in a stack trace.
fn run_one(prompt: &str, it: &GenItem) -> (Outcome, String) {
    match sherd::ollama::generate(prompt) {
        Err(e) => (Outcome::Error, e),
        Ok(r) => grade_reply(&r, it),
    }
}

fn measure(tag: &str, run: usize, prompt: &str, it: &GenItem) -> Outcome {
    let (o, note) = run_one(prompt, it);
    let name = it.sig.split('(').next().unwrap_or(it.sig).trim();
    let row = format!("{run}\t{tag}\t{}\t{name}\t{note}", o.word());
    println!("run {run} · {tag:5} · {} · {name} · {note}", o.word());
    log_row(&row);
    o
}

/// T77. Records; asserts nothing about the model, for T74's reason -- a test
/// demanding a result from a run built to find one is flaky by construction,
/// and the first red would be answered by weakening it.
#[test]
#[ignore]
fn generation_titration() {
    const RUNS: usize = 3;
    let (mut sharp, mut vague) = (Vec::new(), Vec::new());
    for run in 1..=RUNS {
        for it in GEN_CORPUS {
            let s = gen_prompt(it.sharp, it.sig, it.preamble);
            sharp.push(measure("sharp", run, &s, it));
            let v = gen_prompt(it.vague, it.sig, it.preamble);
            vague.push(measure("vague", run, &v, it));
        }
    }
    println!("\nGENERATION TITRATION");
    print!("{}", titration_report("sharp", &sharp));
    print!("{}", titration_report("vague", &vague));
}

/// T82. Same items, same hidden tests, same sharp wording -- the node's real
/// lens pack is the only thing that changes (`.:V108`).
#[test]
#[ignore]
fn context_titration() {
    const RUNS: usize = 3;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let node = root.join("src/tokens");
    // A pack that cannot be read is a run that did not happen, never a zero:
    // `src/assay:V1`, and `src/assay:B1` is what an `.expect()` here costs.
    let Ok(pack) = sherd::lens::pack(root, &node, sherd::lens::Depth::Rule)
    else {
        println!("lens pack unavailable -- nothing measured (V1)");
        return;
    };
    let (mut bare, mut ctx) = (Vec::new(), Vec::new());
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
    print!("{}", titration_report("bare", &bare));
    print!("{}", titration_report("ctx", &ctx));
}
