# SPEC

## §G GOAL

Mechanical pre-review of what `apply` committed. Narrows what a reader ! catch.

## §C CONSTRAINTS

- ⊥ a substitute for reading the diff. it cannot tell whether code satisfies an INVARIANT — the failure that matters.
- ∀ check ! trace to a commit on this branch it would have caught. ⊥ speculative rules.

## §V INVARIANTS

V1: `unwired` — a `pub fn` called only from tests LANDED but was never wired in. searched over the WHOLE crate: a caller in a SIBLING node counts, & checking only the declaring module is a false positive (B1)
V2: `negative-only` — a detector whose test asserts only the empty case is satisfied by a fn that always finds nothing (`src/fed:V10`)
V3: a finding is ADVISORY. review reports; the reader judges. auto-reverting on a heuristic would trade a false negative for a false positive & the false positive costs more
V4: ⊥ claim clean. report what was CHECKED — 2 mechanical rules of ~6 review questions


## §T TASKS

id|status|task|cites
T1|x|`unwired` + `negative-only` + `public_fns`|V1,V2
T2|x|`bbx review [rev]` over a commit's diff|V4
T3|.|duplication check — new fn re-parsing what an existing fn parses|V1
T4|.|wire into `apply` so a finding blocks the commit until acknowledged|V3

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`unwired` searched only the DECLARING module ∴ flagged `find_exhaustive_violations` the moment `cli` started calling it. first real run of the verb, first finding, FALSE|search every non-test line of the crate. advisory design meant a false positive cost a reading, ⊥ a revert — which is why V3 says advisory
B2|2026-08-01|`unwired` counted occurrences & assumed "declaration + call" > 1. `pub fn f<'a>(` does ⊥ contain `f(` ∴ a generic decl counts 0 & a CALLED fn read as uncalled. own test used a non-generic fn ∴ never touched the branch|a CALL is any occurrence on a line that is ⊥ a declaration. 4th guard this session that passed w/o exercising what it guards
