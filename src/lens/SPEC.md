# SPEC

## §G GOAL

Lens pack — what one node costs to work at.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/lens|pack assembly, depth `rule`|`why`, budget verdict
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/review|mechanical checks on what `apply` committed
sib|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it

## §V INVARIANTS

V1: pack self-contained @ its altitude. ⊥ require sibling|child body to act
V2: `Depth::Rule` default. rationale pulled on demand, ⊥ resident — entry cost re-billed EVERY turn
V3: pack = chain (root→node) + node body + child `§F` lens lines
V4: verdict states DIRECTION & DISTANCE — `Fits{slack}` | `Over{by}`, ⊥ a bare bool
V6: a ceiling comes from `.context-limits`, ⊥ a constant & ⊥ an invented file. an unlisted path falls back to `DEFAULT_NODE`, which is ⊥ the same as "no limit" (B1)
V5: node unreadable → error, ⊥ skipped. a pass on a node the parser never saw is indistinguishable from a real pass

## §T TASKS

id|status|task|cites
T1|x|`pack` assembly, chain + children + cost|V1,V3
T2|x|`Depth` rule\|why, `verdict`|V2,V4
T3|.|`SPEC.why.md` resolution for `Depth::Why`|V2
T4|.|needs `sherd.toml` — no tier is declared anywhere yet|V4
T5|.|given a node dir, return its child dirs when the pack exceeds `ceiling_for`|`.:V9`
T6|.|needs facets represented in data — nothing marks them today|`.:V78`
T7|.|needs facets represented in data — see T6|`.:V81`
T8|.|needs a computable notion of volatile — undefined today|`.:V89`

## §B BUGS

id|date|cause|fix
B1|2026-08-01|row said "a node over `budget.node`" & the model read `budget.node` as a FILENAME — `fs::read_to_string(root.join("budget.node"))` — then returned `vec![root.join("hint")]`, a literal fake. 6 compile errors, discarded|name the QUANTITY ("a token ceiling"), ⊥ the config key. a dotted identifier in prose reads as a path
