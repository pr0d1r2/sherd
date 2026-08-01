# SPEC

## §G GOAL

Count tokens. Sole call site for `itok`.

## §C CONSTRAINTS

- ⊥ own tokenizer, ⊥ own bytes/4 (R2).
- `itok::` named here & nowhere else in crate (`.:V72`).

## §V INVARIANTS

V1: ∀ count carries method label. ⊥ bare int
V2: gate runs on `bpe` or better. `dummy` measured 48% low on caveman spec ∴ ⊥ a gate
V3: `ENTRY_COST` = 28,543 — measured harness overhead, re-billed every turn
V4: `working(window)` saturates @ 0. negative budget = "⊥ fit", ⊥ a huge one
V5: unreadable file → error, ⊥ silent 0 (itok B11d counted a dir as 0)

## §T TASKS

id|status|task|cites
T1|x|`count`, `count_file`, method label|V1,V2
T2|x|`ENTRY_COST`, `working`|V3,V4
T3|.|needs `tdd::split_module` — cross-node, name it before driving|`.:V50`
T4|.|apply a tighter ceiling to `mod.rs`/`lib.rs`|`.:V51`
T5|.|read per-path ceilings in `.context-limits` format|`.:V52`
T6|.|needs `bbx.toml` and a TOML parser — neither exists|V2
T7|.|needs a compile-fail test, ⊥ a function|`.:V72`
