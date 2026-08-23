# SPEC

## §G GOAL

`SPEC.md` structure. Sole call site for `microlith`.

## §C CONSTRAINTS

- ⊥ reimpl parse, fmt, id, citation check — `microlith` owns them, zero-dep pure fn (R3).
- `microlith::` named here & nowhere else (`.:V72`).

## §V INVARIANTS

V1: what `microlith` owns is ADAPTED, ⊥ reimplemented. two implementations of one format is the defect `microlith` exists to end
V2: `§F`/`§N` ⊥ belong here — they are sherd additions, they live in `fed`
V3: parity vs `microlith` ! be MEASURED before claiming absorption (`.:V59`). currently BELIEF
V5: what is imported from `microlith` is what its ROOT re-exports. an inner module path is ⊥ contract — the dep may privatize it & does (E0603, B1). same for RENDERING: print the dep's own `Display`, ⊥ recompose its parts
V4: a `§B` row whose fix names no `§V` is UNREFLECTED — pain w/o reflection. it will recur & the log becomes a list of things that happened, ⊥ a set of guards. advisory: some bugs warrant no rule & forcing one manufactures invariants to silence a gate

## §T TASKS

id|status|task|cites
T1|x|`check`, `fmt`, `sections` binding|V1
T2|.|capability-parity audit vs `microlith`, written down|V3
T3|.|`--records` baseline wiring for closed-option survival|V1
T4|.|check closed-option records survive an edit, via `--records`|`.:V44`
T5|.|`SPEC.why.md` format — one rationale per `§V`/`§B` id|`.:V43`
T6|x|`unreflected_bugs` — `§B` rows naming no invariant|V4
T7|x|V5 runner — `microlith` reached only through its root, in every `src/` file|V5

## §B BUGS

id|date|cause|fix
B1|2026-08-05|facade re-exported through the dep's INNER path — `microlith::violation::{Violation, NAMESPACE}` — & `check` recomposed `{NAMESPACE}/{rule}: {msg}`, which `Violation: Display` already prints. the dep trimmed its lib to root verbs ∴ E0603 & a HEAD that ⊥ compile (`.:B5`). the recomposition was a 2nd reading of an id the dep owns — V1 applied to output, ⊥ only to logic|import `microlith::Violation`; print `{v}` w/ the caller's coords prefixed. V5, runner @ T7
