# SPEC

## §G GOAL

`src/` — hub. code nodes of `bbx`. ⊥ own logic itself.

## §F FEDERATION

dir|owns|⊥owns|tokens
tokens|`itok` facade, counts w/ method label, entry cost, working budget|spec structure, federation edges|-
spec|`cavespec` facade, §-section split, structural check, fmt|token counts, `§F`/`§N`|-
fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery|counting, rendering|-
lens|pack assembly, depth `rule`\|`why`, budget verdict|parsing, counting internals|-
ollama|local endpoint client, `num_ctx`, fence extraction|prompt construction, loop control|-
tdd|red→judge→green→gate→repair loop, source region edits|HTTP, token counting|-

## §C CONSTRAINTS

- module = dir + `mod.rs`. `mod.rs` composes, ⊥ implements (`.:V51`).
- ∀ external dep ! ONE facade node. siblings private ∴ compiler enforces (`.:V72`).

## §V INVARIANTS

V1: `main.rs` dispatch only, ⊥ logic
V2: node here ⊥ dep sibling node except through its public surface
