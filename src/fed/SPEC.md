# SPEC

## §G GOAL

Federation edges. `§F` table, parent→child, chain to a node.

## §V INVARIANTS

V1: `§F` row = `dir|owns|⊥owns|tokens`. 4 cells or ⊥ a row
V2: edge depth = parent + 1 exactly. ⊥ skip levels
V3: `⊥owns` ! present — positive lens decides DESCEND, negative one decides STOP. the negative is the byte that prevents loading
V4: literal `|` in a cell escaped `\|`. row splitting honors it
V5: `tokens` = `-` means UNRECORDED, ⊥ zero
V6: header row (`dir|owns|…`) ⊥ an edge
V7: `§F` parse stops @ next `## §` header
V8: ignore globs — `target/`, `.git/`, `node_modules/`, `.direnv/` ⊥ walked
V9: a `§T` row states REMAINING work, ⊥ history. "`X` landed, `Y` still open" reads to a machine as "test `X`" & `X` already passes ∴ ⊥ a red test (`.:plan` B2). what landed lives in the commit trail
V10: a detector's test ! include a POSITIVE case. asserting only that nothing was found is satisfied by a fn that always finds nothing (B6)
V11: sibling `§F` lenses ! EXHAUSTIVE — every child dir on disk appears as a row. a child absent from `§F` is unreachable by descent & invisible to a reader who trusts the table
V12: sibling `§F` lenses ! DISJOINT — 2 rows ⊥ name the same `dir`. a duplicate makes descent ambiguous & `route` would have to open both

## §T TASKS

id|status|task|cites
T1|x|`edges` parse w/ escape handling|V1,V4,V6,V7
T2|x|`chain` root→node|V2
T3|x|`discover` walk w/ ignores, via `is_ignored_dir` (LLM-authored)|V8
T4|~|cycle detection over the federation DAG|V2
T5|x|`find_exhaustive_violations` — dirs named twice, and child dirs w/ no row|V11
T6|.|`§N` derive from parent `§F`|V3
T7|x|wire `find_exhaustive_violations` into `bbx check`|V11
T8|.|orphan check — a `SPEC.md` no parent `§F` row points at|`.:V3`
T9|.|descend one edge per step, reloading only that child|`.:V19`
T10|.|promote an invariant from a leaf to the common ancestor|`.:V13`
T11|.|report `§N` that differs from what `§F` derives|`.:V36`
T12|.|report a flat `.rs` owning node-local invariants — wants its own dir|`.:V73`

## §B BUGS

id|date|cause|fix
B1|2026-08-01|FIXED by replacement. `find_depth_violations` (LLM-authored) hand-rolled its own recursive walk w/ ⊥ ignore globs ∴ descends `target/`, `.git/` — violates `.:V23`. also a 2nd walker in the module that already has `walk()`, the two-readings defect, in the file whose own B-log names it|reuse `discover()`. FIRST CAUSE WRONG: I recorded 'it was ⊥ in the step-2 surface'. VERIFIED FALSE — `discover()` & `walk()` were BOTH in the prompt. the model saw them & duplicated anyway ∴ cause is that nothing ASKED it to reuse. `.:V59` on my own bug record
B2|2026-08-01|FIXED by replacement. LLM-authored test wrote `temp_depth_test` in CWD, ⊥ a real temp dir ∴ races under parallel test runs & leaks the dir if the test panics before cleanup|`std::env::temp_dir()` + unique name, cleanup on drop
B3|2026-08-01|FIXED by replacement. `find_depth_violations` read EVERY `*.md`, ⊥ only `SPEC.md` ∴ a federation table in a README is treated as authoritative|scope to `SPEC.md`, per `.:V5`/`.:V1`
B4|2026-08-01|FIXED & hypothesis CONFIRMED. was twice — `check_edge_depth` & `missing_not_owns`, independent tasks, both re-scan the `§F` section itself — its own `in_f` loop & header skip — instead of calling `edges(text)` & checking `e.dir`. residual two-readings, milder than B1's duplicate walker but real|HYPOTHESIS: step 2 gets the FULL impl body ∴ sees `edges()`'s scan loop & IMITATES it. surface-by-example induces copying. VERIFIED: step 2 given SIGNATURES ⊥ bodies → `depth_violations(&[Edge])`, 5 lines ⊥ 30, COMPOSES w/ `edges()`, `in_f` 6→3. also 20% cheaper (max call 1,438→1,243)
B5|2026-08-01|`c56869e` DELETED `missing_not_owns` (committed 2 commits earlier). I overwrote `src/fed/mod.rs` w/ the pre-experiment file to isolate a variable & the commit swept the loss in. tests 22→21 & I read it as noise ∴ a REGRESSION shipped inside a commit whose message claimed only an improvement|restored by re-running V3 under the new config. GENERALLY: resetting a file to isolate an experiment DISCARDS everything else in it — diff against HEAD before committing an experiment's output, & a falling test count is a finding ⊥ noise
B6|2026-08-01|`bbx apply` committed `detect_cycles(edges) -> Vec::new()` UNATTENDED, w/ a doc comment saying "stub ... satisfies the current test suite". the test asserted only "no cycle in this simple graph" ∴ a fn that always finds nothing passes perfectly. judge approved (the test DID check depth), gates green, committed|(a) gate now compiles `-D warnings` — `unused variable: edges` is how a stub announces itself & would have blocked this exact commit; (b) judge ! require a POSITIVE case for a detector; (c) stub removed. GENERALLY: a detector tested only on the NEGATIVE case is satisfied by returning the negative
B7|2026-08-01|LLM repair reached for `scopeguard::guard` — a crate this repo does ⊥ depend on ∴ `E0433`, & the run was mid-repair when its wall-clock budget ran out|4 lines of local `Drop` replaced it. the model reaches for a crate rather than 4 lines; its surface shows the API but ⊥ the dependency list
B8|2026-08-01|`unused import: Path` in a test module COMMITTED through a `-D warnings` gate. `cargo build` ⊥ compile `#[cfg(test)]` code ∴ the flag never saw it. also `find_exhaustive_violations` is called ONLY by tests — `review::unwired`'s exact case, surfaced by my own check & skimmed past|`RUSTFLAGS` exported so BOTH `build` & `test` deny. T6 wires the fn into `check`. GENERALLY: a flag on one command is ⊥ a flag on the toolchain
