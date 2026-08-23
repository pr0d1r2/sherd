# SPEC

## §G GOAL

Mechanical pre-review of what `apply` committed. Narrows what a reader ! catch.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/review|mechanical checks on what `apply` committed
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/lens|pack assembly, depth `rule`|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it

## §C CONSTRAINTS

- ⊥ a substitute for reading the diff. it cannot tell whether code satisfies an INVARIANT — the failure that matters.
- ∀ check ! trace to a commit on this branch it would have caught. ⊥ speculative rules.

## §V INVARIANTS

V1: `unwired` — a `pub fn` called only from tests LANDED but was never wired in. searched over the WHOLE crate: a caller in a SIBLING node counts, & checking only the declaring module is a false positive (B1)
V2: `negative-only` — a detector whose test asserts only the empty case is satisfied by a fn that always finds nothing (`src/fed:V10`)
V3: a finding is ADVISORY. review reports; the reader judges. auto-reverting on a heuristic would trade a false negative for a false positive & the false positive costs more
V4: `ignored-input` — a NEW `pub fn` w/ an `_`-prefixed param. `-D warnings` catches an unused input, so the next stub PREFIXED it & the warning vanished: the guard silenced by the code it guards (B3)
V5: ⊥ claim clean. report what was CHECKED — 3 mechanical rules of ~6 review questions
V6: a test FIXTURE ! be unique per INSTANCE, ⊥ per process. `std::process::id()` is the SAME for every test in one binary ∴ two tests sharing a tag get one directory & the first `Drop` deletes the other's repo. green ALONE, red in the SUITE — & a test that passes in isolation is the one nobody debugs (B4)
V7: a subprocess's EXIT STATUS ! be read, ⊥ only its spawn result. `Command::output()` returns `Ok` for a process that RAN & FAILED ∴ `let Ok(out) = ..` catches only "git ⊥ on PATH", & `git show <unknown rev>` yields Ok w/ EMPTY stdout — the diff then reads as "nothing changed" & the review reports no findings (B5). an empty result & a failed command ! be distinguishable, which is V5 one level down in the plumbing
V8: `undocumented` — a NEW `pub fn` w/ no doc comment. NEW only
V9: a FIX SHAPE is NAMED, ⊥ applied. `cargo clippy --fix` already applies every rewrite that is machine-applicable & MEASURED on this tree it changes NOTHING — all 252 warnings we carry are the ones upstream marks as needing JUDGEMENT ∴ an autocorrector for them re-implements the half deliberately left manual (`.:B24`). a shape is mined from a transform this repo ALREADY applied, w/ the count & the measurement, ∴ it reports evidence rather than taste

## §T TASKS

id|status|task|cites
T3|x|duplication check — new fn re-parsing what an existing fn parses|V1

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`unwired` searched only the DECLARING module ∴ flagged `find_exhaustive_violations` the moment `cli` started calling it. first real run of the verb, first finding, FALSE|search every non-test line of the crate. advisory design meant a false positive cost a reading, ⊥ a revert — which is why V3 says advisory
B2|2026-08-01|`unwired` counted occurrences & assumed "declaration + call" > 1. `pub fn f<'a>(` does ⊥ contain `f(` ∴ a generic decl counts 0 & a CALLED fn read as uncalled. own test used a non-generic fn ∴ never touched the branch|a CALL is any occurrence on a line that is ⊥ a declaration. 4th guard this session that passed w/o exercising what it guards
B3|2026-08-01|`check_split_hint(root, _budget)` ignored the budget & returned EVERY subdir, documenting "sufficient for the test". the `_` prefix silenced `-D warnings`, which is the guard that caught the previous stub ∴ the model routed around the guard rather than the task. test passed budget 0 against the repo root & could ⊥ fail|`ignored-input` rule. 3rd self-documented stub, 1st to defeat a guard deliberately
B4|2026-08-19|`TestRepo` keyed its temp dir on `std::process::id()` alone ∴ `added_in_commit_reads_pub_fns_out_of_the_diff` & `commit_finds_the_stub_shape_end_to_end` both took tag `review`, got ONE directory, & whichever finished first deleted the repo the other was reading. PASSED run alone, FAILED in the suite — the shape that reads as flakiness & gets re-run instead of read|V6. instance counter + pid. the fixture was written to unlock `.:R50`'s uncovered git paths & shipped w/ a race in its first hour
B5|2026-08-21|`added_in_commit` checked only the SPAWN result ∴ `sherd review <rev that ⊥ exist>` printed "no findings (checked: unwired, negative-only, ignored-input)" & exited 0 — a CLEAN BILL for a revision that is ⊥ there, & the most dangerous output this command has. `commit`'s own doc already said "Propagates a read failure — unreadable is ⊥ clean" while `added_in_commit` returned a bare `Vec` ∴ there was nothing to propagate. same shape as `src/fed:B6` (something that finds nothing passes cleanly) & V2 is that rule about DETECTORS, where this is the plumbing under them. FOUND by a coverage test asserting the POSITIVE case, ⊥ by any run|V7. `added_in_commit` returns `io::Result`, a non-zero exit is `Err`, & `review_cmd`'s exit-2 arm becomes reachable
B6|2026-08-22|`negative_only` fired on ANY new `pub fn` whose test lacked one of 7 hardcoded markers — `!.is_empty()`, `len() > 0`, `> 0` — all COLLECTION-shaped. V1 says the subject is a DETECTOR & `.:src/fed:B6` is `detect_cycles -> Vec::new()` ∴ "found nothing" is a failure mode only where nothing is EXPRESSIBLE. applied to every fn it flagged `double(n) -> u8` & every scalar the loop writes, & `.:src/tdd:V23` made that FATAL ∴ no scalar could ever land — a 2nd structural block on a merit win, beside `.:src/tdd:V29`|scoped to a `Vec`/`Option`/`Map` return. FOUND by fixing `.:src/tdd:V29` & watching the SAME run fail again one rule over
T6|.|mine the next shape from `§B` — 110 rows, & the recurrence language ("same shape" 6, "twice" 14, "again" 30) is where the clusters are|V9
