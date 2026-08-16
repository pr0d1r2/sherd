# SPEC

## §G GOAL

a corpus, & a way to grade a model against it. an assay tests a sample against what it is CLAIMED to be & reports when the claim is FALSE.

## §C CONSTRAINTS

- the grader is `rustc`, NEVER a model. a model grader confounds twice: `.:R40` measured the judge as itself precision-sensitive, & `.:tdd:B2` is a judge loosening under pressure.
- corpora are KNOWN-ANSWER. an item whose right verdict is unknown measures the corpus, ⊥ the model.
- ⊥ the loop. `.:tdd` writes code w/ a model; this node measures WHETHER it can. separate nodes so a bad measurement cannot drive a commit.
- depends on `.:tdd` for `NOTATION` & `blind_prompt` — the RIGHT direction: this node measures that judge, so it reads its prompt.

## §I INTERFACES

- lib: `grade(candidate, preamble, tests, rustc) -> Result<bool,String>` — compile & run against HIDDEN tests
- lib: `titrate_tier(&Tier, judge) -> Result<TierScore,String>`
- lib: `gen_prompt(inv, sig, preamble)` · `gen_prompt_in_context(pack, ..)` · `blind_prompt_bare(inv, added)`
- lib: `GEN_CORPUS` · `RECORDED` · `VAGUE` · `SUBTLE` · `TIERS`
- file: `target/titration.tsv` — one row per call, appended AS PRODUCED

## §R RESEARCH

id|topic|finding|src
R1|corpus size vs runs|variance across runs measured ZERO in every titration to date (`.:R37`, `.:R42`) ∴ more RUNS buy nothing & more ITEMS buy everything. `.:T79` overturned 3 of 11 pre-registered classes by adding items, & would have overturned none by adding runs|`.:R42`, `.:R45`

## §V INVARIANTS

V1: ERROR ⊥ FAIL. a call that did ⊥ RUN says nothing about capability, & counting it as a miss makes a flaky network look like a located frontier (`.:tdd:B25`)
V2: a rung built to FAIL asserts nothing about the score. only the REGRESSION rung asserts — a test demanding success where a boundary is sought is flaky by construction & the first red gets answered by weakening the corpus
V5: a class predicted AFTER seeing the scores fits any result ∴ pre-register, & score the prediction

## §T TASKS

id|status|task|cites
T1|x|promote the node — corpora, grader, tiers & the titrations out of `.:tdd`, where they were the largest cluster (`.:R48`)|`.:V73`,`.:V110`
T2|.|`NOTATION` belongs w/ `.:spec`, which owns `SPEC.md` structure — it is a caveman-reading primer, ⊥ loop machinery. moving it drops this node's dep on `.:tdd` to `blind_prompt` alone|`.:V72`
T3|.|`GEN_CORPUS` is 101 lines & `RECORDED` 59 — data, ⊥ code, but they sit in the impl half & are read only by tests. decide: `#[cfg(test)]`, or a real fixture file|`.:V50`
T4|.|`.:T83` (test authorship) & `.:T84` (signature given vs invented) — the 2 remaining variables of the `.:R44`→`.:tdd:T13` gap. one variable per comparison is `.:V108`|V5
T5|x|`context_titration` per-call outcome `pass\|fail\|error`, errors counted apart & never as failures, partials written AS produced ∴ a crash costs ONE call ⊥ the run. was `.:tdd:T17`, moved here w/ the harness|V1
T6|x|re-ran `.:T82` once T5 landed — the first attempt had NO result: 33 of 66 calls, killed mid-condition. was `.:tdd:T18`|V1,`.:V108`

## §B BUGS

id|date|cause|fix
B1|2026-08-19|`context_titration` `.expect()`d on `generate` ∴ ONE transient (`timeout: receive response`, a 9.6k ctx call) killed the run & discarded 33 completed measurements — 40 min of endpoint time, & `.:T82` left w/ no answer. `grade()` ONE LINE BELOW returns `Err` for exactly this distinction & it was written deliberately. the fn was 58 lines, triple-nested: the two error paths sat 6 lines apart @ depth 3, the shape `cognitive_complexity`/`excessive_nesting` reject. was `.:tdd:B25`, moved here w/ the harness|V1. T5 fixed it, T6 re-ran
