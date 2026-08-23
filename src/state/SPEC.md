# SPEC

## §G GOAL

one cached store. `kind key value`, one line each.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/lens|pack assembly, depth `rule`|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/review|mechanical checks on what `apply` committed
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it

## §V INVARIANTS

V1: a KEY ⊥ contain the delimiter. `value` MAY (`obs` carries a telemetry line) ∴ the parser splits 3 & a spaced key truncates (B1). escape on write, unescape on read
V2: `set` → `save` → `load` → `get` ! return what was set. a reader & writer that disagree lose data w/ no error (`.:fed:V13`)

## §T TASKS

id|status|task|cites
T1|.|`trim_kind` keeps N newest over an UNORDERED map ∴ "newest" is undefined. state the ordering|V2

## §B BUGS

id|date|cause|fix
B1|2026-08-21|`splitn(3, ' ')` ∴ a key w/ a space parses as its FIRST WORD. every loop label has one (`1 red-test`, `2 green`) ∴ the per-label pace model was WRITE-ONLY from day one & every step fell back to `EXPECT_GEN` 13 where the 20B emits ~2,300, mostly reasoning. eta off ~175x ∴ the 4x abort kills step 1 & the loop ⊥ reach step 2 — `.:tdd:T13`'s zero, one layer under `.:tdd:V29`. FOUND by RUNNING it|V1,V2. keys escaped, round trip asserted
