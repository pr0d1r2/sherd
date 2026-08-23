# SPEC

## §G GOAL

Count tokens. Sole call site for `itok`.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/adopt|foreign single-file spec → federation: row placement, citation rewrite, conservation
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
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it

## §C CONSTRAINTS

- ⊥ own tokenizer, ⊥ own bytes/4 (R2).
- `itok::` named here & nowhere else in crate (`.:V72`).

## §V INVARIANTS

V1: ∀ count carries method label. ⊥ bare int
V2: gate runs on `bpe` or better. `dummy` measured 48% low on caveman spec ∴ ⊥ a gate
V3: `ENTRY_COST` = 28,543 — measured harness overhead, re-billed every turn
V4: `working(window)` saturates @ 0. negative budget = "⊥ fit", ⊥ a huge one
V5: unreadable file → error, ⊥ silent 0 (itok B11d counted a dir as 0)
V6: `.context-limits` — longest matching PREFIX wins ∴ a new node inherits its parent's ceiling, ⊥ dropping to the global default
V7: an unparsable ceiling is an ERROR naming its line, ⊥ a skipped row. itok B7 skipped one, reported "checked: 1 of 2" & exited 0 while gating nothing

## §T TASKS

id|status|task|cites
T3|.|needs `tdd::split_module` — cross-node, name it before driving|`.:V50`
T4|.|apply a tighter ceiling to `mod.rs`/`lib.rs`|`.:V51`
T6|.|needs `sherd.toml` and a TOML parser — neither exists|V2
T7|.|needs a compile-fail test, ⊥ a function|`.:V72`
T8|.|entry cost BEFORE & AFTER an adoption — root+chain vs the single file it replaced — ∵ `§N`+`§F` are always-on RESIDUE & the saving can be NEGATIVE on a small tree. this number is what says a migration PAID|`src/adopt:V1`
