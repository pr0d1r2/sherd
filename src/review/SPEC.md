# SPEC

## §G GOAL

Mechanical pre-review of what `apply` committed. Narrows what a reader ! catch.

## §C CONSTRAINTS

- ⊥ a substitute for reading the diff. it cannot tell whether code satisfies an INVARIANT — the failure that matters.
- ∀ check ! trace to a commit on this branch it would have caught. ⊥ speculative rules.

## §V INVARIANTS

V1: `unwired` — a `pub fn` called only from tests LANDED but was never wired in ∴ the duplication the task existed to remove was relocated, ⊥ removed (`src/fed:V8` case)
V2: `negative-only` — a detector whose test asserts only the empty case is satisfied by a fn that always finds nothing (`src/fed:V10`)
V3: a finding is ADVISORY. review reports; the reader judges. auto-reverting on a heuristic would trade a false negative for a false positive & the false positive costs more
V4: ⊥ claim clean. report what was CHECKED — 2 mechanical rules of ~6 review questions

## §T TASKS

id|status|task|cites
T1|x|`unwired` + `negative-only` + `public_fns`|V1,V2
T2|.|`bbx review` verb over the last commit's diff|V4
T3|.|duplication check — new fn re-parsing what an existing fn parses|V1
T4|.|wire into `apply` so a finding blocks the commit until acknowledged|V3
