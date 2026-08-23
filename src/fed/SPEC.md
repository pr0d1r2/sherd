# SPEC

## §G GOAL

Federation edges. `§F` table, parent→child, chain to a node.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
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

## §V INVARIANTS

V1: `§F` row = `dir|owns|⊥owns|tokens`. 4 cells or ⊥ a row
V2: edge depth = parent + 1 exactly. ⊥ skip levels
V3: `⊥owns` ! present — positive lens decides DESCEND, negative one decides STOP. the negative is the byte that prevents loading
V4: `\` escapes the NEXT char only when that char is `\` or `\|` — before anything else it is LITERAL & kept ∴ `C:\path` survives `edges()` & a cell may END in a backslash. a splitter consuming `\` before ANY char eats data silently, & leaves a cell ending in one unrepresentable, which V1 then drops as a non-row (B11, B12)
V5: `tokens` = `-` means UNRECORDED, ⊥ zero
V6: header row (`dir|owns|…`) ⊥ an edge
V7: `§F` parse stops @ next `## §` header
V8: ignore globs — `target/`, `.git/`, `node_modules/`, `.direnv/` ⊥ walked
V9: a `§T` row states REMAINING work, ⊥ history. "`X` landed, `Y` still open" reads to a machine as "test `X`" & `X` already passes ∴ ⊥ a red test (`.:plan` B2). what landed lives in the commit trail
V10: a detector's test ! include a POSITIVE case. asserting only that nothing was found is satisfied by a fn that always finds nothing (B6)
V11: sibling `§F` lenses ! EXHAUSTIVE — every child dir on disk appears as a row. a child absent from `§F` is unreachable by descent & invisible to a reader who trusts the table
V12: sibling `§F` lenses ! DISJOINT — 2 rows ⊥ name the same `dir`. a duplicate makes descent ambiguous & `route` would have to open both
V14: a finished `§T` row is HISTORY & rule depth LOADS `§T` ∴ every chain pays it every turn — MEASURED 106 done rows tree-wide, 45 @ root, & deleting them dropped root 12,420 → 9,922 & the repo 228,835 → 184,174 tok (19%). V9 said this & nothing read it until `check` counted. what a done row CARRIES moves 1st: a measurement → `§R`, which rule depth does ⊥ load ∴ the finding is kept FREE & only the row is paid (`.:R56` came out of a done root `T96` that way)
V13: a parser's test ! cover the char it CONSUMES, ⊥ only the sequence it documents. `escaped_pipe_stays_in_the_cell` covers `\|` & nothing covered a LONE `\` ∴ `split_row()` ate backslashes for the project's whole life behind a green suite (B11). B6 is the same shape — a case nothing asserts is a case that passes

## §T TASKS

id|status|task|cites
T4|~|cycle detection over the federation DAG|V2
T8|.|orphan check — a `SPEC.md` no parent `§F` row points at|`.:V3`
T9|.|needs a query→child match rule before it has a signature|`.:V19`
T10|.|promote an invariant from a leaf to the common ancestor|`.:V13`
T11|.|report `§N` that differs from what `§F` derives|`.:V36`
T12|.|BLOCKED — needs Rust source, ⊥ `§F` data. see B9|`.:V73`
T13|~|replace the hand-rolled walk with `itok::walk`/`itok::glob`|`.:V23`
T14|.|blocked — recomputing needs `crate::tokens`, ⊥ in this node's surface. see B10|`.:V21`
T15|.|fixture: 4 levels deep, one module with two parents — self-repo is a tree|`.:V4`
T16|.|parse `§N` rows, line-anchored|`.:V34`

## §B BUGS

id|date|cause|fix
B1|2026-08-01|FIXED by replacement. `find_depth_violations` (LLM-authored) hand-rolled its own recursive walk w/ ⊥ ignore globs ∴ descends `target/`, `.git/` — violates `.:V23`. also a 2nd walker in the module that already has `walk()`, the two-readings defect, in the file whose own B-log names it|reuse `discover()`. FIRST CAUSE WRONG: I recorded 'it was ⊥ in the step-2 surface'. VERIFIED FALSE — `discover()` & `walk()` were BOTH in the prompt. the model saw them & duplicated anyway ∴ cause is that nothing ASKED it to reuse. `.:V59` on my own bug record
B2|2026-08-01|FIXED by replacement. LLM-authored test wrote `temp_depth_test` in CWD, ⊥ a real temp dir ∴ races under parallel test runs & leaks the dir if the test panics before cleanup|`std::env::temp_dir()` + unique name, cleanup on drop
B3|2026-08-01|FIXED by replacement. `find_depth_violations` read EVERY `*.md`, ⊥ only `SPEC.md` ∴ a federation table in a README is treated as authoritative|scope to `SPEC.md`, per `.:V5`/`.:V1`
B4|2026-08-01|FIXED & hypothesis CONFIRMED. was twice — `check_edge_depth` & `missing_not_owns`, independent tasks, both re-scan the `§F` section itself — its own `in_f` loop & header skip — instead of calling `edges(text)` & checking `e.dir`. residual two-readings, milder than B1's duplicate walker but real|HYPOTHESIS: step 2 gets the FULL impl body ∴ sees `edges()`'s scan loop & IMITATES it. surface-by-example induces copying. VERIFIED: step 2 given SIGNATURES ⊥ bodies → `depth_violations(&[Edge])`, 5 lines ⊥ 30, COMPOSES w/ `edges()`, `in_f` 6→3. also 20% cheaper (max call 1,438→1,243)
B5|2026-08-01|`c56869e` DELETED `missing_not_owns` (committed 2 commits earlier). I overwrote `src/fed/mod.rs` w/ the pre-experiment file to isolate a variable & the commit swept the loss in. tests 22→21 & I read it as noise ∴ a REGRESSION shipped inside a commit whose message claimed only an improvement|restored by re-running V3 under the new config. GENERALLY: resetting a file to isolate an experiment DISCARDS everything else in it — diff against HEAD before committing an experiment's output, & a falling test count is a finding ⊥ noise
B6|2026-08-01|`sherd apply` committed `detect_cycles(edges) -> Vec::new()` UNATTENDED, w/ a doc comment saying "stub ... satisfies the current test suite". the test asserted only "no cycle in this simple graph" ∴ a fn that always finds nothing passes perfectly. judge approved (the test DID check depth), gates green, committed|(a) gate now compiles `-D warnings` — `unused variable: edges` is how a stub announces itself & would have blocked this exact commit; (b) judge ! require a POSITIVE case for a detector; (c) stub removed. GENERALLY: a detector tested only on the NEGATIVE case is satisfied by returning the negative
B7|2026-08-01|LLM repair reached for `scopeguard::guard` — a crate this repo does ⊥ depend on ∴ `E0433`, & the run was mid-repair when its wall-clock budget ran out|4 lines of local `Drop` replaced it. the model reaches for a crate rather than 4 lines; its surface shows the API but ⊥ the dependency list
B8|2026-08-01|`unused import: Path` in a test module COMMITTED through a `-D warnings` gate. `cargo build` ⊥ compile `#[cfg(test)]` code ∴ the flag never saw it. also `find_exhaustive_violations` is called ONLY by tests — `review::unwired`'s exact case, surfaced by my own check & skimmed past|`RUSTFLAGS` exported so BOTH `build` & `test` deny. T6 wires the fn into `check`. GENERALLY: a flag on one command is ⊥ a flag on the toolchain
B9|2026-08-01|`sherd apply` wrote `find_flat_rs_promotions` filtering `§F` rows on `dir == "." && owns.ends_with(".rs")`. NO `§F` row has `dir == "."` — the shape ⊥ exist. gates green, judge YES, test & impl agreed w/ each other & neither related to `.:V73`. REVERTED|the ROW was wrong, ⊥ the model: `.:V73` is a policy about when to create a dir & answering it needs Rust SOURCE, which `fed` ⊥ read. `classify` says actionable because it parses as "add one fn" ∴ shape is necessary & ⊥ sufficient
B10|2026-08-01|`sherd apply` wrote `find_token_mismatches` summing FILE SIZES in bytes & comparing them to `§F`.tokens. bytes ⊥ tokens; `.:R8` measured bytes/4 48% off on caveman text & §C says counting is `itok`'s job. gate refused the commit for an UNUSED IMPORT, ⊥ for being wrong — luck|discarded. same class as B9: the node cannot compute the invariant, so the model invents a proxy. row now names the blocker
B11|2026-08-21|`split_row()` treats `\` as an escape before ANY char & DROPS it, while V4 only ever defined `\|` ∴ a backslash anywhere in a cell is EATEN, & a cell ending in `\\` yields 5 cells where V1 demands 4, so `edges()` returns NO row & a `§F` edge vanishes w/ no error — B6's shape (something that finds nothing passes cleanly) one level over. MEASURED through `edges()` on a 3-row table: an `owns` of `C:\path notes` parsed as `C:path notes`, a row whose `owns` ended `tail\\` produced NO edge, & 2 of 3 rows survived. FIRST CAUSE WRONG, mine, one commit earlier: I wrote that `cell()` & `split_row()` are a broken CODEC PAIR & that `cell()`'s output makes an edge vanish. VERIFIED FALSE — `cell()`'s only caller is `table()` → `sherd graph --table`, a MARKDOWN render, & nothing parses its output back ∴ the EFFECTS were real & the MECHANISM I named was ⊥. `.:V59`, & B1 is the same correction on this node's own log. FOUND by the ambiguity detector (`.:R55`) flagging the `escape_cell` corpus row 3/3 (`.:R55`), followed from fixture into live code|V4 & V13 restated. T17 fixes `split_row()` & tests through `edges()`. ROOT CAUSE is UPSTREAM: `FORMAT.md` says "literal pipe → escape as backslash-pipe" & is SILENT on what escapes the escape ∴ 3 readings of 1 sentence — ours (any char, lossy), `microlith::id::cells` (a 2-char toggle) & `microlith::id::unescape` (1-char) ∴ its own splitter & decoder are ⊥ inverse either. specified there as `microlith/V38`/`B25` 2026-08-22, ⊥ patched locally (`.:V47`)
B12|2026-08-21|SPEC defect, mine, caught by T17's own test before any code shipped. B11's first restatement of V4 said `\` escapes ONLY before `\|` ∴ a cell ending in a backslash was UNREPRESENTABLE: `tail\\` reads as literal-backslash-then-escaped-pipe, the column break is swallowed & the row drops to 3 cells — the SAME vanishing V4 had just been rewritten to stop. the rule now escapes `\` & `\|` & nothing else, which keeps `C:\path` literal AND makes a trailing backslash expressible|V4 restated a 2nd time. GENERALLY: an escape scheme ! be able to express its own escape char, & the test that finds that is a ROUND TRIP over a cell that ENDS in it
