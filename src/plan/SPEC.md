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
sib|src/split|PROPOSE a federation — modules the code separated, the rows naming each, a home for an unmanaged row
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/adopt|foreign single-file spec → federation: row placement, citation rewrite, conservation
sib|src/lens|pack assembly, depth `rule`\|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/wave|the SCHEDULE a parallel build follows — code DAG, ready set per round, depth & width
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

- horizon 3. beyond that is fiction — applying a task edits the SPEC that plans the next one.
- classify conservatively. an unactionable row wrongly attempted burns a run & edits source.

## §V INVARIANTS

V1: ∀ step carries its CONFIDENCE & what INVALIDATES it. a plan that ⊥ say how it fails is a promise
V2: confidence DEGRADES w/ distance — next · likely · tentative
V3: unmanaged rows LISTED, ⊥ hidden. 58 of 72 open rows are ⊥ machine-actionable & silence would read as coverage (`.:V48`)
V4: ordering signal is node DEPTH only. `§T`.cites points at `§V`, ⊥ at another `§T` ∴ stated as weak, ⊥ dressed up
V5: root row ⊥ actionable — no `mod.rs` to add to. EXCEPT a root row citing scripts 1 node owns (V26)
V6: plan ⊥ mutate source. it reads & reports; `apply` is the only writer
V12: `apply` runs `review` on what it just committed & surfaces any disagreement w/ the gate. a check nobody invokes is a check that ⊥ run — `review` existed for hours & caught nothing because it was typed by hand
V8: steps ordered by BELIEVABILITY — a node's measured keep-rate — then depth. `src/fed` failed 3x & kept supplying step 1 because depth was the only signal
V9: Laplace-smoothed `(kept+1)/(tried+2)` ∴ an untried node scores 0.50 & outranks 3 failures (0.20) w/o pretending to be known-good
V10: `kept` counts what survived REVIEW, ⊥ what passed the gate. the gate has gone green on 3 stubs ∴ counting commits measures the wrong thing
V11: a principle a machine can CHECK belongs in a gate, ⊥ a prompt. MEASURED strengths: structure (`insert_impl` cannot touch the test — never violated) > gate (`-D warnings` — evaded twice w/ `_`) > prompt ("reuse existing functions" — ignored, produced worse code)
V13: POSITION words ("around", "wrap", "inside") name where new code goes RELATIVE to existing code ∴ they mark a row that ! EDIT a call site, which the loop cannot do. a row can read as "add one function" & still be undrivable (B9). n=1 in this corpus — the rule is derived from ONE measured failure, ⊥ a survey
V14: a fix ! be tested on the EXAMPLE its own `§B` row names. B8 recorded "missed `wiring` (list had `wire`)" & shipped a stem match that still ⊥ match `wiring` (B11) ∴ the row read as closed for 3 weeks. the bug row's example is a test case already written down — ⊥ using it is discarding the one input known to reproduce
V15: a VOCABULARY is DERIVED, ⊥ listed. `VOCAB` knows 9 of 17 nodes & silently misses ∀ node added since it was written — the drift `sherd check` exists to catch, in the checker's own source. route reads the tree: dir name + `§G` words, ≥4 chars (shorter ones are caveman prose's articles & match everything). MEASURED while building: `sections()` returns the WHOLE heading `## §G GOAL` ∴ a `starts_with("§G")` matched nothing & ∀ node quietly reduced to its dir name — & the test asserting "words ⊥ empty" PASSED on dir names alone, which is why it now asserts a known `§G` word
V7: output names commands that EXIST. `plan` IS replan — it is stateless & re-derives every run ∴ saying "REPLAN" invents a second name for one operation, which is the two-readings defect (B3)
V22: a row is unmanageable for reasons OUTSIDE it. `classify` reads TEXT & answers "could the loop drive this"; a FREEZE, a rung, an embargo answer "may it" & live in the ROOT spec ∴ 2 different questions & the 2nd is invisible to any amount of reading the row (B16). derive the policy from the spec that states it, resolve it against the TREE, & REPORT what it excluded

V24: `plan --milestone M<n>` keeps a row ⟺ its OWN node's milestone table claims it — ids are node-scoped ∴ ⊥ a root table naming other nodes' rows. the partition is microlith's (`spec::milestones` → `microlith::milestones`), ⊥ a 2nd reading of the grammar; a suffixed id rides its base (`T7a` ∈ whatever claims 7, `microlith/V14`). open rows in nodes declaring NO milestones are COUNTED & PRINTED, ⊥ dropped (V3: silence reads as coverage); a milestone NO node declares = usage error (exit 2), ⊥ an empty horizon that reads as all-done
V25: `plan --format json` is PLUMBING, the text form PORCELAIN. 1 object, fixed keys, every step AND every unmanaged row w/ node·id·reason (V3 holds for a machine too); `invalidated_by` a LIST (V1); the root spelled `.` as in a cite. text may change, json only GROWS — pinned whole by a test ∵ a caller parsing a layout breaks silently on a cosmetic edit. an unknown `--format` = usage (exit 2), ⊥ a fall back to text
V26: a SHELL row is planned at its NODE, ⊥ by `mod.rs`. a node w/o `mod.rs` whose row cites a script (`src/split:V7`, via `split::cited_scripts`) or that OWNS scripts (`src/fed:V15`) is its home; the row's FOOTPRINT = the scripts it cites, & a row citing none is planned w/ footprint UNKNOWN, said w/ the step. a ROOT row citing scripts is planned at the 1 non-root node owning ALL of them; scripts in 2+ nodes → unmanaged, w/ that reason; in none → V5 as before. its INVALIDATORS = a change to a script it touches or a row it cites (+ the ordering ones of `Likely`/`Tentative`; `Next`'s are the tdd loop's & ⊥ apply). json: a shell step adds `touches` (null = unknown); a Rust step's object is unchanged. `apply` drives Rust steps only & skips a shell one. a tree w/o shell nodes plans BYTE-IDENTICALLY (#105)

## §T TASKS

id|status|task|cites
T3|.|`needs` column in `§T` so ordering is declared ⊥ guessed|V4
T4|.|machine-actionable marker in `§T` so classify ⊥ heuristic|V5
T5|.|needs `crate::ollama` — cross-node, name it before driving|V1
T6|.|`sherd plan \| head` panics on broken pipe. handle SIGPIPE|V6
T16|.|2 test files over the 2,000 ceiling after `.:T108` gave them their own tree: `tests/git.rs` 3,973 — the repository-backed `apply`·`triage`·believability tests — & `tests/plan.rs` 3,038. split each by subject|`.:V50`,`.:V124`

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
B16|2026-08-24|`plan` OFFERED work the root spec FORBIDS: `src/ollama:T3` ranked as step 3 while `.:V117` freezes the model half until rung `0.7`. 12 rows across `src/ollama`·`src/tdd`·`src/assay` were candidates, & `.:src/tdd:T13` — a MEASUREMENT of the frozen half — sat among them. `classify` reads the ROW ∴ it can never reach this: the freeze is root POLICY, ⊥ a property of the text, & no wording of a row makes it visible. FOUND by reading `plan`'s own output before taking its step 1, ⊥ by any test|V15. `frozen_nodes(root)` DERIVES the list from the row declaring it & resolves each against the tree ∴ a node added to or removed from the freeze needs no code edit — V15's lesson applied to a 2nd list. `Kind::Frozen` REPORTS them (V3: silence would read as coverage), & 89 open rows now show 81 unmanaged instead of 78
