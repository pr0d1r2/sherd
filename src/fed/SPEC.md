# SPEC

## §G GOAL

Federation edges. `§F` table, parent→child, chain to a node.

## §V INVARIANTS

V1: `§F` row = `dir|owns|⊥owns|tokens`. 4 cells or ⊥ a row
V2: edge depth = parent + 1 exactly. ⊥ skip levels
V3: `⊥owns` ! present — positive lens decides DESCEND, negative one decides STOP. the negative is the byte that prevents loading
V4: literal `|` in a cell escaped `\|`. row splitting honors it
V5: `tokens` = `-` means UNRECORDED, ⊥ zero
V6: header row (`dir|owns|…`) ⊥ an edge
V7: `§F` parse stops @ next `## §` header
V8: ignore globs — `target/`, `.git/`, `node_modules/`, `.direnv/` ⊥ walked

## §T TASKS

id|status|task|cites
T1|x|`edges` parse w/ escape handling|V1,V4,V6,V7
T2|x|`chain` root→node|V2
T3|x|`discover` walk w/ ignores|V8
T4|.|depth+1 validation, cycle detect, DAG build|V2
T5|.|sibling lens exhaustive + disjoint check|V3
T6|.|`§N` derive from parent `§F`|V3

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`find_depth_violations` (LLM-authored, `bbx tdd`) hand-rolls its own recursive walk w/ ⊥ ignore globs ∴ descends `target/`, `.git/` — violates `.:V23`. also a 2nd walker in the module that already has `walk()`, the two-readings defect, in the file whose own B-log names it|reuse `discover()`. FIRST CAUSE WRONG: I recorded 'it was ⊥ in the step-2 surface'. VERIFIED FALSE — `discover()` & `walk()` were BOTH in the prompt. the model saw them & duplicated anyway ∴ cause is that nothing ASKED it to reuse. `.:V59` on my own bug record
B2|2026-08-01|LLM-authored test writes `temp_depth_test` in CWD, ⊥ a real temp dir ∴ races under parallel test runs & leaks the dir if the test panics before cleanup|`std::env::temp_dir()` + unique name, cleanup on drop
B3|2026-08-01|`find_depth_violations` reads EVERY `*.md`, ⊥ only `SPEC.md` ∴ a federation table in a README is treated as authoritative|scope to `SPEC.md`, per `.:V5`/`.:V1`
