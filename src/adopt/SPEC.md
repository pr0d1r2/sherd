# SPEC

## §G GOAL

Brownfield on-ramp. A foreign single-file `SPEC.md` → a federation, rows MOVED ⊥ retyped.

MOTIVATING SHAPE: `init` REFUSES a file that exists & `split` proposes from CODE & never writes ∴ a repo that already HAS a spec — every repo but this one — has ⊥ a path in. the fleet is ~80 repos on one pin & each carries one monolith.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/adopt|foreign single-file spec → federation: row placement, citation rewrite, conservation
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
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
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it
sib|src/git|one git invocation shape — the repo a command acts on, & the env it refuses

## §V INVARIANTS

V1: adoption CONSERVES. ∀ row of the source spec lands EXACTLY ONCE — one node, ⊥ zero & ⊥ two — & the counts in/out are REPORTED, ⊥ assumed. a dropped `§B` row drops the memory of a defect & a COPIED `§V` makes 2 rules that drift, which is the defect federation exists to REMOVE
V2: PLACEMENT is a JUDGEMENT & stays one. the verb PROPOSES a row→node map & writes only from a map handed BACK, exactly as `split` refuses `--apply`. a row 2 nodes cite goes to their common ANCESTOR; a row nobody claims stays @ ROOT & is NAMED there, ⊥ left quietly
V3: a proposed HOME ! be a node the `§F` tables already DECLARE (`src/fed:V11`, `src/fed:V12`). adoption places ROWS onto structure that EXISTS — inventing an edge here defeats exhaustive & disjoint at once, & silently
V4: a MOVED id keeps its NUMBER & gains a NAMESPACE — bare `V9` → `` `NODE:V9` `` (`src/spec:V7`), the owner UPPERCASE here ∵ a lowercase one is what MAKES a citation & this line would then cite a node named `node`. ⊥ RENUMBERING, EVER: every citation elsewhere already names the old number. the FOREIGN slash form (`microlith/V14`) names another repo & is untouched
V5: the OUTPUT is CHECKED before it is offered & the measure is the DELTA, ⊥ the total: a migration ! ADD a violation, & one the tree already CARRIED is the tree's own — REPORTED, ⊥ repaired & ⊥ blamed on the move. a migration whose result the project's own checker rejects has shipped a 2nd dialect (`src/spec:V6` one verb over, & that same test caught a scaffold prompt being read as a citation) — but a brownfield spec generally does ⊥ pass `check` ALREADY, & gating on the total refuses every repo the verb exists for (B1)
V6: adoption is RERUNNABLE. a 2nd run over a migrated tree finds ⊥ to move & says so, exit 0. a foreign repo is EDITED between the proposal & the apply ∴ a verb unsafe to repeat is a verb nobody dares finish
V7: a RECEIVING node gets rows IN ID ORDER — ∀ moved row lands before the 1st resident row numbered higher, ⊥ appended. the receiver already HAS rows once a federation grows past its 1st adoption, & `microlith/V14` checks their order ∴ an append makes V5 refuse a migration over an order the verb itself wrote (B3)
V8: `0 rows read` from a source this reader cannot PARSE is ⊥ the same answer as `0 rows read` from one it parsed — the 1st is checked BEFORE any count is printed & exits 2 (USAGE: the file is in a form the verb does ⊥ accept), the 2nd keeps exit 0 meaning "I read this & there is nothing to move" (`src/spec:V9`, B4). ONE reading, ∀ invocation — propose & `--map` alike, ∵ a path that skips it is the bug back

## §T TASKS

T8|.|V2's COMMON ANCESTOR half — a row 2 nodes claim EQUALLY lands @ root today, ⊥ @ the node ABOVE both. `src/parse` & `src/render` tying should mean `src`, & sending it to `.` files a rule about rendering next to the repo's goal. the tie is already DETECTED (`best` returns none on it) ∴ what is missing is the walk up, ⊥ the signal|V2,`src/fed:T10`

## §B BUGS

id|date|cause|fix
B1|2026-08-25|`checked()` gated on the TOTAL violation count of each output ∴ the 1st real migration attempt — `ashlar`, 85 rows, the 4th foreign repo this project has learned from — was REFUSED, & the 2 violations it named (`` `B2` sorts before `B3` above it ``, & the same for `B1`) were in ashlar's monolith BEFORE the verb ran: its `§B` rows are filed newest-first, which `microlith/V14` forbids. the verb would ⊥ have migrated a single repo in the fleet, ∵ a spec that already passes `check` is a spec that has already been federated. found by RUNNING it on a real repo, ⊥ by any test — the fixture was written well-formed ∵ I wrote it|V5 restated on the DELTA: `introduced()` subtracts what the file already carried, COUNTED ⊥ set-deduped so a doubled message is still new, & `Report.carried` names the pre-existing ones so they are reported rather than absorbed. GENERALLY: a tool for brownfield trees ! be tested against a tree that is actually brown — a fixture the author wrote is a fixture that meets the author's standards
B2|2026-08-25|placement scored the row's RAW TEXT ∴ requalification fed the next proposal: a moved row's cites gain the destination's PATH (`` `GIT:V2` `` w/ the owner spelled UPPER here, ∵ the lower form is a live citation & this row would then cite `ashlar`'s tree), `significant()` splits on non-alphanumerics ∴ the dir NAME becomes a WORD of the row, & it matches that node's own lens. a rerun over the migrated `ashlar` proposed 20 MORE moves — 0 of them earned, every one justified by a path the previous run wrote. V6 says a 2nd run finds ⊥ to move & the 2nd run found 20. found by RUNNING it twice on a real repo; the fixture is 6 rows & its 2nd run happened to be clean|V2's matching now scores the SUBJECT — `subject()` drops backticked `owner:ID` spans before words are taken, ∵ a citation is a LINK ⊥ a subject. GENERALLY: a tool that REWRITES its own input ! be measured on the rewritten form — idempotence is a property of the 2nd run, & `.:V118` already names WRITE as one of its 3 layers
B3|2026-09-19|`add_rows()` APPENDED the batch after a section's resident rows ∴ adopting onto a node that already held `T88` put a moved `T3` below it, & V5's delta check refused the WHOLE migration: `` `T3` sorts before `T88` above it ``. the 1st adoption never shows it — every receiver is a fresh scaffold w/ ⊥ rows — & every fixture is a 1st adoption. found by RUNNING a 2nd adoption on a federation that had grown since its 1st, in a repo outside the fleet's usual set|V7: `in_id_order()` inserts each received row before the 1st resident row numbered higher; non-row lines keep their place. GENERALLY: a fixture of the 1st run cannot catch what only the Nth run meets — `received_rows_land_in_id_order_among_resident_rows` seeds resident rows
B4|2026-09-20|`adopt --check` over a `SPEC.md` whose `§T` is written as a BRACKETED markdown table (`\| T1 \| . \| first task \| -\|`) reported `0 rows read · 0 placed · 0 unplaced`, exit 0 — & in this verb's exit scheme 0 means "⊥ to move" ∴ a repo being WEIGHED for adoption read as ALREADY FEDERATED, the opposite of the truth, in the same words & the same code as the honest answer. `spec::rows()` anchors an id @ the START of a line (`microlith`'s grammar, correctly) ∴ the bracketed dialect is PROSE to it & the file reads EMPTY. MEASURED on a 2-row fixture: 2 id-shaped rows in, 0 read, exit 0 — & `sherd check` said `1 nodes examined · 0 violations` over the same file, so nothing in the toolchain dissented. this is `src/fed:B6`'s shape (something that finds nothing passes cleanly) @ the 1 verb whose whole job is reading a STRANGER's spec|V8 & `src/spec:V9`: a DETECTOR for the id-shaped row this grammar declines, checked before any count & exiting 2 w/ the line numbers & the form that works. `M` is excluded by construction — `row_id()` does ⊥ own it & `microlith::milestones` reads that table. GENERALLY: a reader that cannot parse its input ! say so, ∵ EMPTY & UNREADABLE are different answers & the 2nd is ⊥ a quieter version of the 1st
