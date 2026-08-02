# SPEC

## §G GOAL

What to attempt next, and why it might ⊥ survive contact.

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
V8: steps ordered by BELIEVABILITY — a node's measured keep-rate — then depth. `src/fed` failed 3x & kept supplying step 1 because depth was the only signal
V9: Laplace-smoothed `(kept+1)/(tried+2)` ∴ an untried node scores 0.50 & outranks 3 failures (0.20) w/o pretending to be known-good
V10: `kept` counts what survived REVIEW, ⊥ what passed the gate. the gate has gone green on 3 stubs ∴ counting commits measures the wrong thing
V11: a principle a machine can CHECK belongs in a gate, ⊥ a prompt. MEASURED strengths: structure (`insert_impl` cannot touch the test — never violated) > gate (`-D warnings` — evaded twice w/ `_`) > prompt ("reuse existing functions" — ignored, produced worse code)
V7: output names commands that EXIST. `plan` IS replan — it is stateless & re-derives every run ∴ saying "REPLAN" invents a second name for one operation, which is the two-readings defect (B3)

## §T TASKS

id|status|task|cites
T1|x|`open_tasks`, `classify`, `plan` w/ horizon + confidence|V1,V2,V5
T2|x|unmanaged rows reported w/ reason|V3
T3|.|`needs` column in `§T` so ordering is declared ⊥ guessed|V4
T4|.|machine-actionable marker in `§T` so classify ⊥ heuristic|V5
T5|.|needs `crate::ollama` — cross-node, name it before driving|V1
T6|.|`bbx plan \| head` panics on broken pipe. handle SIGPIPE|V6
T7|.|needs Rust source to say WHERE to cut — not this node's data|`.:V54`

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`classify` matched `derive \`§n\`` while the row read `` `§N` derive from parent `§F` `` ∴ a multi-file task planned as an actionable single-node fn. caught by READING the first plan, ⊥ by a test|word-order independent match. but widening a substring list is a PATCH — T4's declared marker is the fix, & prose classification stays wrong-by-default until then
B2|2026-08-01|first real `apply` refused: "test passes already". the `§T` row read "`depth_violations` landed ... cycle detect still open" ∴ the model tested `depth_violations`, which EXISTS, & it passed. a row describing what is DONE misleads a machine reading it as work|`§T` states REMAINING work only (`src/fed:V9`). history → commit trail. the loop was right & the input was wrong — & the refusal is `.:tdd` V4 working
B3|2026-08-01|`apply` said "REPLAN before the next step" & `plan` printed "REPLAN after each apply" — a verb that reads as a command name & is ⊥ one. user tried `bbx replan`, got usage. `plan` IS replan: stateless, re-derives every run|say "run `bbx plan` again". ⊥ an alias — a 2nd name for one operation is the defect, ⊥ the fix
B4|2026-08-01|`classify` was a BLACKLIST — actionable = ⊥ matching known-bad shapes ∴ "replace the hand-rolled walk" & "promote an invariant to the ancestor" both read actionable. the loop only APPENDS (`insert_impl`) & edits ⊥ spec files ∴ both are structurally undrivable & each would burn a cycle & commit something wrong|whitelist the ADD shape; `Replaces` & `NotAFunction` kinds. found by reading what `plan` would hand a run, ⊥ by running it
B5|2026-08-01|the new `Replaces` list matched SUBSTRINGS ∴ "report" contains "port" & every `report ...` row — the most common actionable shape — classified as a replacement|match whole WORDS. caught by my own test in the same commit that introduced it, which is the only reason it cost nothing
B6|2026-08-01|`cited_invariant` parsed only BARE ids ∴ every row moved down — all of which cite the namespaced `` `.:V73` `` form `.:V11` requires — was undrivable, & `drive` looked for the invariant in the NODE's spec when `.:` means ROOT. the moves made rows citation-correct & apply-incompatible in one step|parse `owner:id`, resolve `.` to root; `drive_from` reads the invariant from the owning spec. found by the FIRST cycle of a supervised run, which is what a supervised run is for
B7|2026-08-01|marked a row "BLOCKED" in its text & `plan` handed it back as step 1 — prose is ⊥ a status. also `git revert -q` is ⊥ a valid flag ∴ a revert I reported as done never ran & I committed on top of code I had declared gone|`blocked` joins the ⊥-actionable words. GENERALLY: report a revert only after checking the code is GONE, ⊥ after the command returns
B8|2026-08-01|AUDIT of all 21 actionable rows: only 5 are computable by the node that holds them. 12 fail — needing another node's data, a file that ⊥ exist, or ⊥ being a function at all. `classify` passed every one because they parse as "add one fn". also missed "wiring" (list had "wire") ∴ exact-word matching|stem matching; 12 rows reworded to name their blocker. the actionable COUNT was measuring shape, ⊥ drivability, & I reported 24 as ready
