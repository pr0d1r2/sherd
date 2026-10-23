# SPEC

## §G GOAL

Count tokens. Sole call site for `itok`.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|-
sib|src/split|-
sib|src/fed|-
sib|src/adopt|-
sib|src/lens|-
sib|src/ollama|-
sib|src/tdd|-
sib|src/plan|-
sib|src/wave|-
sib|src/review|-
sib|src/state|-
sib|src/slice|-
sib|src/land|-
sib|src/cli|-
sib|src/code|-
sib|src/debt|-
sib|src/assay|-
sib|src/git|-

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

## §B BUGS

id|date|cause|fix
B1|2026-09-30|`Ceilings::load` matched `Err(_)` on reading `.context-limits` ∴ EVERY read error was a cold start: a file that EXISTS & cannot be read (a directory in its place, a permission) silently became the default ceilings, & `budget`/`check`/`lens` measured against numbers nobody wrote. found testing `budget`'s refusals in #72|V5. only `NotFound` is a cold start; any other read error is `Err(".context-limits: <error>")`, which `budget` reports w/ exit 1
