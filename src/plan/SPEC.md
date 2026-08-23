# SPEC

## §G GOAL

What to attempt next, and why it might ⊥ survive contact.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/lens|pack assembly, depth `rule`|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/review|mechanical checks on what `apply` committed
sib|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/cli|arg dispatch, usage, exit codes
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it

## §C CONSTRAINTS

- horizon 3. beyond that is fiction — applying a task edits the SPEC that plans the next one.
- classify conservatively. an unactionable row wrongly attempted burns a run & edits source.

## §V INVARIANTS

V1: ∀ step carries its CONFIDENCE & what INVALIDATES it. a plan that ⊥ say how it fails is a promise
V2: confidence DEGRADES w/ distance — next · likely · tentative
V3: unmanaged rows LISTED, ⊥ hidden. 58 of 72 open rows are ⊥ machine-actionable & silence would read as coverage (`.:V48`)
V4: ordering signal is node DEPTH only. `§T`.cites points at `§V`, ⊥ at another `§T` ∴ stated as weak, ⊥ dressed up
V5: root row ⊥ actionable — no `mod.rs` to add to
V6: plan ⊥ mutate source. it reads & reports; `apply` is the only writer
V12: `apply` runs `review` on what it just committed & surfaces any disagreement w/ the gate. a check nobody invokes is a check that ⊥ run — `review` existed for hours & caught nothing because it was typed by hand
V8: steps ordered by BELIEVABILITY — a node's measured keep-rate — then depth. `src/fed` failed 3x & kept supplying step 1 because depth was the only signal
V9: Laplace-smoothed `(kept+1)/(tried+2)` ∴ an untried node scores 0.50 & outranks 3 failures (0.20) w/o pretending to be known-good
V10: `kept` counts what survived REVIEW, ⊥ what passed the gate. the gate has gone green on 3 stubs ∴ counting commits measures the wrong thing
V11: a principle a machine can CHECK belongs in a gate, ⊥ a prompt. MEASURED strengths: structure (`insert_impl` cannot touch the test — never violated) > gate (`-D warnings` — evaded twice w/ `_`) > prompt ("reuse existing functions" — ignored, produced worse code)
V13: POSITION words ("around", "wrap", "inside") name where new code goes RELATIVE to existing code ∴ they mark a row that ! EDIT a call site, which the loop cannot do. a row can read as "add one function" & still be undrivable (B9). n=1 in this corpus — the rule is derived from ONE measured failure, ⊥ a survey
V14: a fix ! be tested on the EXAMPLE its own `§B` row names. B8 recorded "missed `wiring` (list had `wire`)" & shipped a stem match that still ⊥ match `wiring` (B11) ∴ the row read as closed for 3 weeks. the bug row's example is a test case already written down — ⊥ using it is discarding the one input known to reproduce
V15: a VOCABULARY is DERIVED, ⊥ listed. `VOCAB` knows 9 of 17 nodes & silently misses ∀ node added since it was written — the drift `sherd check` exists to catch, in the checker's own source. route reads the tree: dir name + `§G` words, ≥4 chars (shorter ones are caveman prose's articles & match everything). MEASURED while building: `sections()` returns the WHOLE heading `## §G GOAL` ∴ a `starts_with("§G")` matched nothing & ∀ node quietly reduced to its dir name — & the test asserting "words ⊥ empty" PASSED on dir names alone, which is why it now asserts a known `§G` word
V16: a SPLIT is PROPOSED, ⊥ applied. which module owns which rule is a judgement over prose, & a tool moving §V rows on its own rewrites law it cannot read. the weight is a HEURISTIC & says so: rows naming 2 modules count for both ∴ columns overlap & ⊥ sum to the chain. grounded ⊥ invented — MEASURED on `rekall`, whose flat spec names `check`·`trigger`·`corpus` on 17 lines each
V17: a federation is proposed from the CODE'S OWN STRUCTURE, ⊥ from prose. 3 grades of evidence, strongest 1st: (1) already a DIRECTORY — the author drew it · (2) `pub mod` — the author PUBLISHED it ∴ a boundary that already exists · (3) a naming FAMILY + `use crate::` cohesion — grouped implicitly. spec rows attach to proposed nodes AFTER & are EVIDENCE about a node, ⊥ the thing that proposes it. MEASURED on `itok`: 2 dirs · 7 `pub mod` · 11 `*cmd` sharing `cli`+`render` = ~12 nodes read off the crate, & 35 flat `mod` declarations that a row-count ranking scattered
V7: output names commands that EXIST. `plan` IS replan — it is stateless & re-derives every run ∴ saying "REPLAN" invents a second name for one operation, which is the two-readings defect (B3)

## §T TASKS

id|status|task|cites
T1|x|`open_tasks`, `classify`, `plan` w/ horizon + confidence|V1,V2,V5
T2|x|unmanaged rows reported w/ reason|V3
T3|.|`needs` column in `§T` so ordering is declared ⊥ guessed|V4
T4|.|machine-actionable marker in `§T` so classify ⊥ heuristic|V5
T5|.|needs `crate::ollama` — cross-node, name it before driving|V1
T6|.|`sherd plan \| head` panics on broken pipe. handle SIGPIPE|V6
T7|.|needs Rust source to say WHERE to cut — not this node's data|`.:V54`

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`classify` matched `derive \`§n\`` while the row read `` `§N` derive from parent `§F` `` ∴ a multi-file task planned as an actionable single-node fn. caught by READING the first plan, ⊥ by a test|word-order independent match. but widening a substring list is a PATCH — T4's declared marker is the fix, & prose classification stays wrong-by-default until then
B2|2026-08-01|first real `apply` refused: "test passes already". the `§T` row read "`depth_violations` landed ... cycle detect still open" ∴ the model tested `depth_violations`, which EXISTS, & it passed. a row describing what is DONE misleads a machine reading it as work|`§T` states REMAINING work only (`src/fed:V9`). history → commit trail. the loop was right & the input was wrong — & the refusal is `.:tdd` V4 working
B3|2026-08-01|`apply` said "REPLAN before the next step" & `plan` printed "REPLAN after each apply" — a verb that reads as a command name & is ⊥ one. user tried `sherd replan`, got usage. `plan` IS replan: stateless, re-derives every run|say "run `sherd plan` again". ⊥ an alias — a 2nd name for one operation is the defect, ⊥ the fix
B4|2026-08-01|`classify` was a BLACKLIST — actionable = ⊥ matching known-bad shapes ∴ "replace the hand-rolled walk" & "promote an invariant to the ancestor" both read actionable. the loop only APPENDS (`insert_impl`) & edits ⊥ spec files ∴ both are structurally undrivable & each would burn a cycle & commit something wrong|whitelist the ADD shape; `Replaces` & `NotAFunction` kinds. found by reading what `plan` would hand a run, ⊥ by running it
B5|2026-08-01|the new `Replaces` list matched SUBSTRINGS ∴ "report" contains "port" & every `report ...` row — the most common actionable shape — classified as a replacement|match whole WORDS. caught by my own test in the same commit that introduced it, which is the only reason it cost nothing
B6|2026-08-01|`cited_invariant` parsed only BARE ids ∴ every row moved down — all of which cite the namespaced `` `.:V73` `` form `.:V11` requires — was undrivable, & `drive` looked for the invariant in the NODE's spec when `.:` means ROOT. the moves made rows citation-correct & apply-incompatible in one step|parse `owner:id`, resolve `.` to root; `drive_from` reads the invariant from the owning spec. found by the FIRST cycle of a supervised run, which is what a supervised run is for
B7|2026-08-01|marked a row "BLOCKED" in its text & `plan` handed it back as step 1 — prose is ⊥ a status. also `git revert -q` is ⊥ a valid flag ∴ a revert I reported as done never ran & I committed on top of code I had declared gone|`blocked` joins the ⊥-actionable words. GENERALLY: report a revert only after checking the code is GONE, ⊥ after the command returns
B8|2026-08-01|AUDIT of all 21 actionable rows: only 5 are computable by the node that holds them. 12 fail — needing another node's data, a file that ⊥ exist, or ⊥ being a function at all. `classify` passed every one because they parse as "add one fn". also missed "wiring" (list had "wire") ∴ exact-word matching|stem matching; 12 rows reworded to name their blocker. the actionable COUNT was measuring shape, ⊥ drivability, & I reported 24 as ready
B10|2026-08-02|B8 RECURRED. B8's audit reworded 12 undrivable rows & `src/ollama` T3 survived it: "retry w/ bounded backoff around `Transport::post`" parses as add-one-fn, every verb check passed, & it needs an EDIT to `generate_via`'s call site. `sherd tdd` ran it TWICE — 8 round-trips, 17,551 tok, 4 compile errors (`E0428` redefined: model added a 2nd `generate_via_with_retry` beside the 1st, exactly the duplication `.:V4` predicts)|position words join `Replaces`. GENERALLY: B8 said the count measured SHAPE ⊥ drivability & the fix reworded rows instead of teaching the classifier ∴ a record, ⊥ a runner (`.:V74`). V13
B11|2026-08-21|`classify`'s stem match is `w.starts_with(k)` w/ stems spelled in FULL ∴ every stem ending in `e` fails its own `-ing` form — MEASURED 9 of 18: replace/removing/migrate/rewrite/delete/supersede/promote/wire/move. "wiring" is the EXACT word B8 names as the miss it was fixing, so B8's recorded fix never covered its own example & rows saying "replacing the walk", "moving the corpus" or "wiring X into Y" have read as ACTIONABLE ever since — B8's failure mode, still live, behind a §B row claiming it closed. FOUND by writing the test that asserts B8's claim, ⊥ by any run|V14. stems now matched w/ a trailing `e` trimmed, & the test asserts the `-ing` form of every stem
T8|x|`route(root, query)` — vocabulary DERIVED from ∀ node's dir name + its `§G` words, ⊥ a hardcoded table. RANKED by match count; tie = ambiguous; ROOT excluded ∵ it owns everything & answers nothing|V15
T9|x|`candidates(root, dir)` — promotable modules ranked by the spec rows NAMING them. whole-word ∵ substring made `plan` match `planning` & every module claim every row. `<name>.rs` + `<name>/` = 1 module in the 2018 layout `.:§C` forbids, ⊥ 2 candidates|V16
B12|2026-08-23|`split` v1 ranked modules by the SPEC ROWS naming them ∴ it answered "what does the prose talk about" — downstream of, & noisier than, "what did the author already SEPARATE". 2 consequences, both measured on `itok`: a module the code separates & the spec never mentions is INVISIBLE (`walk` = 4 rows, a `pub mod`), & on a THIN spec the proposal is empty — which is the repo most needing federation. the ranking also can ⊥ be SUMMED: 82 of 282 itok rows name ≥2 modules ∴ columns overlap|V17. structure FIRST: dirs, then `pub mod`, then naming families & `use crate::` cohesion; rows attach to proposed nodes AFTER. found by dogfooding a 2nd foreign repo, ⊥ by any test — the 1st (`rekall`) is flat & thin ∴ both orderings looked alike on it
T10|x|structure-first `split`: propose nodes from `code::mod_decls` + `code::crate_uses`, grade the evidence, attach row buckets per node. REPORT cohesion edges ⊥ auto-cluster — a graph clustering is where a proposer starts GUESSING, & `V16` says it proposes|V17,B12
