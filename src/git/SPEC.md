# SPEC

## §G GOAL

one shape for every git invocation: WHICH repository a command acts on, and which of the caller's environment it refuses. `.:B25` is the whole reason — nine call sites each wrote the same three lines, so nine were wrong the same way.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/git|one git invocation shape — the repo a command acts on, & the env it refuses
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/adopt|foreign single-file spec → federation: row placement, citation rewrite, conservation
sib|src/lens|pack assembly, depth `rule`|`why`, budget verdict
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

## §C CONSTRAINTS

- BUILDS a `Command`, ⊥ runs it. the caller chooses `output()`/`status()` & owns the error type ∴ this node needs no fixture, no repo & no subprocess to test.
- ⊥ knowledge of any verb. `plan`/`review`/`land` decide what git DOES; this decides only where it points.

## §I INTERFACES

- lib: `at(&Path, &[&str]) -> Command` — aimed at that root & no other repo
- lib: `anywhere(&[&str]) -> Command` — args name the repo (`--git-dir`), no cwd
- lib: `INHERITED: [&str; 8]` — the exported vars both forms remove

## §V INVARIANTS

V1: `current_dir` is ⊥ isolation. an inherited `GIT_DIR` OVERRIDES directory discovery ∴ cwd alone aims nothing (`.:B25`)
V2: git exports `GIT_DIR` to hooks ABSOLUTE when the checkout is a WORKTREE & unset otherwise ∴ a defect here is invisible on a normal clone & fires only in a worktree
V3: a new git call site goes through this node. a bare `Command::new("git")` elsewhere is the duplication `.:B25` recorded

## §T TASKS

id|status|task|cites
T1|.|gate V3 — a `Command::new("git")` outside this node should fail the build, ⊥ wait for a reader|V3

## §B BUGS

id|date|cause|fix
