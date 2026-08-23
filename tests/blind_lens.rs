#![cfg(feature = "ollama")]
//! Gated with the loop it measures: every titration here needs a live 20B,
//! and the corpora it reads live in `sherd::assay`, which the `ollama` feature
//! carries (`.:B15`).

//! The blind lens, run against a live endpoint.
//!
//! `.:V22` is a claim about the endpoint, so it is measured against the
//! endpoint. Two arms and a titration:
//!
//! - the STUB arm — the five stubs actually recorded in `§B`, verbatim, each
//!   paired with the invariant it was written against;
//! - the WORKING arm — real functions from this repo. Without it the stub
//!   number means nothing: a judge answering NO to everything scores 5/5 on
//!   stubs, which is the vacuous pass the other arm exists to catch;
//! - the TITRATION (`.:T74`) — rung 0 is the regression guard and asserts;
//!   rungs 1-3 exist to FAIL, so they report and assert nothing about the
//!   score (`src/assay:V2`).
//!
//! INTEGRATION targets rather than inline `#[ignore]`s (`.:T96`).

#![cfg(feature = "ollama")]

use sherd::assay::{RECORDED, TIERS, titrate_tier};
use sherd::tdd::{blind_prompt, is_yes};

/// One arm of `RECORDED`, scored against the endpoint.
fn measure_arm(violates: bool) -> usize {
    RECORDED
        .iter()
        .filter(|it| it.violates == violates)
        .filter(|it| {
            let r = sherd::ollama::generate(&blind_prompt(it.inv, it.code))
                .expect("endpoint unreachable -- SHERD_ENDPOINT");
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

/// THE CONTROL. A judge that answers NO to everything scores 5/5 on the stub
/// corpus, so the stub number means nothing without this one.
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

/// THE TITRATION (`.:T74`). Rung 0 is the regression guard and asserts;
/// rungs 1-3 exist to FAIL, so they report and assert nothing about the
/// score. A test demanding success at a rung built to break it would be
/// flaky by construction, and the first red would be answered by weakening
/// the corpus -- the one move `.:V103` forbids.
#[test]
#[ignore]
fn blind_lens_titration() {
    let mut judge =
        |p: &str| sherd::ollama::generate(p).map(|r| is_yes(&r.text));
    for tier in TIERS {
        let s = titrate_tier(tier, &mut judge).expect(
            "endpoint unreachable -- a rung that did not run is an error, \
             not a boundary (V26)",
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
