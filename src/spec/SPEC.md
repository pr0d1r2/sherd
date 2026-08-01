# SPEC

## §G GOAL

`SPEC.md` structure. Sole call site for `cavespec`.

## §C CONSTRAINTS

- ⊥ reimpl parse, fmt, id, citation check — `cavespec` owns them, zero-dep pure fn (R3).
- `cavespec::` named here & nowhere else (`.:V72`).

## §V INVARIANTS

V1: what `cavespec` owns is ADAPTED, ⊥ reimplemented. two implementations of one format is the defect `cavespec` exists to end
V2: `§F`/`§N` ⊥ belong here — they are blackbox additions, they live in `fed`
V3: parity vs `cavespec` ! be MEASURED before claiming absorption (`.:V59`). currently BELIEF

## §T TASKS

id|status|task|cites
T1|x|`check`, `fmt`, `sections` binding|V1
T2|.|capability-parity audit vs `cavespec`, written down|V3
T3|.|`--records` baseline wiring for closed-option survival|V1
