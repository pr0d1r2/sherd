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
