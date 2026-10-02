# SPEC

## §G GOAL

PROPOSE a federation: the modules the code already separated, the spec rows that name each, & the node an unmanaged row belongs to. proposes, ⊥ applies.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/split|PROPOSE a federation — modules the code separated, the rows naming each, a home for an unmanaged row
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
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it
sib|src/git|one git invocation shape — the repo a command acts on, & the env it refuses

## §C CONSTRAINTS

- PROPOSES, ⊥ writes. which node owns which law is a judgement over prose (V1).
- reads Rust through `src/code`; a 2nd reading of source is the 2-readings defect (`src/code:V1`).

## §I INTERFACES

- lib: `structure(&Path) -> Vec<Proposed>` — candidates read off the code, graded by `Evidence`; Rust modules & shell dirs/families (V7), each w/ the `scripts` it would own
- lib: `weight(&str, &Proposed) -> (usize, u64)` — rows & tokens naming a candidate: its name (Rust) ∪ rows citing its scripts (V7)
- lib: `ambiguous_scripts(&str, &[Proposed]) -> Vec<String>` — cited basenames naming ≥2 scripts, counted nowhere
- lib: `row_weight(&str, &str) -> (usize, u64)` — rows & tokens a spec spends naming one module
- lib: `rank(&[Proposed], &str) -> Vec<Ranked>` · `uniform_evidence(&[Proposed]) -> bool`
- lib: `propose(&str) -> Proposal` — `Move` · `Decompose` · `Keep`, for one unmanaged row

## §V INVARIANTS

V1: a SPLIT is PROPOSED, ⊥ applied. which module owns which rule is a judgement over prose, & a tool moving §V rows on its own rewrites law it cannot read. the weight is a HEURISTIC & says so: rows naming 2 modules count for both ∴ columns overlap & ⊥ sum to the chain. grounded ⊥ invented — MEASURED on `rekall`, whose flat spec names `check`·`trigger`·`corpus` on 17 lines each
V2: a federation is proposed from the CODE'S OWN STRUCTURE, ⊥ from prose. 3 grades of evidence, strongest 1st: (1) already a DIRECTORY — the author drew it · (2) `pub mod` — the author PUBLISHED it ∴ a boundary that already exists · (3) a naming FAMILY + `use crate::` cohesion — grouped implicitly. spec rows attach to proposed nodes AFTER & are EVIDENCE about a node, ⊥ the thing that proposes it. MEASURED on `itok`: 2 dirs · 7 `pub mod` · 11 `*cmd` sharing `cli`+`render` = ~12 nodes read off the crate, & 35 flat `mod` declarations that a row-count ranking scattered. 4th & weakest grade: DECLARED — a module & nothing more, which always fires ∴ a crate drawing every boundary that way (microlith: 11 `pub(crate) mod`) proposes 11 nodes, ⊥ none (B2)
V3: a measurement ! ⊥ read what its own GENERATOR wrote. `sync` writes `§N` into every node & `§N` names every SIBLING ∴ `row_weight` counts navigation as law — MEASURED on `src/tdd/SPEC.md`, 5 of its 6 rows naming a sibling are `§N` rows, & `src/SPEC.md`'s ONLY mention of `tdd` is its `§F` row. the loop CLOSES: split, `sync` writes more `§N`, weights rise, split proposes more ∴ it ⊥ converge. `§F` & `§N` are STRUCTURE, ⊥ law, & are excluded before counting
V4: only a QUALIFIED citation may be MOVED. the qualified forms — `src/X:`, `.:X:`, a backticked `repo/X` — are this repo's own convention & cannot collide w/ English; bare names can & do. MEASURED at root: `tdd` 13 qualified of 39 bare, `fed` 5 of 13, `cli` 5 of 9, `code` 0 of 20. sherd's node names ARE common English (`spec`·`plan`·`code`·`state`·`lens`·`check`) where `itok`'s are ⊥ (`bpe`·`ollama`·`gitref`) ∴ bare matching is contaminated HERE & looks clean THERE. a move ! REWRITE the citation as it moves (`src/tdd:B7` → `:B7`) ∴ the row ⊥ match at root on the 2nd run, which is what makes `--apply` a fixed point (`src/cli:V13`). bare rows are EVIDENCE for a human, NEVER input to a mover
V5: `split` SPREADS numbers across the repo & that is a FEATURE — under one condition. what the gate enforces is MAX chain, ⊥ sum ∴ 17 ceilings say WHERE the pressure is & 1 number cannot (the shape of `.:T91`: per-node density found `src/cli` at 0%, a project total would ⊥ have). the CONDITION: spread the NUMBERS, centralize the LEDGER. `.context-limits` is 1 file, machine-owned, gated — 17 rows, ⊥ 17 files. a measurement that moves INTO the `SPEC.md`s must then be GENERATED into them, & is then read back as law (V3 — `§N` is exactly this), & goes stale in 17 places rather than 1. a number inside a spec row is dated EVIDENCE: correct in a `§B`, which is a record, & wrong anywhere a gate reads — B3 states `spec 54 rows` in its prose & V4 proves half of that is the FILENAME, 1 day later. what must ⊥ spread is the VERDICT: one line, max chain over the boundary, ∵ nobody reads 17 numbers each near its own ceiling
V6: SOURCE cites anchors ∴ code is a 2nd & INDEPENDENT reading of where law belongs & where an impl grew too big. 2 signals from 1 grep: (a) an anchor cited from EXACTLY ONE node's source & owned by ANOTHER is a candidate to MOVE — MEASURED 25 of 83 distinct anchors here, of which ~10 are `§V`/`§T`, ∵ a `§B` is HISTORY & belongs where the bug HAPPENED, ⊥ where it is cited · (b) DISTINCT FOREIGN OWNERS per file is a SPLIT signal — `src/cli/mod.rs` cites 8, `src/plan/mod.rs` 6, nothing else exceeds 4. (b) AGREES w/ 2 instruments already run: `.:T91` found `src/cli` by COVERAGE (0%) & by LINE COUNT (445 lines, no test module), & this finds it by citation SPREAD ∴ 3 independent readings, 1 file — which is the only kind of confirmation a heuristic gets. still PROPOSES (V1): cli cites `src/fed:V11`·`V12`·`src/spec:V4` ∵ it RUNS those checks, & a dispatch layer legitimately NAMES law it enforces & ⊥ own. the instrument is only as good as the spelling it reads (`src/spec:B2`)
V7: a SHELL codebase is read w/ V2's ladder, mapped to what shell has: (1) DRAWN — a DIRECT child dir of the node holding a tracked `*.sh` @ ANY depth. direct ∵ an edge is parent + 1 (`src/fed:V2`) ∴ a deep monolith splits 1 level per run, & the deeper script dirs are SHOWN under their candidate so the path down is visible · (2) FAMILY — `<prefix>-*.sh`, ≥3 in 1 flat dir, prefix = the name before its 1st `-`. a family is ⊥ a dir yet ∴ promoting it moves files, a judgement (V1). WEIGHT = rows citing a script the candidate would own — by path (suffix match) or by a BASENAME that resolves to exactly 1 file. a basename naming ≥2 files counts NOWHERE & is NAMED. a bare dir word ⊥ a citation (V4: `trips`·`node`·`build` are English). MEASURED on a shell consumer: 353 scripts, 829 rows cite one & only 73 by path ∴ basename resolution is the reading, ⊥ an extra. a dir both readings find is proposed ONCE. calls between scripts (`source`, `bash $DIR/x.sh`) are DEPENDENCY, ⊥ ownership ∴ `src/wave`'s, ⊥ this node's (B6)

## §T TASKS

id|status|task|cites
T1|.|needs Rust source to say WHERE to cut — not this node's data|`.:V54`
T2|.|`split --apply` — move QUALIFIED rows down into the node they cite & REWRITE the citation. ⊥ to be built on the bare-word column|V3,V4,B4
T3|.|`split` reports code-anchor signals: MOVE candidates (solo-cited, foreign owner) & foreign-owner COUNT per file. needs the canonical spelling first (`src/spec:B2`)|`src/split:V6`

## §B BUGS

id|date|cause|fix
B1|2026-08-23|`split` v1 ranked modules by the SPEC ROWS naming them ∴ it answered "what does the prose talk about" — downstream of, & noisier than, "what did the author already SEPARATE". 2 consequences, both measured on `itok`: a module the code separates & the spec never mentions is INVISIBLE (`walk` = 4 rows, a `pub mod`), & on a THIN spec the proposal is empty — which is the repo most needing federation. the ranking also can ⊥ be SUMMED: 82 of 282 itok rows name ≥2 modules ∴ columns overlap|V2. structure FIRST: dirs, then `pub mod`, then naming families & `use crate::` cohesion; rows attach to proposed nodes AFTER. found by dogfooding a 2nd foreign repo, ⊥ by any test — the 1st (`rekall`) is flat & thin ∴ both orderings looked alike on it
B2|2026-08-23|`microlith` — 11 modules, 5,636 lines — proposed NOTHING. 2 defects, both found only by a 3rd foreign repo: (1) `pub(crate) mod` parsed as neither `pub ` nor bare `mod ` ∴ ⊥ declaration was seen at all · (2) the 3 grades had no case for a module that is neither a dir, nor `pub`, nor in a family — & microlith is ENTIRELY that shape. `rekall` & `itok` both hid it: rekall marks ∀ module `pub`, itok has dirs & a family ∴ each had SOME grade that fired|V2 gains `Declared`, the weakest grade. `pub(crate)`/`pub(super)` are visibility INSIDE the crate = what a bare `mod` says, ⊥ published. GENERALLY: a grade LADDER ! have a bottom rung that always fires, or a repo w/ ⊥ strong evidence reads as a repo w/ ⊥ structure
B3|2026-08-23|the `rows` column read 0 for EVERY node of a FEDERATED repo — the only kind where the question matters. `module_names` excluded a dir that already carries a `SPEC.md` ∵ `candidates` listed modules to PROMOTE, & `split` reused it for WEIGHT. MEASURED on our own tree: root names `tdd` on 40 rows, `spec` on 54, `lens` on 29, & the table said 0 for all 14. 2 questions were 1 fn: "which modules could BECOME nodes" (structure answers it, better) vs "how much of the parent spec is about a node that EXISTS"|V2. `row_weight(spec, name)` answers the 2nd; `candidates`/`Candidate`/`merge_layouts` deleted ∵ `structure` answers the 1st w/ evidence grades. layout detection MOVED, ⊥ lost — `<name>.rs` + `<name>/` is flagged where the node is proposed
B4|2026-08-23|`row_weight` counts rows the TOOL wrote & rows about a FILE. 2 causes, both measured: (1) `names_word` LOWERCASES ∴ `SPEC.md` splits to `spec` & the `spec` column reads 54 where a case-sensitive count reads 27 — half of its `4,109 tok` is the FILENAME, on rows about no node at all · (2) `§F` & `§N` are counted, & `sync` GENERATES `§N` naming every sibling ∴ 5 of the 6 sibling-naming rows in `src/tdd/SPEC.md` are navigation. at ROOT `§N` is 2 rows ∴ the defect is small exactly where I first read the table & large at every node beneath it. ⊥ findable by dogfooding a stranger: no foreign repo is federated deep enough to HAVE a sibling list, so a bug in what we generate for OURSELVES only shows at home|V3,V4. exclude `§F`/`§N` before counting, match case-SENSITIVELY, prefer the qualified form. DONE: `§F`/`§N` bodies skipped, match case-SENSITIVE. MEASURED after — root `spec` 54 rows/4,109 tok → 26/2,395, `code` 22 → 19, `fed` 13 → 12, `tdd` 40 → 39 ∴ the filename was ~half of ONE column & the nav a row or 2 of the rest. T2 unblocked
B5|2026-08-24|the EVIDENCE LADDER ranks nothing in 5 of the 6 repos it has met, & `split` ordered proposals ALPHABETICALLY ∴ the 1 signal actually present was thrown away. MEASURED across every tree dogfooded: `sherd` 15 directory · `ashlar` 14 pub · `metope` 18 pub · `microlith` 11 declared — all UNIFORM; `rekall` 2 dir + 17 pub, effectively uniform; only `itok` (2 dir · 6 pub · 1 family · 15 declared) discriminates. `B13` RECORDED this observation for the 1st 3 repos ("a signal that fires on every module carries no information") & changed only the tally line, ⊥ the ORDER. `metope` spans 0→58 rows across 18 modules & led w/ its 58-row node by luck of the letter b|V2,V1. `rank()` — heaviest first, grade breaks ties, then name ∴ stable. `uniform_evidence()` SAYS when the grade ranks nothing rather than letting the order read as a verdict. FOUND on a 5th foreign repo that shares our conventions (`dev/`, `hk.pkl`, a flake) & is WARM — `.context-limits` present, chain 5,734 of 8,200, 0 structural, 0 drifted — ∴ the 1st stranger to pass CLEAN, which is the negative control the messy ones could ⊥ provide. GENERALLY: an observation recorded in a `§B` fix column is ⊥ a change; `B13` saw this & the next 3 repos paid for it
B6|2026-10-02|`split` read only Rust `mod` declarations ∴ a shell repo got `no module declarations found -- nothing to propose` & exit 0 while its 1 node sat ~300k tok over a 2,000 ceiling. measured on a consumer w/ 353 `*.sh` in ~10 dirs & 1 root `SPEC.md`: ⊥ path from monolith to federation except writing every node by hand (#81)|V7. shell dirs & families join the ladder, weight by cited scripts. found the day #80 made that consumer read as 1 node — the 1st time `split` had a clean tree to read
