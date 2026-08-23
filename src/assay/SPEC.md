# SPEC

## §G GOAL

FROZEN until rung `0.7` (`.:V117`) — it works, & no further development lands here before the mechanical surface ships.

a corpus, & a way to grade a model against it. an assay tests a sample against what it is CLAIMED to be & reports when the claim is FALSE.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/lens|pack assembly, depth `rule`|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/review|mechanical checks on what `apply` committed
sib|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way

## §C CONSTRAINTS

- the grader is `rustc`, NEVER a model. a model grader confounds twice: `.:R40` measured the judge as itself precision-sensitive, & `.:src/tdd:B2` is a judge loosening under pressure.
- corpora are KNOWN-ANSWER. an item whose right verdict is unknown measures the corpus, ⊥ the model.
- ⊥ the loop. `.:tdd` writes code w/ a model; this node measures WHETHER it can. separate nodes so a bad measurement cannot drive a commit.
- depends on `.:tdd` for `NOTATION` & `blind_prompt` — the RIGHT direction: this node measures that judge, so it reads its prompt.

## §I INTERFACES

- lib: `grade_detail(candidate, preamble, tests, rustc) -> Result<Grade,String>` — compile & run against HIDDEN tests, under `GRADE_TIMEOUT`. `Grade` = `Pass|Fail|NoCompile|Hung`; the last two graded NOTHING & are counted APART (V1, V6)
- lib: `cross(code, test, preamble, rustc) -> Result<Reading,String>` — one BLIND test against one BLIND impl of the SAME row. `Reading` = `Agree|Disagree|Uncallable|Hung`
- lib: `RowReadings` · `ambiguity_report(&[RowReadings]) -> String` — per ROW, ⊥ pooled. a REPORT, ⊥ a gate
- lib: `titrate_tier(&Tier, judge) -> Result<TierScore,String>`
- lib: `gen_prompt(inv, sig, preamble)` · `gen_prompt_no_sig(inv, preamble)` · `test_prompt(inv, sig, preamble)` · `gen_prompt_in_context(pack, ..)` · `blind_prompt_bare(inv, added)`
- lib: `GEN_CORPUS` · `RECORDED` · `VAGUE` · `SUBTLE` · `TIERS` · `stub_for(sig)`
- file: `target/titration.tsv` — one row per call, appended AS PRODUCED
- file: `target/ambiguity.txt` — the impl & the test per row, kept AS PRODUCED (B1)

## §R RESEARCH

id|topic|finding|src
R1|corpus size vs runs|variance across runs measured ZERO in every titration to date (`.:R37`, `.:R42`) ∴ more RUNS buy nothing & more ITEMS buy everything. `.:R43` overturned 3 of 11 pre-registered classes by adding items, & would have overturned none by adding runs|`.:R42`, `.:R45`

## §V INVARIANTS

V1: ERROR ⊥ FAIL. a call that did ⊥ RUN says nothing about capability, & counting it as a miss makes a flaky network look like a located frontier (`.:src/tdd:B25`)
V2: a rung built to FAIL asserts nothing about the score. only the REGRESSION rung asserts — a test demanding success where a boundary is sought is flaky by construction & the first red gets answered by weakening the corpus
V5: a class predicted AFTER seeing the scores fits any result ∴ pre-register, & score the prediction
V6: a graded child ! run under a WALL-CLOCK bound, & non-termination is its OWN outcome counted apart. the MODEL writes what gets run ∴ termination is ⊥ assumable — `abort_budget_ms`'s authored test loops to `u64::MAX/4` (~4.6e15 iters) & `.output()` waits forever. killing the child ⊥ a repair: the wait then returns non-success & `grade_detail` reads it as `Fail` ∴ a hang gets RECORDED as a disagreement about a row nothing disagreed about — V1 inverted, & a FABRICATED finding is worse than a lost run
V7: the CORPUS is apparatus, ⊥ a lookup table ∴ it stays COMPILE-CHECKED. `predicted: Channel::Prose` is registered BEFORE the run so the 3-way split is falsifiable (`.:R43`), & `VAGUE`/`SUBTLE` are DERIVED from `RECORDED` — `code: RECORDED[n].code`, identical impl, 1 variable changed — ∴ whatever a rung loses is attributable to that variable alone. a parsed data file (`toml`, `jsonl`) moves a typo from a BUILD error to a runtime one, which is `.:V100`'s structure → gate DOWNGRADE on this repo's own measured ladder, & it makes the derivation a convention nobody checks ∴ the experiment silently loses its control. a 2nd `.rs` in the node keeps both (`.:V119`)
V8: the 4th VARIABLE is what the model is TOLD TO WRITE, ⊥ how much it is told. 3 have run & none closed the `.:R44`→`.:src/tdd:T13` gap: pack size changed NOTHING (`.:R49`, 30/33 both arms @ a 10,009-tok pack), a self-authored test is ANTI-correlated (`.:R51`), & the 1 that MOVED was handing over the SIGNATURE — 30/33 given vs 2/33 invented, 31/33 uncallable (`.:src/tdd:V28`). ∴ the axis is SPECIFICITY OF THE TARGET FORM, & a `§V` row states a RULE while `src/review`'s fix shape states the FORM: `let [a, b] = xs.as_slice() else` ⊥ "avoid indexing". a shape is `V28`'s signature generalised from 1 fn to a TRANSFORM ∴ the comparison is prompt-carrying-the-`§V`-row vs prompt-carrying-the-SHAPE, 1 variable, `V6`'s grader

## §T TASKS

id|status|task|cites
T2|.|`NOTATION` belongs w/ `.:spec`, which owns `SPEC.md` structure — it is a caveman-reading primer, ⊥ loop machinery. moving it drops this node's dep on `.:tdd` to `blind_prompt` alone|`.:V72`
T4|.|all 3 variables of the `.:R44`→`.:src/tdd:T13` gap have now RUN — pack size `.:R49`, test authorship `.:R51`, signature `.:src/tdd:V28` — & none closes it. name the 4th before running one: one variable per comparison is `.:V108`|V5
T5|~|FROZEN until rung `0.7` (`.:V117`). move the 7 corpora to `src/assay/corpus.rs` — `mod corpus;` beside `mod.rs`, ⊥ a parsed data file. 4,947 tok of the node's 13,413: `GEN_CORPUS` 2,398 · `RECORDED` 841 · `SUBTLE` 795 · `STUBS` 378 · `VAGUE` 364 · `TIERS` 112. supersedes the deleted `T3`, whose framing was WRONG: it said "read only by tests" & `RECORDED` has 15 impl-half references (10 of them `VAGUE`'s derivation) ∴ `#[cfg(test)]` was never available for it. only `GEN_CORPUS` fits that description — 12 test-half uses, 1 in impl|V7,`.:V119`,`.:V50`
T6|~|FROZEN until rung `0.7` (`.:V117`). MEASURE the 4th variable: same items, same hidden tests, prompt carries the `§V` ROW vs the FIX SHAPE. `.:V108` — 1 variable. PRE-REGISTER the prediction ∵ `.:R43`'s 3-way split is falsifiable only if the class is assigned BEFORE the run: shapes beat rows on items whose defect HAS a named transform, & change nothing where the fix is a judgement over seams. ⊥ RUN IT until `src/review`'s catalogue is worth measuring — 3 shapes is n=3, & `.:B19` is what a default tuned on 1 tree costs|V8,`.:V108`,`src/review:V9`
T7|.|the corpus needs items whose fix IS a named shape. today's 3 were mined from 34 + 24 + 1 applications in OUR tree, & a 4th was PROBED & REFUTED (`src/review:V10`) ∴ the corpus grows slower than the recurrences do ∴ the 4th variable is measurable only once a 2nd & 3rd stranger have contributed theirs|V8,`src/review:T6`,`src/review:V10`

## §B BUGS

id|date|cause|fix
B1|2026-08-19|`context_titration` `.expect()`d on `generate` ∴ ONE transient (`timeout: receive response`, a 9.6k ctx call) killed the run & discarded 33 completed measurements — 40 min of endpoint time, & `.:R49` left w/ no answer. `grade()` ONE LINE BELOW returns `Err` for exactly this distinction & it was written deliberately. the fn was 58 lines, triple-nested: the two error paths sat 6 lines apart @ depth 3, the shape `cognitive_complexity`/`excessive_nesting` reject. was `.:src/tdd:B25`, moved here w/ the harness|V1. T5 fixed it, T6 re-ran
B2|2026-08-20|`grade_detail` ran the compiled child w/ `.output()` & NO time bound since the day it was written ∴ T97's first run hung 55 min @ item 8 of 33 & would ⊥ have returned. B1 is the same lesson one level up — that lost 33 measurements to an `.expect()`, this loses a whole run to a wait w/ no bound, & BOTH are the harness trusting a call it does ⊥ control. `.:R51`, `authored_tests_vs_mutants` & the signature titration (`.:src/tdd:V28`) ran this path & got lucky|V6. T7 builds the bound
