# SPEC

## §G GOAL

TDD as separate round-trips — red, judge, green, local gate, capped repair.

## §C CONSTRAINTS

- ∀ step declares its profile. ⊥ one call sees spec + tests + impl together (`.:V87`).
- step 3 = LOCAL, deterministic, ZERO tokens (`.:V18`).

## §V INVARIANTS

V1: step 2 & 4 ⊥ touch the test region. STRUCTURAL — `insert_impl` writes above `#[cfg(test)]` only ∴ ⊥ a matter of asking nicely
V2: RED test ! fail at an ASSERTION, ⊥ at compile. a test that ⊥ build is ⊥ a red test
V3: judge sees invariant + test, NEVER the impl. guards self-consistent wrongness — one model writing both test & impl to match its own misreading
V4: test already green → STOP & restore. ⊥ a red test, nothing to drive
V5: repair budget capped. exhaustion → report what was TRIED, ⊥ silent give-up
V6: test ! cite a declared `§V` id. a test for an invariant that ⊥ exist encodes an unstated rule
V7: `split_module` has ONE definition here. the code ceiling (`.:V50`) ! reuse it — 2 readings of one rule is the defect this project ends

## §T TASKS

id|status|task|cites
T1|x|`split_module`, `insert_test`, `insert_impl` w/ structural test-region guard|V1,V7
T2|x|red → judge → green → gate → repair loop|V2,V3,V4,V5
T3|x|invariant-exists precondition|V6
T4|.|assert RED fails at assertion ⊥ at compile|V2
T5|.|`§T` row status flip on green (`.` → `x`)|V5
