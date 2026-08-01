# SPEC

## §G GOAL

Lens pack — what one node costs to work at.

## §V INVARIANTS

V1: pack self-contained @ its altitude. ⊥ require sibling|child body to act
V2: `Depth::Rule` default. rationale pulled on demand, ⊥ resident — entry cost re-billed EVERY turn
V3: pack = chain (root→node) + node body + child `§F` lens lines
V4: verdict states DIRECTION & DISTANCE — `Fits{slack}` | `Over{by}`, ⊥ a bare bool
V5: node unreadable → error, ⊥ skipped. a pass on a node the parser never saw is indistinguishable from a real pass

## §T TASKS

id|status|task|cites
T1|x|`pack` assembly, chain + children + cost|V1,V3
T2|x|`Depth` rule\|why, `verdict`|V2,V4
T3|.|`SPEC.why.md` resolution for `Depth::Why`|V2
T4|.|budget derived from declared target tier, ⊥ constant|V4
