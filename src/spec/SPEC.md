# SPEC

## §G GOAL

`SPEC.md` structure. Sole call site for `microlith`.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
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
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it

## §C CONSTRAINTS

- ⊥ reimpl parse, fmt, id, citation check — `microlith` owns them, zero-dep pure fn (R3).
- `microlith::` named here & nowhere else (`.:V72`).

## §V INVARIANTS

V1: what `microlith` owns is ADAPTED, ⊥ reimplemented. two implementations of one format is the defect `microlith` exists to end
V2: `§F`/`§N` ⊥ belong here — they are sherd additions, they live in `fed`
V3: parity vs `microlith` ! be MEASURED before claiming absorption (`.:V59`). currently BELIEF
V5: what is imported from `microlith` is what its ROOT re-exports. an inner module path is ⊥ contract — the dep may privatize it & does (E0603, B1). same for RENDERING: print the dep's own `Display`, ⊥ recompose its parts
V4: a `§B` row whose fix names no `§V` is UNREFLECTED — pain w/o reflection. it will recur & the log becomes a list of things that happened, ⊥ a set of guards. advisory: some bugs warrant no rule & forcing one manufactures invariants to silence a gate
V6: a SCAFFOLD emits ⊥ ids & ⊥ inference. a seeded `T1` is `T1` FOREVER (ids monotonic, never reused) & a task citing a placeholder rule is a spec that lies from commit one ∴ prompts are PROSE a human replaces. `§G` is never guessed from a dir name, & an `§F` row's owns/⊥owns are prompts, ⊥ a model's guess. its own test is `check` on the output: a generator whose output its checker rejects has shipped a 2nd dialect — & that test caught the prompt "numbered from V1" being read as a CITATION

## §T TASKS

id|status|task|cites
T1|x|`check`, `fmt`, `sections` binding|V1
T2|.|capability-parity audit vs `microlith`, written down|V3
T3|.|`--records` baseline wiring for closed-option survival|V1
T4|.|check closed-option records survive an edit, via `--records`|`.:V44`
T5|.|`SPEC.why.md` format — one rationale per `§V`/`§B` id|`.:V43`
T6|x|`unreflected_bugs` — `§B` rows naming no invariant|V4
T7|x|V5 runner — `microlith` reached only through its root, in every `src/` file|V5

## §B BUGS

id|date|cause|fix
B1|2026-08-05|facade re-exported through the dep's INNER path — `microlith::violation::{Violation, NAMESPACE}` — & `check` recomposed `{NAMESPACE}/{rule}: {msg}`, which `Violation: Display` already prints. the dep trimmed its lib to root verbs ∴ E0603 & a HEAD that ⊥ compile (`.:B5`). the recomposition was a 2nd reading of an id the dep owns — V1 applied to output, ⊥ only to logic|import `microlith::Violation`; print `{v}` w/ the caller's coords prefixed. V5, runner @ T7
T8|x|`scaffold(dir, children)` — the `SPEC.md` skeleton: 4 sections, `§F` rows from child dirs, ZERO ids & no inference. its test is `check` on its own output, ⊥ a string compare|V6
T9|x|`upsert_section` — the ONE writer for a generated section: replaces in place or inserts after an anchor, ⊥ appends. FORMAT fixes the order ∴ a section appended below `§B` is in the wrong place the moment it is written|V6
