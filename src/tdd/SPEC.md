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

## §B BUGS

id|date|cause|fix
B1|2026-08-01|step 1 saw spec + tests but ⊥ the data model ∴ test author could ⊥ see `Edge`'s fields & reached for `owns` to compute depth. judge caught it (twice)|`signatures()` — public surface, ⊥ bodies. it is `§I`, ⊥ `§V`
B2|2026-08-01|judge LOOSENED from "does this prove the invariant" to "would a violator fail it" after 2 rejections read as over-strict. 2 rejections were the GUARD WORKING. under the weak bar it passed a test asserting on `not_owns` (prose) as if it were a path — impl & test agreed w/ each other & neither related to V2. gates GREEN, verdict MERGEABLE, code meaningless. self-consistent wrongness, the exact failure V3 exists to catch, caused by relaxing V3|judge ! require BOTH — right SUBJECT (field names vs data model) AND falsifiable. judge gets `signatures()` too. GENERALLY: a guard rejecting repeatedly is evidence about the WORK, ⊥ about the guard
B3|2026-08-01|`cargo test` ⊥ refresh `target/debug/<bin>` — it builds a separate test harness ∴ 2 runs used a stale binary & reported identical token counts. read as "prompt unchanged" ⊥ "binary unchanged"|`cargo build` before any run that exercises the bin. gate ! rebuild first
B4|2026-08-01|`signatures()` stripped `///` docs ∴ judge saw `pub not_owns: String` bare & could ⊥ tell prose from path. it approved a test asserting `not_owns` as a path, twice. I built a surface extractor that drops exactly the semantics it exists to convey|keep doc comments. own test then caught a 2nd bug — field-level docs inside a type body routed to `pending` & never drained
B5|2026-08-01|repair prompt said "reply w/ the corrected FULL implementation region" ∴ model deleted the module `//!` header, rewrote `edges()` to SILENTLY SKIP multi-level rows (violating `.:V48`, which it never saw), & swapped hyphens for U+2011|repair returns ONLY the function it added. steps 2/4 ⊥ see root §V ∴ ⊥ license to touch anything they cannot check
B6|2026-08-01|⊥ prompt carried `FORMAT.md`. model asked to satisfy `V2: ... ⊥ skip levels` w/ no symbol key — `⊥` could read as math bottom or noise. `cavespec` vendors FORMAT.md verbatim "so a tool that cites the format can read it"; bbx cites it everywhere & had ⊥ copy|vendor `FORMAT.md`; compile a NOTATION slice (symbols + section meanings, ~200 tok ⊥ the whole 750) into every prompt that reads an invariant. `.:V82` contract-⊥-implementation applied to our own prompts
B7|2026-08-01|repair splice keyed on `\npub fn check_` ∴ when the model named it `find_depth_violations` the fix APPENDED instead of replacing → `E0428` redefined. a heuristic where a record would do|track the inserted block; ⊥ found → refuse & say so, ⊥ guess
