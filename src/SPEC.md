# SPEC

## §G GOAL

`src/` — hub. code nodes of `sherd`. ⊥ own logic itself.

## §F FEDERATION

dir|owns|⊥owns|tokens
tokens|`itok` facade, counts w/ method label, entry cost, working budget|spec structure, federation edges|-
spec|`microlith` facade, §-section split, structural check, fmt|token counts, `§F`/`§N`|-
fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery|counting, rendering|-
lens|pack assembly, depth `rule`\|`why`, budget verdict|parsing, counting internals|-
ollama|local endpoint client, `num_ctx`, fence extraction|prompt construction, loop control|-
tdd|red→judge→green→gate→repair loop, source region edits|HTTP, token counting|-
plan|open `§T` rows, horizon, confidence, `apply` one step|writing code, judging it|-
review|mechanical checks on what `apply` committed|reading the diff, judging intent|-
state|one idempotent cached store — pace, telemetry, applied rows|everything else|-
slice|distil a document to the part needed to ACT, generated|judging what the slice says|-
land|run branch → `main` when believability earns it|writing code, judging it, reading the diff|-
cli|arg dispatch, usage, exit codes|every verb's logic|-
code|read Rust source as text — split, public fns, call detection, signatures|judging what it reads, `SPEC.md` structure|-
debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way|deciding WHAT to measure, running the gate|-
assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it|writing code with a model, judging a diff|-

## §N NAV

rel|path|lens
up|.|-
self|src|code nodes — tokens, spec, fed, lens facades & logic
sib|dev|repo-maintaining tooling, `publish = false` — README generation

## §C CONSTRAINTS

- module = dir + `mod.rs`. `mod.rs` composes, ⊥ implements (`.:V51`).
- ∀ external dep ! ONE facade node. siblings private ∴ compiler enforces (`.:V72`).

## §V INVARIANTS

V1: `main.rs` dispatch only, ⊥ logic
V2: node here ⊥ dep sibling node except through its public surface
