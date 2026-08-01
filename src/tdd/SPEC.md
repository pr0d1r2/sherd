# SPEC

## §G GOAL

TDD as separate round-trips — red, judge, green, local gate, capped repair.

## §C CONSTRAINTS

- ∀ step declares its profile. ⊥ one call sees spec + tests + impl together (`.:V87`).
- step 3 = LOCAL, deterministic, ZERO tokens (`.:V18`).

## §V INVARIANTS

V1: step 2 & 4 ⊥ touch the test region. STRUCTURAL — `insert_impl` writes above `#[cfg(test)]` only ∴ ⊥ a matter of asking nicely
V2: RED test ! fail at an ASSERTION, ⊥ at compile. a test that ⊥ build is ⊥ a red test
V3: judge sees invariant + test, NEVER the impl. guards self-consistent wrongness — one model writing both test & impl to match its own misreading
V4: test already green → STOP & restore. ⊥ a red test, nothing to drive
V5: repair budget capped. exhaustion → report what was TRIED, ⊥ silent give-up
V6: test ! cite a declared `§V` id. a test for an invariant that ⊥ exist encodes an unstated rule
V7: `split_module` has ONE definition here. the code ceiling (`.:V50`) ! reuse it — 2 readings of one rule is the defect this project ends
V8: judge ! see the DATA MODEL. w/o field shapes it cannot tell a test asserting on the wrong field from one asserting on the right one (B2)
V9: judge answers on SUBJECT ∧ FALSIFIABILITY, ⊥ either alone. a test can be perfectly falsifiable & still test the wrong quantity (B2)
V10: repeated rejection = evidence about the WORK. loosening a guard to make it pass is silencing it (B2)
V11: gates GREEN ⊥ correctness. `cargo test` + `bbx check` both passed on semantically empty code (B2) — green for the wrong reason, inside the tool built to catch it
V12: prompts that must READ an invariant get the NOTATION contract. asking a model to satisfy `⊥ skip levels` w/o the symbol key is asking it to guess (B6)
V13: repair REPLACES exactly what the previous attempt added, tracked ⊥ guessed. name-prefix heuristics duplicate definitions (B7); full-region rewrite deletes unrelated code (B5)
V14: surface keeps DOC COMMENTS. bare field names cannot distinguish `not_owns` (prose) from `dir` (path) & that is the judge's whole question (B4)
V15: prompt = RULE depth. §G §C §I §V §T in, §B §R out. §T is the PLAN (the row names the work); §B/§R are archive. MEASURED: one §B row pushed step 1 1,210→1,520 tok & flipped a correct run to rejected (B9)
V16: judge objection FED BACK, ⊥ discarded. it is actionable signal; hand-tuning the prompt instead burned 6 configurations before I noticed (B10)
V17: model DETERMINISTIC @ temp 0 — 3/3 identical `gen` counts on one prompt ∴ outcome differences are prompt differences, & isolation works. ⊥ blame variance
V18: step 2 gets the CONTRACT — the calls the test makes that ⊥ exist yet, extracted deterministically (`.:V18`). the SIGNATURE constrains the design: `check_edge_depth(text)` took no path ∴ no walking ∴ no ignore-glob violation & no temp dir. fixing a NAME mismatch removed 3 unrelated defects (B12)

## §T TASKS

id|status|task|cites
T1|x|`split_module`, `insert_test`, `insert_impl` w/ structural test-region guard|V1,V7
T2|x|red → judge → green → gate → repair loop|V2,V3,V4,V5
T3|x|invariant-exists precondition|V6
T4|.|assert RED fails at assertion ⊥ at compile|V2
T5|.|`§T` row status flip on green (`.` → `x`)|V5
T6|.|judge gets `signatures()` data model|V8,B2
T7|.|`cargo build` before gate runs that exercise the bin|B3
T8|.|record per-request template overhead (~67 tok, measured) in entry-cost accounting|V2
T9|x|step 2 ! define exactly the fn the test calls — pass the expected signature|V18

## §B BUGS

id|date|cause|fix
B1|2026-08-01|step 1 saw spec + tests but ⊥ the data model ∴ test author could ⊥ see `Edge`'s fields & reached for `owns` to compute depth. judge caught it (twice)|`signatures()` — public surface, ⊥ bodies. it is `§I`, ⊥ `§V`
B2|2026-08-01|judge LOOSENED from "does this prove the invariant" to "would a violator fail it" after 2 rejections read as over-strict. 2 rejections were the GUARD WORKING. under the weak bar it passed a test asserting on `not_owns` (prose) as if it were a path — impl & test agreed w/ each other & neither related to V2. gates GREEN, verdict MERGEABLE, code meaningless. self-consistent wrongness, the exact failure V3 exists to catch, caused by relaxing V3|judge ! require BOTH — right SUBJECT (field names vs data model) AND falsifiable. judge gets `signatures()` too. GENERALLY: a guard rejecting repeatedly is evidence about the WORK, ⊥ about the guard
B3|2026-08-01|`cargo test` ⊥ refresh `target/debug/<bin>` — it builds a separate test harness ∴ 2 runs used a stale binary & reported identical token counts. read as "prompt unchanged" ⊥ "binary unchanged"|`cargo build` before any run that exercises the bin. gate ! rebuild first
B4|2026-08-01|`signatures()` stripped `///` docs ∴ judge saw `pub not_owns: String` bare & could ⊥ tell prose from path. it approved a test asserting `not_owns` as a path, twice. I built a surface extractor that drops exactly the semantics it exists to convey|keep doc comments. own test then caught a 2nd bug — field-level docs inside a type body routed to `pending` & never drained
B5|2026-08-01|repair prompt said "reply w/ the corrected FULL implementation region" ∴ model deleted the module `//!` header, rewrote `edges()` to SILENTLY SKIP multi-level rows (violating `.:V48`, which it never saw), & swapped hyphens for U+2011|repair returns ONLY the function it added. steps 2/4 ⊥ see root §V ∴ ⊥ license to touch anything they cannot check
B6|2026-08-01|⊥ prompt carried `FORMAT.md`. model asked to satisfy `V2: ... ⊥ skip levels` w/ no symbol key — `⊥` could read as math bottom or noise. `cavespec` vendors FORMAT.md verbatim "so a tool that cites the format can read it"; bbx cites it everywhere & had ⊥ copy|vendor `FORMAT.md`; compile a NOTATION slice (symbols + section meanings, ~200 tok ⊥ the whole 750) into every prompt that reads an invariant. `.:V82` contract-⊥-implementation applied to our own prompts
B7|2026-08-01|repair splice keyed on `\npub fn check_` ∴ when the model named it `find_depth_violations` the fix APPENDED instead of replacing → `E0428` redefined. a heuristic where a record would do|track the inserted block; ⊥ found → refuse & say so, ⊥ guess
B8|2026-08-01|first PLANTED violation proving the pre-commit gate passed straight through. the plant cited `` `V998` `` in BACKTICKS ∴ FORMAT reserves backticks for verbatim & cavespec correctly ⊥ read it as a citation. a bad plant, ⊥ a bad gate — but indistinguishable from one until re-planted|plant a BARE `V777`; commit then REFUSED, HEAD ⊥ moved. GENERALLY: a guard test passes vacuously if the plant is ⊥ actually a violation — green for the wrong reason, one level up, in the act of proving a guard
B9|2026-08-01|adding ONE §B row to `src/fed/SPEC.md` pushed step 1 from 1,210 to 1,520 tok & turned a run producing CORRECT code into one the judge rejected. bug history in a node spec pollutes the prompt that AUTHORS tests. `.:V43`/`.:V45` said rationale by reference for COST; the cost is also QUALITY|`rule_depth()` — §G §C §I §V §T in, §B §R out. dropping §T too was a 2nd error: §T is the plan, ⊥ the archive
B10|2026-08-01|judge rejections were treated as a HARD STOP ∴ I hand-tuned the prompt across 6 configurations chasing 1 success, discarding an actionable objection each time. the loop had feedback available & ⊥ used it|feed the objection back, capped at 3. GENERALLY: a reviewer's REASON is signal; throwing it away & guessing is the expensive path
B11|2026-08-01|attributed run-to-run differences to model variance. MEASURED FALSE: 3/3 trials identical `gen 3487` ∴ deterministic @ temp 0. every difference was a prompt change I made. `.:V59` — measurement ⊥ belief, on my own methodology|isolate ONE variable per run. determinism makes that possible
B12|2026-08-01|step 2 never told what to DEFINE ∴ test called `check_edge_depths(root,&e)` & step 2 invented another name → `E0425`, 3 repairs could ⊥ recover. also the root cause of `.:fed` B1/B2/B3: an unconstrained signature let it choose a `&Path` design & drag in walking, temp dirs & `*.md` globbing|`expected_calls()` — deterministic parse of calls absent from the surface, passed as a contract. NEXT run: judge YES first try, 3 round-trips, all 3 prior defects ABSENT. own test caught `fn x(` being read as a call
