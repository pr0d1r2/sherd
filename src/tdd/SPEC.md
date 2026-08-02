# SPEC

## §G GOAL

TDD as separate round-trips — red, judge, green, local gate, capped repair.

## §C CONSTRAINTS

- ∀ step declares its profile. ⊥ one call sees spec + tests + impl together (`.:V87`).
- step 3 = LOCAL, deterministic, ZERO tokens (`.:V18`).

## §V INVARIANTS

V1: step 2 & 4 ⊥ touch the test region. STRUCTURAL — `insert_impl` writes above `#[cfg(test)]` only ∴ ⊥ a matter of asking nicely
V2: RED test ! fail at an ASSERTION, ⊥ at compile. a test that ⊥ build is ⊥ a red test
V3: judge sees invariant + test, NEVER the impl. guards self-consistent wrongness — one model writing both test & impl to match its own misreading
V4: test already green → STOP & restore. ⊥ a red test, nothing to drive
V5: repair budget capped. exhaustion → report what was TRIED, ⊥ silent give-up
V6: test ! cite a declared `§V` id. a test for an invariant that ⊥ exist encodes an unstated rule
V7: `split_module` has ONE definition here. the code ceiling (`.:V50`) ! reuse it — 2 readings of one rule is the defect this project ends
V8: judge ! see the DATA MODEL. w/o field shapes it cannot tell a test asserting on the wrong field from one asserting on the right one (B2)
V9: judge answers on SUBJECT ∧ FALSIFIABILITY, ⊥ either alone. a test can be perfectly falsifiable & still test the wrong quantity (B2)
V10: repeated rejection = evidence about the WORK. loosening a guard to make it pass is silencing it (B2)
V11: gates GREEN ⊥ correctness. `cargo test` + `bbx check` both passed on semantically empty code (B2) — green for the wrong reason, inside the tool built to catch it
V12: prompts that must READ an invariant get the NOTATION contract. asking a model to satisfy `⊥ skip levels` w/o the symbol key is asking it to guess (B6)
V13: repair REPLACES exactly what the previous attempt added, tracked ⊥ guessed. name-prefix heuristics duplicate definitions (B7); full-region rewrite deletes unrelated code (B5)
V14: surface keeps DOC COMMENTS. bare field names cannot distinguish `not_owns` (prose) from `dir` (path) & that is the judge's whole question (B4)
V15: prompt = RULE depth. §G §C §I §V §T in, §B §R out. §T is the PLAN (the row names the work); §B/§R are archive. MEASURED: one §B row pushed step 1 1,210→1,520 tok & flipped a correct run to rejected (B9)
V16: judge objection FED BACK, ⊥ discarded. it is actionable signal; hand-tuning the prompt instead burned 6 configurations before I noticed (B10)
V17: model DETERMINISTIC @ temp 0 — 3/3 identical `gen` counts on one prompt ∴ outcome differences are prompt differences, & isolation works. ⊥ blame variance
V19: step 2 & repair see SIGNATURES ⊥ bodies. surface-by-EXAMPLE induces imitation — shown `edges()`'s scan loop the model copied it, twice, on independent tasks. shown only the interface it COMPOSES: `depth_violations(&[Edge])`, 5 lines ⊥ 30, `in_f` 6→3, 20% cheaper (`.:fed` B4)
V21: a step that takes >10s ! report BEFORE it starts & stream while it runs. silence is indistinguishable from a hang — `.:V48` applied to a live process, ⊥ only to a report (B19)
V20: constrain what the model SEES, ⊥ ask it for good behaviour. asking to reuse produced WORSE code (data-laundering `sanitize_first_cell`); removing the bodies fixed it w/ no instruction at all
V22: the impl judge sees invariant + impl, NEVER the test — the MIRROR of V3, ⊥ a second opinion. every stub in §B passed because test & impl AGREED; agreement is invisible to a reviewer holding both & obvious to two each holding one. giving either judge the other side restores the blind spot it exists to cover. MEASURED 2026-08-02 on gpt-oss:20b: 5/5 recorded stubs REJECTED & 5/5 real repo fns ACCEPTED — 10/10 separation, ~440 tok & ~4s per call
V23: step 2 is a FIELD, ⊥ a first draft. N candidates compete on the SAME evidence (gate + mechanical review). GREEN + a finding = the stub signature ∴ FATAL, ⊥ ranked — ranking always returns something ∴ least-bad stub wins by default. ALL RED = UNFINISHED, ⊥ bad → repair candidate 0, so N>1 can NEVER do worse than N=1 (B21). tie → candidate 0 (the temp-0 call): merit ! be DEMONSTRATED to displace determinism
V18: step 2 gets the CONTRACT — the calls the test makes that ⊥ exist yet, extracted deterministically (`.:V18`). the SIGNATURE constrains the design: `check_edge_depth(text)` took no path ∴ no walking ∴ no ignore-glob violation & no temp dir. fixing a NAME mismatch removed 3 unrelated defects (B12)

## §T TASKS

id|status|task|cites
T1|x|`split_module`, `insert_test`, `insert_impl` w/ structural test-region guard|V1,V7
T2|x|red → judge → green → gate → repair loop|V2,V3,V4,V5
T3|x|invariant-exists precondition|V6
T4|~|needs wiring, ⊥ a new function — `classify_failure` exists|V2
T5|.|`§T` row status flip on green (`.` → `x`)|V5
T6|x|judge gets `signatures()` data model|V8,B2
T7|.|needs a hook change, ⊥ a function|B3
T8|.|record per-request template overhead (~67 tok, measured) in entry-cost accounting|V2
T9|x|step 2 ! define exactly the fn the test calls — pass the expected signature|V18
T10|x|blind impl judge after green — `blind_prompt`, `is_yes`|V22,V11
T11|x|MEASURE V22 — `blind_lens_vs_the_recorded_stubs` 5/5 reject + `blind_lens_vs_working_code` 5/5 accept. the CONTROL is ⊥ optional: a lens answering NO to everything scores 5/5 on stubs alone|V22
T12|x|`Candidate`, `best`, `candidate_count` + N-candidate step 2|V23
T13|~|MEASURE V23 @ N=3: run 1 = 3/3 red, exposed B20. cost 3 gen (1,590+1,407+1,651 tok) + 3 full gate runs, 2m27s total. RERUN pending after the fix|V23

## §B BUGS

id|date|cause|fix
B1|2026-08-01|step 1 saw spec + tests but ⊥ the data model ∴ test author could ⊥ see `Edge`'s fields & reached for `owns` to compute depth. judge caught it (twice)|`signatures()` — public surface, ⊥ bodies. it is `§I`, ⊥ `§V`
B2|2026-08-01|judge LOOSENED from "does this prove the invariant" to "would a violator fail it" after 2 rejections read as over-strict. 2 rejections were the GUARD WORKING. under the weak bar it passed a test asserting on `not_owns` (prose) as if it were a path — impl & test agreed w/ each other & neither related to V2. gates GREEN, verdict MERGEABLE, code meaningless. self-consistent wrongness, the exact failure V3 exists to catch, caused by relaxing V3|judge ! require BOTH — right SUBJECT (field names vs data model) AND falsifiable. judge gets `signatures()` too. GENERALLY: a guard rejecting repeatedly is evidence about the WORK, ⊥ about the guard
B3|2026-08-01|`cargo test` ⊥ refresh `target/debug/<bin>` — it builds a separate test harness ∴ 2 runs used a stale binary & reported identical token counts. read as "prompt unchanged" ⊥ "binary unchanged"|`cargo build` before any run that exercises the bin. gate ! rebuild first
B4|2026-08-01|`signatures()` stripped `///` docs ∴ judge saw `pub not_owns: String` bare & could ⊥ tell prose from path. it approved a test asserting `not_owns` as a path, twice. I built a surface extractor that drops exactly the semantics it exists to convey|keep doc comments. own test then caught a 2nd bug — field-level docs inside a type body routed to `pending` & never drained
B5|2026-08-01|repair prompt said "reply w/ the corrected FULL implementation region" ∴ model deleted the module `//!` header, rewrote `edges()` to SILENTLY SKIP multi-level rows (violating `.:V48`, which it never saw), & swapped hyphens for U+2011|repair returns ONLY the function it added. steps 2/4 ⊥ see root §V ∴ ⊥ license to touch anything they cannot check
B6|2026-08-01|⊥ prompt carried `FORMAT.md`. model asked to satisfy `V2: ... ⊥ skip levels` w/ no symbol key — `⊥` could read as math bottom or noise. `cavespec` vendors FORMAT.md verbatim "so a tool that cites the format can read it"; bbx cites it everywhere & had ⊥ copy|vendor `FORMAT.md`; compile a NOTATION slice (symbols + section meanings, ~200 tok ⊥ the whole 750) into every prompt that reads an invariant. `.:V82` contract-⊥-implementation applied to our own prompts
B7|2026-08-01|repair splice keyed on `\npub fn check_` ∴ when the model named it `find_depth_violations` the fix APPENDED instead of replacing → `E0428` redefined. a heuristic where a record would do|track the inserted block; ⊥ found → refuse & say so, ⊥ guess
B8|2026-08-01|first PLANTED violation proving the pre-commit gate passed straight through. the plant cited `` `V998` `` in BACKTICKS ∴ FORMAT reserves backticks for verbatim & cavespec correctly ⊥ read it as a citation. a bad plant, ⊥ a bad gate — but indistinguishable from one until re-planted|plant a BARE `V777`; commit then REFUSED, HEAD ⊥ moved. GENERALLY: a guard test passes vacuously if the plant is ⊥ actually a violation — green for the wrong reason, one level up, in the act of proving a guard
B9|2026-08-01|adding ONE §B row to `src/fed/SPEC.md` pushed step 1 from 1,210 to 1,520 tok & turned a run producing CORRECT code into one the judge rejected. bug history in a node spec pollutes the prompt that AUTHORS tests. `.:V43`/`.:V45` said rationale by reference for COST; the cost is also QUALITY|`rule_depth()` — §G §C §I §V §T in, §B §R out. dropping §T too was a 2nd error: §T is the plan, ⊥ the archive
B10|2026-08-01|judge rejections were treated as a HARD STOP ∴ I hand-tuned the prompt across 6 configurations chasing 1 success, discarding an actionable objection each time. the loop had feedback available & ⊥ used it|feed the objection back, capped at 3. GENERALLY: a reviewer's REASON is signal; throwing it away & guessing is the expensive path
B11|2026-08-01|attributed run-to-run differences to model variance. MEASURED FALSE: 3/3 trials identical `gen 3487` ∴ deterministic @ temp 0. every difference was a prompt change I made. `.:V59` — measurement ⊥ belief, on my own methodology|isolate ONE variable per run. determinism makes that possible
B12|2026-08-01|step 2 never told what to DEFINE ∴ test called `check_edge_depths(root,&e)` & step 2 invented another name → `E0425`, 3 repairs could ⊥ recover. also the root cause of `.:fed` B1/B2/B3: an unconstrained signature let it choose a `&Path` design & drag in walking, temp dirs & `*.md` globbing|`expected_calls()` — deterministic parse of calls absent from the surface, passed as a contract. NEXT run: judge YES first try, 3 round-trips, all 3 prior defects ABSENT. own test caught `fn x(` being read as a call
B13|2026-08-01|`expected_calls` skipped any ident preceded by `!` — written to exclude macros, but a macro is `name!(` (already excluded by the `(` check) while `!name(` is NEGATION. `assert!(!classify(r))` ∴ contract silently EMPTY on a real run & step 2 got no name to define. run passed anyway, by luck|drop the leading-`!` skip. own unit test used `assert_eq!(a,b)` which the `(` check excludes regardless ∴ the test could ⊥ see the bug — a guard that passes for the wrong reason
B14|2026-08-01|README mermaid rendered BLANK + GitLab perf warning, twice. blamed `flowchart` vs `graph`, HTML labels, unicode — all wrong. REAL cause: `G=$(bbx graph)` strips trailing newlines & I concatenated onto the fence ∴ ```` ``` ```` landed on the SAME LINE as content, fence never closed, mermaid got the WHOLE REST OF THE README as diagram source. 610-char block, 6KB actual input ∴ the perf warning was TRUE|write the fence w/ an explicit `\n` before it; assert the generator ends w/ a newline & that fences sit at column 0. GENERALLY: I theorised 3 causes from the SOURCE & never looked at the RENDERED bytes. the warning was accurate & I dismissed it as generic
B15|2026-08-01|pushed code that does ⊥ COMPILE. python slice `t[:start]+t[end:]` where start>end (dead `fn trim` sits AFTER `cell` in the file) ∴ it DUPLICATED a region instead of removing one — `cell`, `ident`, `tree` defined twice, `E0428`|assert on the RESULT, ⊥ the operation: rebuilt from the known-good blob & asserted each fn appears exactly once
B16|2026-08-01|pre-commit gate ran `bbx check` ONLY ∴ passed on code that does ⊥ compile. the gate reads SPECS, & I let it stand for correctness. `cargo test` was run separately & I read a grep that printed nothing as "fine"|gate now `cargo build` + `cargo test` + `bbx check`. GENERALLY: a gate proves what it RUNS. green from a check that never invoked the compiler is `.:V11` — green for the wrong reason — in the guard itself
B17|2026-08-01|hook hardened to run `cargo`, but `cargo` is ⊥ on PATH outside the dev shell ∴ the COMMIT aborted — & the `&&`-chained `git push` ran anyway, pushing the BROKEN HEAD. read "pushed" as success|hook honors `BBX_CARGO`; ⊥ resolvable → FAIL, never pass unverified. GENERALLY: `commit && push` hides a failed commit behind a successful push — the push had nothing to do w/ the commit that failed
B18|2026-08-01|`expected_calls` leaked `new(env!(...))` into the contract from `Path::new(..)` — I skipped idents preceded by `.` but ⊥ by `:` ∴ the tail of a `Type::method` path read as a bare call. surfaced in a USER's run, ⊥ my tests|skip `:` too. own tests used only free fns & method calls ∴ never exercised an associated fn — 3rd guard this session that passed w/o touching its own branch
B19|2026-08-01|`bbx tdd` printed the node line then went SILENT for 40-90s per step — `eprintln!` fires only AFTER the reply lands. user could ⊥ tell working from hung|stream (`"stream": true`, NDJSON) + a `->` line before each call & a dot per ~25 chunks. `-v` dumps full prompts & replies, which is what every prompt-debugging turn this session needed & guessed at instead
B20|2026-08-01|B17 RECURRED 3h after being recorded: `git commit` (separate stmt) then `git push && echo pushed`. commit ABORTED on a failing doctest, push pushed the OLD head, output said "pushed". recorded the lesson & ⊥ enforced it ∴ repeated it verbatim|chain `commit && push`. GENERALLY: a §B row is a record, ⊥ a runner (`.:V74`) — the 2nd time this exact shape has been written down & repeated anyway
B21|2026-08-02|first cut of V23 disqualified a candidate on a RED GATE. MEASURED @ N=3 on `src/ollama` T3: 3/3 red, 3/3 disqualified, whole step reverted — ∴ asking for MORE candidates REMOVED repair & made the loop strictly WORSE than N=1. a red gate is ⊥ a verdict, it is an UNFINISHED attempt; repair is the step that answers it. I wrote the disqualify rule to avoid least-bad-wins & aimed it at the wrong signal|`select` → `Pick::Unfinished(0)` when all red: repair the same thing N=1 would have. GREEN + finding stays fatal — repair polishes a stub, ⊥ fixes one. V23
