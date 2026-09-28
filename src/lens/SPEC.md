# SPEC

## §G GOAL

Lens pack — what one node costs to work at.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/lens|pack assembly, depth `rule`\|`why`, budget verdict
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/split|PROPOSE a federation — modules the code separated, the rows naming each, a home for an unmanaged row
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/adopt|foreign single-file spec → federation: row placement, citation rewrite, conservation
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/wave|the SCHEDULE a parallel build follows — code DAG, ready set per round, depth & width
sib|src/review|mechanical checks on what `apply` committed
sib|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it
sib|src/git|one git invocation shape — the repo a command acts on, & the env it refuses

## §V INVARIANTS

V1: pack self-contained @ its altitude. ⊥ require sibling|child body to act
V2: `Depth::Rule` default. rationale pulled on demand, ⊥ resident — entry cost re-billed EVERY turn
V3: pack = chain (root→node) + node body + child `§F` lens lines
V4: verdict states DIRECTION & DISTANCE — `Fits{slack}` | `Over{by}`, ⊥ a bare bool
V6: a ceiling comes from `.context-limits`, ⊥ a constant & ⊥ an invented file. an unlisted path falls back to `DEFAULT_NODE`, which is ⊥ the same as "no limit" (B1)
V5: node unreadable → error, ⊥ skipped. a pass on a node the parser never saw is indistinguishable from a real pass
V7: lens pack ordered STABILITY-DESCENDING — root, ancestors, then node. an edit invalidates every token of prefill AFTER it (`.:R15`) ∴ volatile content LAST. `pack()` order is load-bearing, ⊥ cosmetic
V8: FACET = stable classification of content (impl · tests · spec · agents · guard-infra · guard-local · human). a property of the FILE, fixed
V9: facet value = token share × P(task ⊥ needs it). tests 34% × ~0.7 ≈ 24% · human 2.7% × ~0.95 ≈ 2.6% ∴ rank by the PRODUCT, ⊥ by size
V10: more facets help ONLY where a facet matches how tasks cluster. a facet no task selects is a manifest to maintain — `.:R4` rejected that once already
V11: facets ! PARTITION — exhaustive & disjoint, same rule as sibling lenses (`.:V64`/`.:V65`). else content double-loads or vanishes between facets
V12: `AGENTS.md` ∈ SET, ⊥ SETTING. it says HOW to work ∴ needed while working. guardrails say what is CHECKED after ∴ ⊥ needed while working
V13: PROFILE = task-dependent SELECTION over facets. `set`/`setting` is the DEFAULT profile (`implement`), ⊥ a partition of the repo. `tests` ∈ setting under `implement` & ∈ set under `tdd` — the file ⊥ change, the TASK does
V14: axes COMPOSE multiplicatively. facet alone fails TDD — MEASURED 81.4% of itok still loads (`.:B3`). facet × horizontal @ one node = 6.1%, 13x smaller ∴ neither axis alone is sufficient
V15: profile DECLARED per task, ⊥ inferred. an inferred profile silently loads the wrong facets & the failure looks like a model that forgot
V16: pack layout = canonical facet ORDER, volatile last. widening mid-run re-prefills everything AFTER the insertion point ∴ declare the profile up front (V15) & build the pack once
V17: MEASURED widening cost — append 4.12s (only new tok, prefix cached) vs prepend 8.60s (everything) on the same 11.6k pack = 2.1x, & the gap grows w/ prefix size

## §T TASKS

id|status|task|cites
T3|.|`SPEC.why.md` resolution for `Depth::Why`|V2
T4|.|needs `sherd.toml` — no tier is declared anywhere yet|V4
T5|.|given a node dir, return its child dirs when the pack exceeds `ceiling_for`|`.:V9`
T6|.|needs facets represented in data — nothing marks them today|`src/lens:V8`
T7|.|needs facets represented in data — see T6|`src/lens:V11`
T8|.|needs a computable notion of volatile — undefined today|`src/lens:V16`
T10|.|profile declaration — flag > §T row > `sherd.toml` default. ⊥ inference|V15

## §B BUGS

id|date|cause|fix
B1|2026-08-01|row said "a node over `budget.node`" & the model read `budget.node` as a FILENAME — `fs::read_to_string(root.join("budget.node"))` — then returned `vec![root.join("hint")]`, a literal fake. 6 compile errors, discarded|name the QUANTITY ("a token ceiling"), ⊥ the config key. a dotted identifier in prose reads as a path
