# SPEC

## §G GOAL

the SCHEDULE a parallel build follows: the code DAG, the ready set per round, and the 2 numbers that make the case — DEPTH & WIDTH. report-only.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/wave|the SCHEDULE a parallel build follows — code DAG, ready set per round, depth & width
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/adopt|foreign single-file spec → federation: row placement, citation rewrite, conservation
sib|src/lens|pack assembly, depth `rule`\|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/review|mechanical checks on what `apply` committed
sib|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it
sib|src/git|one git invocation shape — the repo a command acts on, & the env it refuses

## §C CONSTRAINTS

- REPORTS, ⊥ runs. WHO writes the code is a named, swappable executor & is ⊥ built here (`.:V117`).
- reads Rust source through `src/code` & the tree through `src/fed`. re-reading either is the 2-readings defect (`src/code:V1`).
- takes the DAG it is HANDED where it can (`src/cli:V6`): `schedule()` is pure & needs no tree, which is why its rules are testable w/o a fixture.

## §I INTERFACES

- lib: `code_deps(&Path) -> Vec<CodeDep>` — per node, the SIBLING nodes its `use crate::` lines name
- lib: `schedule(&[CodeDep]) -> Schedule` — ready set per round, pure
- lib: `wave(&Path, &Path) -> Schedule` — the schedule a wave over one dir would follow
- lib: `Schedule::depth()` · `Schedule::width()` — rounds, & the most workers ever busy at once

## §V INVARIANTS

V1: the CODE dag is ⊥ the FEDERATION dag. code edge = `use crate::` between SIBLING nodes; federation edge = a `§F` row, parent→child ∴ 2 graphs over 1 tree & conflating them ships a schedule nobody can run — `§F` makes co-children read INDEPENDENT when the code makes one wait for the other, & declares parent→child edges the code ⊥ have. a CYCLE in the code dag is legal Rust ∴ NAMED & exit 0, where `.:V4`'s federation cycle is exit 1: 1 word, 2 graphs, 2 verdicts. MOVED here w/ the code it governs, from the rule the planner carried while the scheduler lived there
V2: a dependency LEAVING the scope is DROPPED, ⊥ left unsatisfiable. keeping it reports every scoped node blocked — a cycle report for a tree w/ no cycle
V3: a schedule is STABLE between runs ∴ labels sort. a report whose rounds reorder cannot be diffed, & the 2 numbers are what a reader compares

## §T TASKS

id|status|task|cites
T1|.|the EXECUTOR half — frozen til rung 0.7 (`.:V117`). it lands HERE, ⊥ in the planner|`.:V123`
T2|.|`wave --json` — the rounds as data, for a runner that is ⊥ a human|`.:V83`

## §B BUGS

id|date|cause|fix
