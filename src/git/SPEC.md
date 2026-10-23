# SPEC

## §G GOAL

one shape for every git invocation: WHICH repository a command acts on, and which of the caller's environment it refuses. `.:B25` is the whole reason — nine call sites each wrote the same three lines, so nine were wrong the same way.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/git|one git invocation shape — the repo a command acts on, & the env it refuses
sib|src/tokens|-
sib|src/spec|-
sib|src/split|-
sib|src/fed|-
sib|src/adopt|-
sib|src/lens|-
sib|src/ollama|-
sib|src/tdd|-
sib|src/plan|-
sib|src/wave|-
sib|src/review|-
sib|src/state|-
sib|src/slice|-
sib|src/land|-
sib|src/cli|-
sib|src/code|-
sib|src/debt|-
sib|src/assay|-

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
