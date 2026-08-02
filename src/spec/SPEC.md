# SPEC

## §G GOAL

`SPEC.md` structure. Sole call site for `microlith`.

## §C CONSTRAINTS

- ⊥ reimpl parse, fmt, id, citation check — `microlith` owns them, zero-dep pure fn (R3).
- `microlith::` named here & nowhere else (`.:V72`).

## §V INVARIANTS

V1: what `microlith` owns is ADAPTED, ⊥ reimplemented. two implementations of one format is the defect `microlith` exists to end
V2: `§F`/`§N` ⊥ belong here — they are blackbox additions, they live in `fed`
V3: parity vs `microlith` ! be MEASURED before claiming absorption (`.:V59`). currently BELIEF
V4: a `§B` row whose fix names no `§V` is UNREFLECTED — pain w/o reflection. it will recur & the log becomes a list of things that happened, ⊥ a set of guards. advisory: some bugs warrant no rule & forcing one manufactures invariants to silence a gate

## §T TASKS

id|status|task|cites
T1|x|`check`, `fmt`, `sections` binding|V1
T2|.|capability-parity audit vs `microlith`, written down|V3
T3|.|`--records` baseline wiring for closed-option survival|V1
T4|.|check closed-option records survive an edit, via `--records`|`.:V44`
T5|.|`SPEC.why.md` format — one rationale per `§V`/`§B` id|`.:V43`
T6|x|`unreflected_bugs` — `§B` rows naming no invariant|V4
