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
sib|src/adopt|foreign single-file spec → federation: row placement, citation rewrite, conservation
sib|src/lens|pack assembly, depth `rule`|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/review|mechanical checks on what `apply` committed
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it
sib|src/git|one git invocation shape — the repo a command acts on, & the env it refuses

## §V INVARIANTS

V1: a KEY ⊥ contain the delimiter. `value` MAY (`obs` carries a telemetry line) ∴ the parser splits 3 & a spaced key truncates (B1). escape on write, unescape on read
V2: `set` → `save` → `load` → `get` ! return what was set. a reader & writer that disagree lose data w/ no error (`.:src/fed:V13`)
V3: a CAPPED log ! carry its own ORDER in the key. eviction reads the key & nothing else ∴ a key that ⊥ encode time evicts by accident — & a content hash is the natural key here ∵ it is also the idempotence guard. seq PREFIX, zero-padded, & an UNSEQUENCED key sorts OLDEST (`None` < `Some`, which is true: it was written 1st). a TIMESTAMP is still refused ∵ it changes the file when nothing changed; the dedup is what makes a clock unnecessary — a repeat append is a no-op ∴ the file is byte-identical

## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
B1|2026-08-21|`splitn(3, ' ')` ∴ a key w/ a space parses as its FIRST WORD. every loop label has one (`1 red-test`, `2 green`) ∴ the per-label pace model was WRITE-ONLY from day one & every step fell back to `EXPECT_GEN` 13 where the 20B emits ~2,300, mostly reasoning. eta off ~175x ∴ the 4x abort kills step 1 & the loop ⊥ reach step 2 — `.:src/tdd:T13`'s zero, one layer under `.:src/tdd:V29`. FOUND by RUNNING it|V1,V2. keys escaped, round trip asserted
B2|2026-08-24|`trim_kind` kept N by dropping the LEXICALLY lowest keys, & the one real caller (`src/ollama::record_obs_in`) keys by CONTENT HASH ∴ eviction was by hash, ⊥ by age. the retained set was a hash-sampled slice of ALL HISTORY: MEASURED over 700 observations capped @ 200, 62 survivors came from the FIRST 200 & only 63 from the last 200 ∴ day-one telemetry outlived recent runs at equal weight & the pace model stopped learning. the TEST agreed w/ the impl ∵ its fixture keyed `00`..`09`, zero-padded, where lexical order IS age — a fixture no caller produces (`src/tdd:B2`'s shape, in our own suite). the row said only "newest is undefined"; the consequence was ⊥ measured until asked|V3. `push_kind(kind, dedup, value, cap)` — seq-prefixed key, dedup by suffix, evict oldest-first. `trim_kind` DELETED, ⊥ deprecated: `push_kind` supersedes it & a 2nd way to bound a log is the 2-readings defect. 2 defects found while fixing it, both by testing the MIGRATION: `.last()`'s seq restarted @ 0 forever ∵ a legacy bare hash `ff78..` sorts AFTER `00000000-..` (→ MAX over keys), & the same lexical order evicted the NEWEST first (→ order by seq, `None` 1st)
