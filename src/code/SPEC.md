# SPEC

## §G GOAL

read Rust source AS TEXT. the mirror of `.:spec`, which reads `SPEC.md` as text — root `§G` already pairs the two, & now the code does.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/adopt|foreign single-file spec → federation: row placement, citation rewrite, conservation
sib|src/lens|pack assembly, depth `rule`\|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/wave|the SCHEDULE a parallel build follows — code DAG, ready set per round, depth & width
sib|src/review|mechanical checks on what `apply` committed
sib|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it
sib|src/git|one git invocation shape — the repo a command acts on, & the env it refuses

## §C CONSTRAINTS

- ∀ fn PURE over `&str`. ⊥ IO, ⊥ model, ⊥ subprocess ∴ a test needs no fixture dir, no endpoint, no toolchain.
- line-oriented heuristics, ⊥ a real parse. `syn` is a dep tree this repo ⊥ take for 5 fns, & the §B rows below are the price of that choice, stated ⊥ hidden.
- ONE owner. `.:tdd` & `.:review` each had their own reading before (`.:B13`) — that is why this node exists.

## §I INTERFACES

- lib: `split_module(&str) -> (&str, &str)` — the CUT: where the 1st test region begins. a POSITION, for `tdd`'s region write & `review`'s per-side read
- lib: `split_regions(&str) -> (String, String)` — the MEASURE: EVERY non-test region, every test region (V6, `.:B29`)
- lib: `public_fns(&str) -> Vec<String>` — declared `pub fn` names
- lib: `fn_names(&str) -> Vec<String>` — every `fn`, ⊥ only the public ones
- lib: `fn_body(&str, name) -> Option<&str>` — one fn's body, braces matched
- lib: `is_called(&str, name) -> bool` — called outside its own declaration
- lib: `expected_calls(test, existing) -> Vec<String>` — calls a test makes that ⊥ exist yet
- lib: `signatures(&str) -> String` — public surface, docs kept, bodies dropped
- lib: `test_decls(&str) -> String` — the test names a region declares
- lib: `literal_indexes(&str) -> Vec<(String, usize)>` — literal index expressions & where
- lib: `markers(&str) -> Vec<String>` — the stub words a body admits to
- lib: `mod_decls(&str) -> Vec<ModDecl>` — `mod x;` declarations & their visibility
- lib: `crate_uses(&str) -> Vec<String>` — WHICH sibling a file reaches for (V5)
- lib: `crate_imports(&str) -> Vec<Import>` — WHAT it names there: module + last segment (V5)
- lib: `public_types(&str) -> Vec<PubType>` — `pub` struct|enum|trait|type, the names a sibling can spell
- lib: `types_in(&[String]) -> Vec<PubType>` — the vocabulary of a NODE: pooled, deduped, sorted

## §R RESEARCH

id|topic|finding|src
R1|why 5 fns & ⊥ a parser|the whole node is ~156 lines of `&str` work. a syntax-tree dep would be ⊥ zero-cost & ⊥ zero-dep, & every §B here is a heuristic edge a parser would ⊥ have hit ∴ the trade is REAL & re-decidable if the list grows|measured @ promotion

## §V INVARIANTS

V1: `split_module` matches @ COLUMN 0 only. a fixture string carrying `"#[cfg(test)]\nmod t {"` ⊥ split the file — `.:tdd`'s corpora carry exactly that
V2: `is_called` searches the WHOLE crate, ⊥ the declaring module. a `pub fn` called from a sibling node is WIRED (`.:src/review:B1`)
V3: a GENERIC declaration `fn f<'a>(` ⊥ contain `f(` ∴ ⊥ count occurrences & assume "declaration + 1" (`.:src/review:B2`)
V4: a heuristic here ! carry the case that broke it as a TEST. the fn is 10 lines; the reason it is 10 & ⊥ 3 is the §B row it answers
V5: `crate_uses()` answers WHICH sibling a file reaches for; `crate_imports()` answers WHAT it names there — module + the LAST segment of each branch. a caller asking whether an import is a TYPE reference needs the 2nd (`src/wave:V4`), & a path this line-oriented reader cannot resolve is dropped ∴ the edge counts as BLOCKING, the safe direction
V6: a CUT is ⊥ a MEASURE. `split_module()` answers WHERE the 1st test region starts — a POSITION, & `tdd`/`review` EDIT against it; `split_regions()` answers HOW MUCH of a file is code — a TOTAL over every region. 1 fn serving both reads production code below a test module as test weight (`.:B29`), & the halves are ⊥ contiguous ∴ the measure ! own its strings. a region opens on `#[cfg(test)]` @ col 0 (V1) & closes @ the next `}` @ col 0 — the ITEM, `mod` | `fn` alike

## §T TASKS

id|status|task|cites
T2|.|audit `.:review` for parse logic still hiding in a FINDING builder — `unwired` gave up `is_called` & may ⊥ be the only one|V2,V3
T4|.|relocate the OWNING rows: `.:src/tdd:V7` (split_module) · `.:src/tdd:B13`/`B18` (expected_calls) · `.:src/review:B1`/`B2` (call detection). they constrain code that lives HERE now, & a reader of this node cannot see why `expected_calls` is 75 lines without them (V4). NOTE this LOWERS `.:tdd` & raises this chain — measure both (`.:V110`)|V4,`.:V110`
T5|x|`seam` — per node dir, the `pub` types a sibling names; emit the vocabulary a parallel build needs, report-only|`.:R57`

## §B BUGS

id|date|cause|fix
B1|2026-09-19|`crate_uses` took the LEADING ident after `use crate::` ∴ a BRACE GROUP begins w/ `{` & yielded NOTHING. MEASURED on this crate: 4 such lines, & `src/cli` names 8 siblings on one of them ∴ read as reaching for ⊥ & lands in the 1st ready set of any graph built from this. unseen ∵ the only consumer was the family COHESION column & no repo dogfooded HAS a family|V4. split the group on `,` & take each part's leading ident. the test carries the `use crate::{fed, spec};` shape verbatim
