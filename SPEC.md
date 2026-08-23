# SPEC

## §G GOAL

`sherd` (cmd `sherd`) — two-axis federation of `SPEC.md` + Rust source over a dir DAG, so a 128k local model works at one altitude & pulls deeper only when measured cost says it must.

MOTIVATING NUMBER: `itok` = 135,096 tok (28,462 spec + 106,634 code) vs 102,529 working on the target box. An 11,291-line CLI ⊥ fit its own best-case hardware.

TARGET: most modules BUILDABLE on the 20B (V106) — ⊥ merely readable by it. context was the binding constraint & after T41 is ⊥: the fattest chain is 9,886 of 102,529 working = 9.6% (R41). competence is what binds now: `src/tdd:T13` has ONE merit win & it did ⊥ survive review ∴ the loop's MERGEABLE & the gate's disagree, & the loop is the optimistic one. rung `0.5` (V114) is where they ! agree, measured.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|code nodes — tokens, spec, fed, lens facades & logic|inference harness, endpoint config|-
dev|repo-maintaining tooling, `publish = false` — README generation|anything a consumer installs|-

## §N NAV

rel|path|lens
up|-|-
self|.|-

## §C CONSTRAINTS

- lang: Rust **edition 2024**. stable. MSRV **1.95** = the FLEET PIN (`nixpkgs-lock` → nixos-26.05). ⊥ a number copied from a sibling: `itok`/`microlith` declare 1.96 & MEASURED compile clean on 1.95 ∴ their floor is a mirror of an old pin, ⊥ a minimum.
- nixpkgs rev FOLLOWED from `nixpkgs-lock`, ⊥ spelled here. one rev, ~80 repos.
- target model: `gpt-oss:20b`, 131,072 ctx, local. ⊥ cloud fallback.
- inference: local HTTP (Ollama) only. ⊥ network otherwise.
- deterministic core: parse/DAG/budget/ceiling = pure Rust, ⊥ model. model ? prose gen & drift judgement only. DEFAULT features are EMPTY ∴ the installed binary carries ⊥ HTTP client, ⊥ TLS, ⊥ network code; `ollama` is 1 flag away & named when a verb needs it.
- separator = **directory**. dir tree ! source of truth. ⊥ manifest, ⊥ name-encoded grouping (`core-parse` ⊥ imply parent).
- federation edge = parent dir → child dir, depth **+1 exactly**. ⊥ skip.
- graph ! DAG. cycle ⊥. re-parent (2+ parents) OK.
- intra-file spec ops = `microlith` lib dep (crates.io 0.6, zero-dep, pure fn over `&str`). ⊥ reimpl parse/fmt/check/anchors.
- token counting = `itok` lib dep (crates.io 0.3). ⊥ own tokenizer, ⊥ own bytes/4.
- fs walk + ignore globs = `itok::walk`/`itok::glob`. ⊥ reimpl.
- SPEC syntax = FORMAT **4.1.0**, sections `G C I R V T B` fixed & ordered + `§F`/`§N`. `§F`/`§N` ! land in FORMAT/microlith upstream, ⊥ invented locally (V47).
- layout: **one crate**. module = **dir + `mod.rs`** (the `default.nix` shape — dir is the unit, entry is conventional). ⊥ 2018 `foo.rs`+`foo/`: that puts the facade OUTSIDE the dir it fronts ∴ module entry & its `SPEC.md` land in different federation nodes.
- node = dir = Rust module. all three aligned or a flat `.rs` owns spec it cannot hold.
- repo partitions **set** \| **setting** \| **human** (`set-and-setting` vocabulary). default pack = set.
- caveman encoding ∀ generated spec text. MEASURED 22% saving ⊥ 75% (R23) — it disciplines saying LESS, ⊥ encodes denser.
- gate runner = `hk` (from `nix-hk`; nixos-26.05 ships none — landed on master after branch-off). ops DECLARED in `hk.pkl`, ⊥ a shell body in `.githooks`. schema VENDORED `pkl/Config.pkl` ∴ the gate runs w/ ⊥ network.
- SETTING as contract (V82), one line each: `rustfmt` 80 col + edition 2024 · `clippy -D warnings` · `cargo test` · `sherd slice/check/budget` · coverage · lint ratchet · hygiene & linters, named in `hk.pkl`.
- CODE limits as contract (V82), `clippy.toml` @ root + `[lints.clippy]` denies: fn ≤15 lines · cognitive ≤7 · nesting ≤4 · args ≤4 · fn bools ≤1 · struct bools ≤3 · trait bounds ≤3 · type complexity ≤150 · `unwrap`/`expect`/`panic`/`indexing_slicing`/`todo`/`unimplemented`/`dbg_macro`/`arithmetic_side_effects` DENY · `unsafe_code` FORBID. thresholds are Sandi Metz adapted; reasoning RESTATED here, ⊥ cited to a sibling — an id that resolves in no namespace this repo can reach reads authoritative & is ⊥ checkable.
- ⊥ global index file. discovery by walk.

## §I INTERFACES

- cmd: `sherd init [dir] [--stdout]` → scaffold `SPEC.md` @ dir, `§F` rows from child dirs. REFUSES an existing file, exit 1, ⊥ `--force`
- cmd: `sherd lens <dir> [--depth rule|why|all]` → context pack. default `rule`
- cmd: `sherd lens <dir> --json` → `{chain:[],body:{},children:[],tokens:{},examined:{}}` (0.3)
- cmd: `sherd route "<query>"` → dir + the words that matched. RANKED: the node matching MORE of the query wins, a tie is ambiguous, & ROOT is never an answer. 0 hit / 2 miss / 3 ambiguous
- cmd: `sherd check [dir]` → drift spec↔code + file ceilings. 0 clean / 1 violation / 2 usage
- cmd: `sherd split [dir]` → PROPOSE a federation from the CODE'S structure: dirs · `pub mod` · naming families w/ their shared hub. spec rows attach as EVIDENCE about a node, ⊥ as the reason for it. writes ⊥ ever; `--apply` refuses, ∵ which module owns which rule is a JUDGEMENT
- cmd: `sherd sync [dir] [--check]` → regen `§N` from the `§F` above it. exit 1 IF IT WROTE, ∵ a generated section that had to change means the committed tree was STALE. `--check` reports & writes ⊥
- cmd: `sherd graph [--dot|--json|--mermaid]` → federation DAG. `--mermaid` = the generated architecture diagram
- cmd: `sherd lens <dir> [--facet set|setting|human|all]` → default `set`
- cmd: `sherd budget [dir]` → node/chain/lens/file token table. exit 1 over
- cmd: `sherd validate` → structural + edges + ceilings + slice drift, & REPORTS what it examined. exit 1 fail
- cmd: `sherd fed [dir]` → the federation edges a node DECLARES, ⊥ the ones it has
- cmd: `sherd review [rev]` → mechanical checks on what a commit ADDED (default `HEAD`). ADVISORY: a finding ⊥ fail the cmd, ∵ intent is the reader's call
- cmd: `sherd slice [--check|--list]` → regen distilled slices from source. `--check` exit 1 on DRIFT, & a slice is never hand-edited
- cmd: `sherd outcome <node> <kept|reverted>` → record whether a node's work survived review. feeds believability (`src/land`)
- cmd: `sherd plan [--triage]` → next 3 steps + what would INVALIDATE each. `--triage` = unmanaged rows w/ a proposed home
- cmd: `sherd apply [--land]` → execute step 1 ONLY, commit to a run branch, STOP. ⊥ main (`.:B12`)
- cmd: `sherd land [--push]` → fast-forward main to the run branch IF it earned it: gate green + believability
- cmd: `sherd ask <dir> <q>` → ask the endpoint from a node's lens pack. the pack is the whole prompt
- cmd: `sherd tdd <dir> <Vn> <task>` → red → judge → green → gate → repair. the LOOP the §G target is about
- cmd: `sherd oneshot <dir> <Vn> <task>` → the MONOLITH arm: 1 call, whole repo. exists to be COMPARED against `tdd` (R29/R30), ⊥ recommended
- file: `SPEC.md` ∀ dir any depth. `§G §C §I §R §V §T §B` + `§F` + `§N`
- file: `§F FEDERATION` pipe table `dir|owns|⊥owns|tokens` — child dir depth +1. `⊥owns` = what it does NOT own + where that lives
- file: `§N NAV` pipe table `rel|path|lens`, `rel` ∈ `up`|`self`|`sib`. generated, ⊥ hand-edit
- file: `SPEC.why.md` ∀ dir w/ rationale — `<id>|<rationale>`, addressed by `§V`/`§B` id
- file: `.context-limits` — per-path ceilings, `itok` format, reused ⊥ reinvented
- file: `.spec-records` — closed-option baseline, `microlith --records`
- file: `.claude/commands/sit.md` — `/sit`, ONE oversight cycle. loop-safe, halts w/ a recorded reason
- file: `.claude/commands/titrate.md` — `/titrate`, granularity descent. attempt → enrich pack | split task. floor = VERIFIABILITY ⊥ size
- file: `.sherd-frontier` — `shape rung= kind= pack= sig= tests= tried= kept=`. TRACKED, ⊥ `.sherd-state`: learning that dies at the clone boundary ⊥ learning
- file: `AGENTS.md` — supervisor class. auto-loaded by Codex & Claude, ⊥ reachable by a worker prompt (V97)
- env: `SHERD_MODEL` (`gpt-oss:20b`), `SHERD_ENDPOINT` (`http://localhost:11434`)
- lib: `microlith::check_spec(&text,&records)`, `microlith::fmt`, `::anchors`
- lib: `itok::estimate`, `itok::walk`, `itok::glob`
- violation: `<file>:<line>: sherd/<Vn>: <msg>` + `why` + `mechanical`|`judgment`. `--format json` carries `kind` as data

## §R RESEARCH

id|topic|finding|src
R1|crates.io name|`sherd` taken (BBCode parser, lib-only, `bin_names: []`). BIN name `sherd` free on brew, debian, PATH ∴ package≠bin|crates.io API, formulae.brew.sh, sources.debian.org
R2|cargo workspace|`members=["crates/*"]` ERRORS on a hub dir w/o `Cargo.toml`. per-depth globs + cross-nested path deps build clean|cargo 1.96.1, tested
R3|rust module form|`mod.rs` = the `default.nix` shape. private sibling → `error[E0603]` ∴ COMPILER enforces the facade, ⊥ grep|cargo 1.96.1, tested
R4|itok size|`SPEC.md` 28,462 tok + code 106,634 = 135,096 ∴ 103% of a 128k window. an 11,291-line CLI ⊥ fit|itok 0.2.0 `--bpe`
R5|spec fatness|itok 656B/rule × 103 rules. rationale = 80% of §V (microlith 73%) ∴ per-row FATNESS is the multiplier, ⊥ rule count|measured over both §V
R6|federation yield|dir federation moves 41% of itok §V bytes; intermediate hubs absorb 9.5%; sherd own §V 64% stays root|classified 103 + 55 rules
R7|inline tests|42% of itok+microlith `src/` is `#[cfg(test)]`; `tracecmd.rs` 65% ∴ whole-file ceiling ranks a SMALL module worst|measured per file
R8|tokenizer drift|bytes/4 measured 48% LOW on caveman-encoded `SPEC.md` (5,761 real vs 3,890 est) ∴ ⊥ ever a gate|itok `--bpe` vs bytes/4
R9|gpt-oss:20b arch|24 layers, 8 KV heads, k/v len 64, sliding_window 128, ctx 131,072, 32 experts/4 used, MXFP4, 20.9B|ollama `/api/show` @ 192.168.0.181
R10|target box|24GB M5 Pro: ctx 131,072 ALLOCATED, 11.98G of 24G resident, 100% GPU, ⊥ CPU spill, ollama 0.32.3|ollama `/api/ps`, measured
R11|KV cost|KV/tok = 2 × layers × kv_heads × head_dim × bytes. sliding-window halves it (12 of 24 layers full) ∴ 24GB needs NO KV quant|derived from R9 + R10
R12|32k models|any 32k-ctx model leaves 4,225 working after 28,543 entry cost ∴ unusable for SDD at EVERY hw tier, incl M64|derived from R9/R11
R13|microlith §R|FORMAT 4.1.0 §R = RESEARCH (`id|topic|finding|src`), ⊥ records. closed options live in `.spec-records`|microlith 0.4.0 `check.rs`, FORMAT.md
R14|prompt cache|identical prefix → prefill 4.60s → 0.04s (~115x). cache is per-PREFIX & survives across requests|ollama 0.32.3 @ .181, measured
R15|edit locality|an edit invalidates all prefill AFTER it. same 7k pack: cached 0.04s · edit TAIL 0.49s · edit HEAD 4.61s (full cold)|measured, 3 runs
R16|prefill dominance|workload is prefill-bound ⊥ decode-bound. 7k: 4.60s prefill vs 1.83s decode. 28k: 30.32s vs 5.74s = 83% prefill|measured @ .181
R17|prefill superlinear|1,519 tok/s @ 7k → 1,233 @ 15k → 941 @ 28k. 4.09x tokens costs 6.59x time ∴ small packs pay off faster than linearly|measured, 3 points
R18|facet shares|itok SET 47.4% / SETTING 49.9% / HUMAN 2.7%; nanokit 49.1 / 47.6 / 3.3 ∴ ~half a repo never loads for impl work|measured, 2 repos (`nanokit` = `microlith`, renamed 2026-08-01)
R19|tests dominate|tests 34.0% itok · 24.9% nanokit = largest single facet. 42% of it INLINE in `#[cfg(test)]` ∴ ⊥ reachable by the dir axis|measured per file
R20|fleet duplication|guard-infra near-identical absolute size across unrelated repos: 16,058 vs 15,218 tok ∴ same scaffolding copied. × 54 repos ≈ 840k tok duplicated|measured, 2 of 54
R21|human docs grow|`set-and-setting` human 24,965 vs itok 5,236 = 4.8x. CHANGELOG alone 13,762 — append-only, never needed to implement|measured
R22|mermaid density|mermaid 71 tok vs 40 tok prose for the same info = 1.8x. bytes/tok: prose 4.15 · README 3.51 · mermaid 3.20 · caveman SPEC 2.95|measured
R23|caveman saving|MEASURED 22%, ⊥ the 75% FORMAT.md claims (10 / 13 / 35% on 3 invariants vs faithful prose). symbols cost 1-3 tok for 2-3 bytes ∴ the saving is STRUCTURAL (omit rationale), ⊥ encodative. n=3, own comparators|measured
R24|profile shares|itok by profile: implement 47.4% · tdd 81.4% · refactor 80.9% · harden 31.2% · document 17.5%. tdd @ ONE node = 6.1% ∴ 13x from composing facet × horizontal|derived from R18
R25|widening cost|same 11.6k pack: APPEND a facet 4.12s (prefix cached, only new tok prefill) · PREPEND 8.60s (full cold) = 2.1x ∴ optional facets belong at the END|measured @ .181
R27|notation cost|NOTATION slice 182 tok vs whole `FORMAT.md` 892 ∴ slicing saves 80%. doc comments in the surface cost 78 tok & were what let the judge tell prose from path|measured
R28|tdd loop cost|3 round-trips · 3,707 tok · max single call 1,651 vs 157,071 monolithic = 95x. step 3 (gates) = 0 tok. +15% max-call for notation+docs turned 3 failed runs into a correct one|measured, `sherd tdd` on src/fed V2
R29|premise gate|HEAD-TO-HEAD same node/invariants. monolith 1/2 green (V2 `E0425`, V3 green) · decomposed 2/2. TOTAL tok comparable (2,659 vs 2,712) ∴ decomposition ⊥ save total. MAX SINGLE CALL 2,659 vs 1,243 = 2.1x — that is the whole benefit, & it is the binding constraint|measured, `sherd oneshot` vs `sherd tdd`
R30|quality parity|when the monolith SUCCEEDS its output matches the decomposed form — `missing_not_owns(&e)`, `in_f`=3, composed ⊥ duplicated ∴ decomposition's win is FITTING, ⊥ quality, at this task size|measured
R31|scale curve|MONO call grows w/ BODIES, DECOMP max bounded by SIGNATURES. impl 375→5,218 tok: ratio 1.1x · 1.1x · 1.2x · 1.6x · 1.9x · 3.4x (predicted, no inference)|computed over 6 nodes
R32|scale, measured|large node (impl 5,218): MONO 9,029 tok one call, FAILED `E0425` · DECOMP max 3,155, GREEN = 2.9x on max call. small node (impl 1,118): 2.1x ∴ the gap WIDENS w/ node size|measured
R33|total ⊥ cheaper|DECOMP total varies w/ retries — 8,558 (5 trips) & 12,909 (7 trips) vs MONO 9,029 ∴ total cost is ⊥ a reliable win. MAX CALL is the consistent one|measured
R34|target box moved|target = 192.168.0.24, ollama 0.32.1, `gpt-oss:20b` 11.98G resident, `size_vram` = `size` ∴ 100% GPU ⊥ CPU spill, ctx 131,072 ALLOCATED. same residency profile as R10 (.181, 0.32.3) ∴ the KV arithmetic of R11 carries over unchanged|`/api/ps` + `/api/version` @ .24, measured
R35|prefill is 4x slower here|373 tok/s @ 9,048 · 320 @ 17,846 · 247 @ 36,070 vs R17's 1,519 / 1,233 / 941 ∴ .24 is 3.8-4.1x SLOWER at every size. superlinearity HOLDS: 3.99x tokens costs 6.03x time (R17: 4.09x → 6.59x) ∴ R17's SHAPE generalizes across hardware, its RATE ⊥. a rate is a fact about ONE box|`/api/generate` `prompt_eval_*`, 3 sizes, cache defeated by a unique prefix
R36|prefill dominance rises|decode 26-33 tok/s (`pace decode` carried 50 from .181). 36k pack = 146.3s prefill vs ~13s for a 400-tok reply = 92% prefill, where R16 measured 83% @ 28k on .181 ∴ SLOWER hardware makes the small-pack thesis STRONGER, ⊥ weaker — the penalty for a fat pack scales with how slow prefill is|derived from R35 + measured `eval_duration`
R37|judge is STABLE|blind lens, 3 runs × 2 arms × 5 items = 30 calls @ ~420 tok: 5/5 stubs REJECTED & 5/5 working fns ACCEPTED, identical ∀ 3 runs. zero variance ∴ `P(kept\|shape)` is a REAL parameter at this shape, ⊥ a coin flip — a known-good rung STAYS known-good ∴ exploit (`/sit`) is distinct from search (`/titrate`)|`cargo test -- --ignored`, 3 runs @ .24, both arms
R38|the corpus is BELOW the frontier|perfect separation on BOTH arms w/ ⊥ a single miss ∴ this task sits comfortably inside competence & LOCATES NOTHING. a test that never fails measures no boundary. next titration ! go UP (harder judge: longer fn, weaker invariant, ⊥-obvious stub) or SIDEWAYS (generation, where `src/tdd:T13` still reads 0 merit wins), ⊥ repeat this one|derived from R37
R39|judge boundary LOCATED|blind lens titration, 4 rungs, 2 runs IDENTICAL: `0-tells` 10/10 · `1-bare` 10/10 · `2-vague` 7/10 · `3-subtle` 6/8. ∴ the boundary is real AND stable AT THE EDGE, ⊥ only deep inside competence — R37 had only shown stability at 100%|`cargo test -- --ignored blind_lens_titration`, 2 runs @ .24, 76 calls
R40|invariant PRECISION is the axis, ⊥ prompt help|stripping `blind_prompt`'s enumerated tells cost NOTHING (10/10 → 10/10) though every recorded stub matches a clause near-verbatim ∴ the checklist was ⊥ doing the work — HYPOTHESIS FALSIFIED. vague `§V` wording costs 30%, the largest single drop, vs 25% for stubs no clause reaches ∴ cheapest lever on delegation = sharper `§V` rows, ⊥ prompt engineering. `src/lens:B1` & `src/plan:B1` reached this from the other direction|derived from R39
R41|context stopped binding|T41 made `--depth rule` select: whole repo 170,822 → 109,363 tok (-36%), root 11,576 → 7,638, `src/tdd` 17,127 → 9,886 (-42%). 13 of 13 chains were OVER ceiling, now 0 ∴ the fattest chain is 9.6% of working budget & R4's motivating number no longer describes THIS repo. what remains unsolved is competence, ⊥ context|`sherd budget` before/after T41
R42|precision drives WRITING too|generation titration, 5 pure fns × sharp\|vague × 3 runs, graded by `rustc` on HIDDEN tests the writer never saw: sharp 12/15 · vague 6/15 = 2.0x. per-item results IDENTICAL ∀ 3 runs ∴ deterministic, ⊥ underpowered — R40's judging effect carries to writing|`cargo test -- --ignored generation_titration`, 30 calls @ .24
R43|the TYPE is the cheaper channel|the 2x splits 3 ways, ⊥ evenly: `working` & `bucket` sharp 3/3 vague 0/3 — their rule is MAGIC NUMBERS (`ENTRY_COST`, 4 bucket bounds) no signature carries ∴ prose is the only channel. `verdict` & `for_path` 3/3 BOTH — `enum Verdict{Fits{slack},Over{by}}` & named params already encode the rule ∴ vague prose costs NOTHING. `is_yes` 0/3 both = beyond the frontier at every wording|derived from R42
R44|sharp wording is 3.3x|extended corpus, 11 pure fns × sharp\|vague × 3 runs, 66 calls, graded by `rustc` on HIDDEN tests: sharp 30/33 (91%) · vague 9/33 (27%). at SHARP wording the 20B wrote 10 of 11 correct ∴ the frontier for `one pure fn from a fixed signature` is FAR wider than assumed — `src/tdd:T13` 0 merit wins is evidence about the TASK SHAPE `sherd tdd` uses, ⊥ about capability|`generation_titration`, 66 calls @ .24
R45|the type ! carry the DECIDING rule, ⊥ the answer SHAPE|R43 pre-registered as classes, scored 8/11. `checked_working` predicted Type & behaved Prose: `Option<u64>` says CAN FAIL, ⊥ FAILS BELOW 28,543 ∴ shape ⊥ content. `parse_limit` & `escape_cell` predicted Beyond, both PASSED @ sharp ∴ a 3-rule parser & an escaper are INSIDE. both frontier misses erred PESSIMISTIC. `is_yes` alone is beyond at any wording|derived from R44
R46|plan coverage is 3 of 78|`sherd plan`: 3 actionable rows across the whole repo (`src/tdd` T13 — itself a MEASUREMENT row —, `src/ollama` T3, `src/review` T3), 69 unmanaged: 26 root rows w/ no `mod.rs` · 23 edit specs\|wiring ⊥ add a fn · 9 need `main.rs` · 7 write beyond their module · 2 replace ⊥ append · 2 research. ∴ ANY per-node `sherd tdd` experiment is bounded by TASKS ⊥ by clock, & 2 usable nodes is ⊥ a sample (`src/plan:B8` measured 5 of 21; it is now 3 of 78)|`sherd plan`, measured
R47|code size, ⊥ one runner|`src/tdd/mod.rs` 62,963B code (~4x V50's 4,000 tok) + 25,612B tests (~3x the 2,000). `src/cli` 25,122B, `src/ollama` 23,032B, both over. longest fns: `drive_from` 313 lines · `run_sampled` 100 · `expected_calls` 75 · `oneshot` 63. ⊥ a function-size rule anywhere, & V50's file rule has no runner (B11)|`wc`, split @ col-0 `#[cfg(test)]` as `split_module` does
R48|the density map named a NODE|`src/tdd` structural violations cluster into 3 concerns ⊥ spread evenly: the LOOP (`drive_from` 267 lines/cognitive 21 · `run_sampled` 85 · `oneshot` 54) · SOURCE READING (`expected_calls` 70/14 · `signatures` 50/12 · `split_module`) · the experiment harness. & the source-reading cluster SPANS `src/review` too (`public_fns` · `unwired`) ∴ V109 produced a CROSS-NODE finding, ⊥ a file-size complaint — the duplication is ⊥ inside `src/tdd` & a purely internal split would have preserved it|derived from the T85 map
R49|node context changes NOTHING|T82 re-run, fixed harness, 66 calls, 0 errors: `bare` 30/33 · `ctx` 30/33 @ a 10,009-tok pack. the SAME single item (`is_yes`) failed 3/3 in BOTH ∴ a TRUE null, ⊥ two noisy sets cancelling. BOUNDED: these items are self-contained BY DESIGN ∴ this says context is SAFE, ⊥ that context is useless — a task needing the node's spec would be a different measurement. `is_yes` has now failed across 4 conditions (sharp·vague·bare·ctx) ∴ T81 has a stable frontier item to dissect|`context_titration`, 66 calls @ .24
R50|coverage is 64.76%, ⊥ 98|MEASURED once the flake carried `cargo-llvm-cov` + `llvmPackages.llvm` (nixpkgs rustc ships no `llvm-tools-preview`): lines 64.76% · regions 65.01% · fns 70.80%. by node: `code` 98.3 · `lens` 94.5 · `spec` 92.5 · `tokens` 88.9 · `state` 88.0 · `fed` 87.1 · `ollama` 70.9 · `tdd` 69.0 · `plan` 66.5 · `slice` 63.6 · `review` 63.0 · `land` 55.2 · `assay` 54.0 · `cli` 0.0. `src/cli` alone is 445 uncovered lines ≈ 11% of the crate ∴ a 98% floor is 33 points & weeks away, & would gate every commit red meanwhile|`cargo llvm-cov --summary-only`, measured
R51|a self-authored test is ANTI-correlated|T83, 33 measurements, impl written BLIND in both arms so the ONLY variable is which tests grade it. hidden 30/33 · own 26/33. the 2x2: own PASS+hidden PASS 23 · own PASS+hidden FAIL 3 · own FAIL+hidden PASS 6 · own FAIL+hidden FAIL 0 · own ⊥ compiled 1. ∴ the model's own test caught 0 of 3 wrong impls & REJECTED 7 of 30 correct ones — worse than a coin flip, which would have caught ~1.5|`authorship_titration`, 66 calls @ .24
R52|it fails hardest where it matters|`is_yes` — the ONE item beyond the frontier @ any wording (R45) — failed hidden 3/3 & its own test CLEARED it 3/3 ∴ when the model cannot write the fn, it cannot write a test that detects that either. `for_path` & `escape_cell` had CORRECT impls rejected by their own tests 3/3 each ∴ the test encodes a different reading of the invariant than the impl does, deterministically|derived from R51
R53|the mutation remedy is REFUTED|33/33 authored tests KILLED a known-wrong stub — 0 survived, 0 ⊥ compiled. control: the hidden tests kill all 11 stubs ∴ the mutants are real. the check proposed after T83 would have caught NONE of its failures: the tests are ⊥ vacuous|`authored_tests_vs_mutants`, 33 calls @ .24
R54|the failure is OVER-specification|`escape_cell`'s authored test demands `" a | b "` → `"a\\|b"` (spaces round the PIPE gone, where the row trims the CELL) & that an already-escaped pipe stays. NEITHER is in the invariant ∴ a correct impl FAILS it 3/3. the model fills an underspecified row w/ plausible unstated rules; the impl — blind, same text — fills them differently|`target/authored-tests.txt`
R55|the detector re-found R54's row, BLIND & mechanically|T97, 11 rows × 3 runs, 66 calls, every run IDENTICAL. ONE row flagged — `escape_cell` 3/3 — & it is R54's row, which a HUMAN found by reading raw output after the fact. the GAP: the row says pipe → `\|` & cell trimmed, & is SILENT on an ALREADY-escaped pipe ∴ the impl escapes it twice, the test demands it stay, & BOTH readings are in the row (`FORMAT.md`'s own rule is silent identically). the invented rule MOVES — R54's test demanded 2, this one demands 1 — & the row is flagged either way. ⊥ reproduced: R52's `for_path`, 0/3 here ∴ that was pooled, ⊥ per-item. 2 of 11 rows yield NO verdict, deterministically: `sign` 3/3 ⊥ compiled (`[i64; 10]` holding 9 elements — an ordinary compile error, ⊥ a `.:src/tdd:B12` name mismatch) & `abort_budget_ms` 3/3 HUNG ∴ `src/assay:B2` was ⊥ a one-off & `src/assay:V6` paid for itself on run 1|`cargo test --test ambiguity -- --ignored`, 66 calls @ .24
R56|an ignored body reads as uncovered|MEASURED `.coverage` 75.50 → 75.11 when the `#[ignore]`d titrations moved from `src/assay` inline tests to `tests/`. an ignored test body counts in the coverage DENOMINATOR & never runs ∴ every experiment added inline LOWERS the number & a better instrument reads worse. carried out of `T96` when that row went — the row was history, the measurement is not|`tests/`, `cargo llvm-cov`

## §V INVARIANTS

V1: ∀ `§F` row → child dir exists ∈ fs
V2: ∀ `§F` row → child depth = parent depth + 1
V3: ∀ dir w/ `SPEC.md` & non-root → ≥1 parent `§F` row points at it
V4: federation graph ! DAG. cycle → `validate` exit 1
V5: root `SPEC.md` ! exist @ repo root
V6: node pack ≤ `budget.node` tokens (default 2000)
V7: chain pack (root→dir) ≤ `budget.chain` (default 8000)
V8: lens pack (chain + child `§F` lens) ≤ `budget.lens` (default 12000)
V9: node over `budget.node` → `check` emit split hint naming candidate child dirs
V10: ids namespaced by dir path — `src/plan:V3` ≠ `src/spec:V3` (both real, both V3). bare id = current node
V11: cross-node cite ! namespaced. bare cross-node cite → exit 1
V12: ∀ id monotonic **within node**. ⊥ reuse. APPEND ⊥ insert — insertion moves every citation below it
V13: invariant crossing 2+ children → ! live @ common ancestor, ⊥ duplicated per child
V14: bug @ leaf → `§B` @ leaf. cause spans 2+ nodes → invariant promoted to ancestor
V15: `lens` output self-contained @ its altitude — ⊥ require sibling|child body to act
V16: dir w/ source & ⊥ own `SPEC.md` → ! covered by nearest ancestor `§F`.lens, else `validate` warn
V17: token count via `itok` tier ≥ `bpe`. `dummy` (bytes/4) ⊥ for any gate — 15-30% off
V18: parse/DAG/budget/ceiling ⊥ call model. `--offline` → all cmds but prose gen work
V19: `route` descend one edge per step, reload only that child. ⊥ load whole tree
V20: `route` ambiguous → exit 3 + candidates. `route` miss → exit 2 naming what was tried. ⊥ exit 0 empty
V21: `§F`.tokens stale (≠ recomputed ±10%) → `check` flag
V23: ignore globs (`target/`, `.git/`) ⊥ walked, ⊥ ceiling-checked. per-FILE ignores too (generated, vendored)
V24: ∀ emitted token number ! carry method label. ⊥ bare int
V25: `ollama` tier unreachable → fall back `bpe` + warn stderr. ⊥ fall to `dummy`, ⊥ silent
V26: `§F`.tokens written & checked by same tier. tier switch → recompute all
V27: this repo ! valid federation. `sherd validate` on self exit 0, CI gate
V30: bootstrap — parse/DAG/budget land before self-spec written. ⊥ claim dogfood til self-validate green
V34: ∀ non-root `SPEC.md` ! carry `§N` — `up` ≥1, `self` = 1, `sib` = ∀ co-child
V35: root `§N` — `up` = `-`, `self` = `.`, ⊥ sib
V36: `§F` authoritative, `§N` generated. mismatch → `§F` wins, `sync` rewrites. ⊥ hand-edit `§N`
V38: `§N`.lens = verbatim copy of that dir's `§F`-row lens. single source
V39: multi-parent → `up` 2+ rows. `sib` = union ∀ parent, deduped
V40: `§N` alone ! answer "where am I, what is beside me" ⊥ opening another file
— two axes —
V42: federation has 2 axes. **horizontal** = dir depth. **vertical** = detail (`rule` → `why` → evidence). node over budget → vertical FIRST, horizontal only if rule-only still over. MEASURED 3x that horizontal ⊥ the lever: itok 59% of §V bytes stay @ root · itok 66% of stmts · sherd own 64%
V43: `§V`/`§B` statement = rule + rationale. rule inline @ `SPEC.md`, rationale @ `SPEC.why.md` keyed by id. MEASURED: rationale = 80% of `itok` §V, 73% of `microlith` ∴ vertical buys 5x vs horizontal 1.7x
V44: vertical split lossless **by reference ⊥ by deletion**. ∀ id ∈ `SPEC.md` → row ∈ `SPEC.why.md` | explicit `-`. rationale is where closed-option records live ∴ dropping it is the failure both sibling repos already guard
V45: `lens --depth rule` default. `why` pulled on demand, ⊥ resident. entry cost is re-billed EVERY turn
V46: budget sized against MEASURED entry cost, ⊥ raw window. measured: harness overhead ~28,543 tok before any file ∴ `budget.lens` + entry ≤ 40% of 131,072
— guards learned from `itok`/`microlith` §B —
V47: `§F`/`§N` are FORMAT extensions. microlith `check` fixes section set `G C I R V T B` ∴ unknown section ! be negotiated upstream. ⊥ ship a dialect microlith cannot read
V48: ∀ report ! state what was EXAMINED, ⊥ only what failed. node discovered & ⊥ parsed = FAIL, ⊥ skip. a pass on a section the parser cannot see is indistinguishable from a real pass
V49: split (either axis) ! PROVE item-set preserved before write — ids(parent) ⊆ ⋃ ids(children ∪ parent′), asserted pre-write. per-file losslessness is blind to content vanishing BETWEEN files
V50: `.rs` file **code** > `ceiling.file` (default 4000 tok) → `check` violation, kind `judgment`. tests counted separately vs `ceiling.test` (2000). ⊥ one ceiling over both
V51: `mod.rs`/`lib.rs` ceiling = `ceiling.mod` (default 1500) — tighter. it COMPOSES, ⊥ implements (`default.nix` does the same). `session/mod.rs` = 4,946 tok / 0% test = a default that grew a body
V52: ceiling forces a REVIEW, ⊥ a shrink. raise = reviewed event, reason in the commit. a ceiling set a hair above current turns every addition into a raise & trains the reflex it exists to catch
V53: `split` proposal ! report COUPLING after the cut, ⊥ sizes alone. a split that lowers bytes & raises cross-module refs is a regression
V54: `Mechanical` ⊥ mean easy — it means the tool computes the SINGLE answer. where to cut a module is ⊥ computable ∴ every `split` direction is `Judgment`. an agent applying Judgment blindly silences the guard
V55: ∀ printed id qualified (`sherd/V13`) — an unqualified id lands in the CONSUMER's namespace where it names a different rule. coordinates stay `file:line:`, id beside the message ⊥ inside them
V56: guard & rule ! share a UNIT. ceiling in `itok` tokens, cap in chars, order in lines — each states its unit at the point it gates
V57: CI globs by DATA DEPENDENCY ⊥ file extension. a step's glob ! name every input its tests read
V58: grammar ⊥ ship til run over a corpus. `§F`/`§N` recognition measured over the 54-spec fleet & FP rate reported. a false positive naming the wrong rule costs more than a false negative
V59: "X absorbs Y" ! name a MEASUREMENT ⊥ a belief. ⊥ delete|skip building Y til capability parity vs Y is written down. the deletion commit is where nobody re-checks
V60: premise MEASURED (R29/R32/R33): decomposition bounds the MAX CALL — 2.1x @ impl 1,118, 2.9x @ 5,218, widening w/ node size ∴ the claim is FITTING. ⊥ cheaper in total (varies w/ retries), ⊥ better in quality when both succeed. monolith 1/3 green vs decomposed 3/3
V61: ∀ guard proven by a PLANTED violation + a companion proving it accepts real shapes. ⊥ proven by reading it
V62: sibling divergence report — near-duplicate invariants across sibling nodes = V13 promotion candidates. V13 w/o a detector is a comment, ⊥ a guard
V63: registry-owned identifier checked by ASKING the registry, before it appears anywhere a user can run
— lens = decidable, ⊥ descriptive —
V64: sibling `§F` lenses ! EXHAUSTIVE — ⋃ child domains + what parent keeps = parent domain. else "matches no child" ⊥ mean "⊥ in subtree" & reader ! open all of them
V65: sibling `§F` lenses ! DISJOINT — 2 siblings claiming one ground → `route` ! open both ∴ V19 one-edge descent unsound. overlap → `validate` exit 1
V66: ∀ `§F` row ! state `⊥owns` + where it lives. a positive lens decides only DESCEND; a negative one decides STOP ∴ negative space is the byte that prevents loading. also guards the "rule never carried to a sibling path" class (5 of 15 `itok` bugs)
V67: lens written in vocabulary of the QUESTION ⊥ the implementation. `parses SPEC.md` ⊥ answer "why are bulleted ids rejected"
V68: `§F`.lens length scales w/ P(descend) × cost of a wrong descend, ⊥ w/ child size. the row is resident @ that altitude EVERY turn; the child costs only when opened
V69: ∀ `§V`/`§R`/`§B` row inline ≤ `cap.row` (default 200B). rationale by reference → `SPEC.why.md`. MEASURED: itok 656B/rule × 103 = 67.8KB; same COUNT @ 200B = 20.6KB ∴ per-row fatness (~5x) is the multiplier, ⊥ rule count
V70: `cap.row` enforced BEFORE rationale is written, ⊥ after. a spec compacted to satisfy a number trains the same reflex as a ceiling raised to satisfy one
— module as unit —
V71: facade dir named by CAPABILITY ⊥ vendor — `src/tokens/` ⊥ `src/itok/`. reader asks "count tokens" ⊥ "itok" (V67), & a swapped dep makes a vendor name lie. vendor named in `owns`/`⊥owns`
V72: ∀ external dep ! have ONE call site — its facade `mod.rs`. siblings private ∴ **compiler** enforces it (`error[E0603]`), ⊥ grep. VERIFIED cargo 1.96.1. guards the "rule never carried to a sibling path" class @ its source
V76: lens pack ordered STABILITY-DESCENDING — root, ancestors, then node. an edit invalidates every token of prefill AFTER it (R15) ∴ volatile content LAST. `pack()` order is load-bearing, ⊥ cosmetic
V77: federation's payoff on local hw is CACHE LOCALITY, ⊥ only fit. root+ancestor prefix byte-identical across ∀ node ∴ stays hot; only the leaf re-prefills. a monolith edited near its top pays full re-prefill EVERY turn (R15/R16)
V78: FACET = stable classification of content (impl · tests · spec · agents · guard-infra · guard-local · human). a property of the FILE, fixed
V79: facet value = token share × P(task ⊥ needs it). tests 34% × ~0.7 ≈ 24% · human 2.7% × ~0.95 ≈ 2.6% ∴ rank by the PRODUCT, ⊥ by size
V80: more facets help ONLY where a facet matches how tasks cluster. a facet no task selects is a manifest to maintain — R4 rejected that once already
V81: facets ! PARTITION — exhaustive & disjoint, same rule as sibling lenses (V64/V65). else content double-loads or vanishes between facets
V82: SETTING enters a pack as CONTRACT ⊥ implementation — one line per guard (`line cap 80`, `clippy pedantic`, `coverage floor 98`). ~200 tok replaces ~31k. a guard the agent cannot SEE is B1
V83: structural diagram GENERATED from `§F` (`graph --mermaid`), ⊥ authored. a hand-drawn architecture diagram is a second reading of what `§F` declares — microlith's founding defect
V84: guard-infra encoding FLEET standard is materializable (flake input, content-addressed). repo-specific facts — tests, `.context-limits`, baselines — STAY. ⊥ materialize what encodes THIS repo
V86: PROFILE = task-dependent SELECTION over facets. `set`/`setting` is the DEFAULT profile (`implement`), ⊥ a partition of the repo. `tests` ∈ setting under `implement` & ∈ set under `tdd` — the file ⊥ change, the TASK does
V87: axes COMPOSE multiplicatively. facet alone fails TDD — MEASURED 81.4% of itok still loads (B3). facet × horizontal @ one node = 6.1%, 13x smaller ∴ neither axis alone is sufficient
V89: pack layout = canonical facet ORDER, volatile last. widening mid-run re-prefills everything AFTER the insertion point ∴ declare the profile up front (V88) & build the pack once
V90: MEASURED widening cost — append 4.12s (only new tok, prefix cached) vs prepend 8.60s (everything) on the same 11.6k pack = 2.1x, & the gap grows w/ prefix size
V100: a principle a machine can CHECK belongs in a gate or a shape, ⊥ a prompt. MEASURED: structure never violated · gate evaded twice · prompt ignored entirely ∴ prefer structure > gate > prompt, & a principle that becomes a check should LEAVE the slice
V96: assets classed by AUDIENCE, ⊥ only by concern. **worker** → the 20B prompt · **supervisor** → the higher agent only · **human** → readers. supervisor text in a worker prompt is wasted tokens AND instructions aimed at the wrong reader — "revert this" means nothing to a model writing one function
V97: audience enforced by DISCOVERY, ⊥ convention. `.claude/`, `.github/`, `.codex/` ⊥ walked ∴ a `SPEC.md` dropped there can never become a node & can never reach a prompt
V92: TWO brains, split by what each can do. local 20B writes code from a narrow context; the higher agent JUDGES. MEASURED: gates passed wrong code 3x (stub w/ a doc comment saying so · data-laundering `sanitize_first_cell` · wrong-field test) & a human-level reader caught all 3
V98: a run duration is a FLOOR, ⊥ a ceiling. finish the cycle in flight, then check the clock — a run killed between `apply` and its review leaves generated code uncommitted, which already happened once (`src/fed:B7`)
V99: a run ENDS in a summary commit — cycles, what was applied & judged, anchors planted, halt cause, cost, what is next. autonomous means the report ! survive where a human finds it, same argument as V95
V95: a halt is COMMITTED (`--allow-empty`), ⊥ only printed. autonomous means nobody reads stdout ∴ the reason must survive in git where a human finds it later
V93: the loop STOPS on: nothing actionable · same row failed 2x · `check` unclean · 2 aborts @ 10x · 2 reverts in a row. an "infinite loop" w/o stop conditions optimizes for whatever the gate rewards
V94: nothing actionable → MAINTENANCE, ⊥ done. §B w/o §V · claims w/o runners · duplication · unmanaged rows · budgets · stale §R
V91: ONE primitive at every axis — a cheap summary that supports a decision + a pointer to the expensive thing. horizontal `§F` owns/⊥owns → child. vertical rule → `SPEC.why.md`. facet contract line → implementation
V88: profile DECLARED per task, ⊥ inferred. an inferred profile silently loads the wrong facets & the failure looks like a model that forgot
V85: `AGENTS.md` ∈ SET, ⊥ SETTING. it says HOW to work ∴ needed while working. guardrails say what is CHECKED after ∴ ⊥ needed while working
V74: §C claims ! have a runner. `fed::walk` contradicted §C for a whole session & no gate could see it (B1) — a constraint no check reads is a comment
V75: format facts read from the CHECKER's own source, ⊥ a vendored `FORMAT.md`. the local copy was 6 sections while the dep shipped 7 (B2)
V101: a dep ! resolve to an IMMUTABLE artifact — registry version + lock checksum. a sibling PATH dep is a shared working tree ∴ the gate's green is true only for the INSTANT it ran & expires silently when the sibling moves (B5). a new path dep ! carry a §B-recorded reason
V102: a gate ! declare its OP SET as data, ⊥ bury it in a hook body. green names only what RAN ∴ an op nobody declared is invisible, ⊥ merely absent — fmt & clippy were missing for the project's whole life & every verdict looked identical (B6)
V103: the criterion ⊥ WEAKEN as the rung narrows — a rung-4 task judged against the same `§V` as a rung-1 one, else the descent proves nothing. a granularize-until-success loop converges on TRIVIA by construction (`src/tdd:B2` = a judge loosening from "proves the invariant" to "would compile"). success below the verifiability floor is recorded UNVERIFIED, ⊥ kept
V104: a declared LIMIT ! have a runner that EXITS NONZERO — V74's shape, widened from §C to any number the spec names. V6/V7/V8 declare ceilings, `.context-limits` carries them, `sherd budget` PRINTS them & exits 0 ∴ 4 nodes drifted over unseen (B7). & a path w/ NO ceiling row is UNCHECKED ⊥ unlimited: absence ! read as violation or default, ⊥ as permission
V105: a declared OPTION ! CHANGE BEHAVIOUR. `Depth::Rule` is §I's default & selected nothing for the project's whole life ∴ every pack shipped archive & every ceiling was measured against it (B8). testable ∀ flag: same input, two settings, DIFFERENT output — a flag whose branches agree is a claim w/ no runner (V74)
V106: BUILDABLE on the 20B = (a) chain + the file it edits fits working budget & (b) ∃ a rung where the output SURVIVES REVIEW (⊥ merely the gate) & (c) setup cost < writing it by hand. (a) alone is what `budget` checks & is ⊥ SUFFICIENT — after T41 every chain is ≤10% of working & 0 merit wins remain. a module claimed buildable w/o (b) & (c) measured is a wish (`.:B4`: name the denominator)
V107: push the invariant into the TYPE & the SIGNATURE before sharpening its prose — & the type ! carry the DECIDING rule, ⊥ merely the SHAPE of the answer. MEASURED: where the type holds the rule (`enum Verdict{Fits{slack},Over{by}}`), vague wording costs 0; `Option<u64>` holds only `can fail` & its magic number still cost 100% (R45). prose precision is the fallback for what a type cannot hold, & it is worth 3.3x (R44)
V108: an experiment comparing 2 conditions ! differ in exactly ONE variable. the R44 → `src/tdd:T13` gap holds FIVE at once — pack size · signature given vs invented · tests hidden vs model-authored · judge in the loop · repair steps — ∴ ⊥ result across it is attributable & the honest form is one row per variable (R46). a 5-variable comparison that CONFIRMS a hypothesis is worth as little as one that refutes it
V109: clippy violation DENSITY = a 3rd dir-promotion trigger (V73 has 2). a file whose fns keep tripping `too_many_lines`/`cognitive_complexity` holds several concerns, & the CLUSTER names the SEAM — where V50 says only `too big` & leaves the boundary to eye. limits ∴ ⊥ hygiene: they are how federation boundaries are DISCOVERED ⊥ guessed
V110: decomposition is ⊥ uniformly costly — BREADTH & DEPTH differ & the rule must ⊥ discourage the cheap direction w/ the dear one. a SIBLING @ depth 2 pays root + `src` + itself & adds NOTHING to any existing chain ∴ promoting sideways LOWERS per-node cost & raises only the total-if-all-loaded, which nothing ever loads. a CHILD adds a level EVERY descendant pays EVERY turn (R6: hubs absorb 9.5%) ∴ depth is where splitting can RAISE the cost of working. prefer siblings. ∀ proposed split ! carry `sherd budget` before/after — this repo can MEASURE that curve where ordinary refactoring cannot
V111: a test the WRITER authored ⊥ grade that writer. MEASURED anti-correlated (R51): 0 of 3 wrong impls caught, 7 of 30 correct ones rejected. `src/tdd` step 1 authors the test that step 3's RED gate then requires ∴ the loop's red is satisfied by a test that does ⊥ measure the invariant, & `src/tdd:B2`/`B12` are that shape reported one at a time
V112: test-vs-impl DISAGREEMENT measures the INVARIANT, ⊥ the code. both written blind from ONE row, they disagreed 7 of 33 (R51), & the disagreements are gaps the row left open (R54) ∴ 2 calls + a compile is a mechanical AMBIGUITY DETECTOR for a `§V` row. it grades the SPEC — & unlike a mutation gate (refuted, R53) needs no reference answer
V113: GENERATED means a RUNNER regenerates it, ⊥ that it was generated once. an artefact spliced by hand & never re-spliced is indistinguishable from a hand-written one the day after, & the sentence claiming otherwise is the most misleading line on the page (B16). ∀ generated block ! carry markers, a generator, & a `--check` in the gate
V114: an EVEN minor is STABLE; an ODD minor is FUNCTIONAL & ⊥ for PRODUCTION. parity classifies the RELEASE, ⊥ the work in it — it answers *may I run this tag in production?*. pre-1.0 SemVer lets ANY minor break ∴ `0.4` vs `0.5` otherwise carries NOTHING a consumer can act on, & parity is the 1 bit the number carries FREE, before a changelog is read. ⊥ novel: Linux 2.x & GNOME shipped it ∴ a reader may already know it. `microlith` V34 states the same rule ∴ one reading serves both. RETIRES @ `1.0`, where every minor after is stable by definition
V115: §I is what SHIPS. ∀ verb the binary dispatches appears in §I, & ∀ §I cmd either dispatches or carries its rung — an interface section that advertises 5 absent verbs & hides 10 present ones is worse than none, ∵ a reader trusts it (B17)
V116: a COUNT derived from a tool's output is ⊥ a measurement until the tool is known to have RUN. `cargo clippy` prints no warnings for a target that fails to compile, `cargo llvm-cov` reports no lines for a suite that did ⊥ build, & every such silence reads as GOOD NEWS to a ratchet. ∀ step deriving a number ! refuse on the tool's own error before comparing (B18)
V117: the MODEL half is FROZEN until rung `0.7` (V114). `src/ollama`·`src/tdd`·`src/assay` WORK & no further development lands there before the mechanical surface is published. the 2 halves answer different KINDS of question: whether a dir DAG parses, budgets & validates is settled by TESTS, while whether a 20B writes code that survives review is a RESEARCH result that may take months ∴ tying a release to the 2nd holds the 1st hostage, & the 1st is the half a consumer can reuse. an open §T row in those 3 nodes is 0.7 work BY LOCATION, ⊥ by a label anyone has to maintain
V118: IDEMPOTENCE has 3 LAYERS & we have broken each exactly once. (1) WRITE — `f(f(x))=f(x)`, stated at `src/cli:V13`, broken by `src/cli:B4`. (2) MEASUREMENT — the number ! be a fn of the TREE ALONE: `.:B18` (clippy is silent on a target that ⊥ COMPILE ∴ 270→180 read as debt PAID) & `src/ollama:B8` (coverage 75.35 vs 75.26 on ONE tree ∵ the suite shares `.sherd-state` & line execution depends on run ORDER). (3) FEEDBACK — a measurement ! ⊥ read what its own generator WROTE, or generating MOVES the number & nothing converges (`src/plan:V18`). a ratchet asserts MONOTONE over a measurement ∴ a measurement that is ⊥ a fn of the tree makes the ratchet a guard firing on NOISE, & a ratchet ! also measure the SHIPPED config — `default = []` here, while every count we keep is `--all-features` (B20), & every one of the 3 was found by a HUMAN reading a number that ⊥ move the way the tree did
V119: `V50` is measured PER `.rs` FILE ∴ a node may hold a 2nd `.rs` BESIDE `mod.rs` & `§C`'s ban is on the 2018 `foo.rs`+`foo/` shape — a facade OUTSIDE the dir it fronts — ⊥ on a 2nd file INSIDE it. the 2nd file also gives `plan::structure` the `Declared` evidence it asks for: `split src/assay` \& `split src/cli` both answer "no module declarations found" ∵ nobody drew a line, & the tool refuses to guess (`src/plan:V16`). ∴ the fix to a node's SIZE is the same edit as the fix to the tool being BLIND to it. no node here has ever had a 2nd file — this states it is allowed, ⊥ that it is usual
V120: SIZE EQUALITY across nodes is ⊥ a health signal & optimising it costs boundaries. MEASURED: node impl 903 (`lens`) → 13,413 (`assay`), a 14x spread, while the gated number — the CHAIN — spans 9,922 → 12,480, a 26% spread ∵ every chain pays root + parent + self ∴ node variance is DAMPED by construction. `lens` @ 903 is one of the cleanest nodes here & equalising would merge it into something. equal size is a CONSEQUENCE of equal-grained concerns, ⊥ a cause of health. what IS a signal is size PLUS a lopsided `§B` count — `src/tdd` 31 bug rows vs `src/code` 0 says where defects live; size alone said nothing
V73: dir promotion has 2 triggers — (a) V50 code ceiling, (b) module owns SPEC worth its own node even under ceiling. vendor facades are (b): few hundred lines carrying V17/V24/V25. ⊥ promote every `.rs` — 30 files → 60 is ceremony

## §T TASKS

id|status|task|cites
T3|.|capability-parity audit `microlith` vs what `sherd` needs. write the comparison BEFORE relying on it|V59
T12|.|id namespacing + resolver `path:Vn`|V10,V11
T19|.|route ambiguous exit 3 / miss exit 2|V20
T23|.|coverage gap warn for un-specced source dirs|V16
T28|.|backprop: bug → leaf `§B`, decide promote|V14
T37|.|`sherd sync` + exit 1 when wrote|I,V36
T42|.|cross-file losslessness proof, asserted pre-write|V49,V44
T46|.|coupling report after proposed split|V53
T47|.|violation renderer `file:line: sherd/Vn:` + why + mechanical\|judgment + json `kind`|V55,V54
T48|.|sibling-divergence detector for V13|V62
T49|.|`§F`/`§N` upstream to FORMAT/microlith before shipping a dialect|V47
T50|.|corpus run over 54-spec fleet, FP rate reported|V58
T52|.|planted-violation test ∀ guard + accepts-real-shapes companion|V61
T58|.|`cap.row` gate — inline `§V`/`§R`/`§B` text over 200B|V69,V70
T59|.|PAY THE DEBT: record rationale for V1-V63 into `SPEC.why.md` before it accretes inline. rationale currently lives only in the design conversation|V69,V70,V44
T64|.|setting-as-contract extraction — guard files → one line each|V82
T67|.|materializability audit: which guard files are fleet standard vs repo facts|V84,R20
T68|.|report caveman 22%-⊥-75% upstream to cavekit FORMAT.md|R23
T69|.|profile declaration — flag > §T row > `sherd.toml` default. ⊥ inference|V88
T73|.|`/titrate` machinery — `.sherd-frontier` record, believability re-keyed node → SHAPE, cost ledger w/ the denominator named (`.:B4`)|V103,V60
T76|.|BUILDABILITY SWEEP: `sherd tdd` @ every node, N=3, record (node, rung, kept/tried) → `.sherd-frontier`. answers WHICH modules are buildable, ⊥ whether the idea works|V106,V103,R37
T80|.|audit `§V` rows for what a TYPE could carry instead (V107). the rows that need prose precision are the ones no signature can hold|V107,R43
T81|.|`is_yes` is the ONLY item beyond reach @ sharp wording (R45) — find WHY. 3 conjoined rules (first line · case-insensitive · trim) or something else? it is the one datapoint about what the frontier is MADE of|V107,R45
T86|.|build the FILE-ceiling runner V50 never had into `sherd check`, wire it like `budget`. §I already claims `check` does it (B11)|V50,V104,B11
T88|.|`no-commit-to-branch --branch main` in the PRE-COMMIT set only — ⊥ `all`, or CI on `main` fails itself (microlith's note). AGENTS.md claims it & nothing enforces it (B12)|V74,B12
T89|.|pub-fn ↔ test PAIRING via `sherd review` — reuse `public_fns`/`expected_calls`, ⊥ reimpl. catches what a % hides: a fn w/ NO test, carried by its neighbours|V72,V16
T91|~|`src/cli` FIRST: 445 lines, 0 tests, 0% — found twice by different instruments (T85 density map, R50 coverage). biggest single lever on the floor & the node w/ no test module at all|R50,V16
T92|~|`src/tdd`: move `RECORDED`/`VAGUE`/`SUBTLE`/`GEN_CORPUS`/`grade`/the titrations behind `#[cfg(test)]` — they are FIXTURES in the impl half — & decompose `drive_from` (267 lines, cognitive 21, worst in the repo). the source-reading half leaves via T93, ⊥ internally (R48)|V50,V109,V110,R48
T101|.|move the scripted-toolchain fixtures (`scratch`·`scripted_cargo`·`write_exec`·`repo_fixture`·`node_fixture`) `src/tdd` tests → `testrepo`, then the 7 gate tests follow the code T99 moved. today they sit in `src/tdd` testing `crate::land::` fns ∵ the fixtures do|V74,B15
T103|~|LADDER rungs as work: 0.4 = `split`+`sync` DONE · 0.5 = the MECHANICAL surface correct & reusable, measured on a foreign repo · 0.6 = settle · 0.7 = the model half resumes (V117). 0.1-0.3 reached|V114,V117
T104|.|lint ratchet ! also count `--no-default-features` — 131 warnings on the SHIPPED binary are uncounted today|V118,B20
T105|x|BUILD the `V50` check — per-`.rs` code & test ceilings, kind `judgment`. cited 5x as the rule that did the design work & computed nowhere|V119,B23

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`fed::walk` hand-rolled while §C says fs walk = `itok::walk`/`itok::glob`, ⊥ reimpl. wrote it w/o reading itok's walk API — the exact belief-⊥-measurement trap V59 names, committed in the first commit that could commit it. ⊥ caught by `check`: no runner reads §C|V74. port to `itok::walk` or amend §C w/ the measured reason itok's walk ⊥ fit
B2|2026-08-01|`§R` written as RECORDS w/ `id|state|record`. FORMAT 4.1.0 §R = RESEARCH `id|topic|finding|src`. assumed from a stale local `FORMAT.md` (6 sections) while the dep shipped 4.1.0 (7). 16 violations on first `sherd check`, all real|read the CHECKER's own `SECTIONS`/`CANONICAL_WORDS`, ⊥ a vendored copy. §R now RESEARCH; closed options → `.spec-records` (R13)
B3|2026-08-01|SPEC defect, mine: V78 written as a STATIC partition — SET = impl+spec+agents, SETTING = tests+guardrails — & committed. TDD breaks it: the test IS the work ∴ `tests` ∈ set, & nothing about the file changed. facet (property of the FILE) conflated w/ profile (property of the TASK). MEASURED after: `tdd` loads 81.4% of itok, `refactor` 80.9% ∴ the axis nearly collapses for 2 of 5 profiles|V78 now names the classification only; V86 adds PROFILE as the task-dependent selection; V87 records that facet × horizontal = 6.1% where facet alone = 81.4%. found by a READER asking about TDD, ⊥ by any check — no gate reads a partition's fitness for a workflow
B4|2026-08-01|SPEC defect, mine, repeated across ~6 commit messages: claimed "95x on the binding constraint" comparing our max call to 157,071 — which is `itok`'s WHOLE-REPO tdd profile, ⊥ what a one-call prompt needs. MEASURED baseline for the same task = 2,659 tok ∴ real ratio 2.1x. compared against a straw man nobody would build|`sherd oneshot` built as the honest monolith arm; R29/R30 carry the measurement; V60 restated. GENERALLY: a ratio ! name what is in the DENOMINATOR & that thing ! be something someone would actually do
B5|2026-08-05|HEAD stopped COMPILING w/ ⊥ sherd commit. `67fa9ad` (08-02 10:26) imported `microlith`'s inner `violation` module & the gate passed; microlith privatized it 9h later (`421ab02`, 08-02 19:16) ∴ E0603 on a tree already judged green, & already pushed. `../microlith` was a path dep ∴ no version could hold it still, & `Cargo.lock` read `0.4.0` for a sibling saying `0.5.0`|`microlith` = crates.io `0.5` + lock checksum (V101). `itok` too (T71) ∴ none left. `src/spec:B1` carries the import half
B6|2026-08-18|gate ran `build` + `test` ONLY, from the first commit that had a hook — no fmt, no clippy — ∴ an entirely unformatted tree & 18 clippy findings accrued behind a verdict that read green every time. the ops lived as a shell BODY in `.githooks/pre-commit` ∴ the SET of checks was never reviewable data & nobody could see what was ⊥ there|ops → `hk.pkl`, file-scoped & readable; fmt + clippy gated & the debt paid (`790bcf6`). V102. `-D warnings` moved off `RUSTFLAGS` so it stops reaching `../itok`
B7|2026-08-18|`.context-limits` declares per-node chain ceilings & `sherd budget` PRINTED the table w/o comparing against them or failing — T10 sat `.` from the first commit ∴ every chain drifted over unseen. w/ the comparison RUNNING it is 13 of 13, ⊥ the 4 first counted: root alone grew 10,060 → 11,576 in ONE session & every chain pays root|V104. T10 built the runner (exit 1). `src/land`/`src/slice` inherit `src` by longest prefix — they are over an INHERITED ceiling, ⊥ unbounded as this row first claimed
B8|2026-08-18|`lens::pack` reads the WHOLE `SPEC.md` ∀ chain member & consults `Depth` only to APPEND `SPEC.why.md` ∴ `--depth rule` — §I's documented DEFAULT — selected nothing, & `§R`/`§B` archive has ridden in every pack & every budget since the command existed. MEASURED 33% of root, 59% of `src/tdd`, 58% of `src/plan`. `rule_depth` was written, tested & correct in `src/tdd` the whole time, called only by the tdd worker path|V105. T41 moves `rule_depth` to the owner & makes `Depth` select; T75 re-measures after
B9|2026-08-18|`sherd lens <dir>` passes `PathBuf::from(d)` straight to `pack` while `budget`/`fed` go through `arg_dir`, which T10 fixed to resolve against ROOT ∴ `lens .` reports a 2-node chain where `budget` reports 1 node for the same target, & from a subdirectory `lens` reads a truncated chain w/ no error. the same relative-vs-absolute defect T10 fixed, one command over, missed because the fix was applied to the HELPER & ⊥ to every caller|T78. GENERALLY: fixing a shared helper ! be followed by finding who does ⊥ use it
B11|2026-08-19|V50 declared a `.rs` file code ceiling from the first commit & `check()` runs ONLY `spec::check` + `unreflected_bugs` ∴ it has no runner — while §I claims `sherd check` does "drift spec↔code + file ceilings", which is FALSE. 3rd of this family in one week (B6 gate ops nobody declared, B7 chain ceilings nobody compared). MEASURED 4x over @ `src/tdd` (R47), unseen for the project's life|V104. T86 builds the runner; T85'"'"'s clippy limits catch the FUNCTION half V50 never covered
B12|2026-08-19|`AGENTS.md` says "Branch only. Never commit to `main`" & nothing enforces it ∴ ~20 commits landed on `main` in one session, mine, unchallenged & unremarked until a reader asked about something else. the rule was READ by the agent it governs & still lost to convenience|V74 again — a rule w/ no runner is a comment. T88 gates it in the PRE-COMMIT set only (⊥ `all`: CI runs `check --all` on `main` & would fail itself)
B13|2026-08-19|TWO readings of "parse Rust source" shipped across nodes: `src/tdd` has `signatures` (declarations + shapes) · `expected_calls` (call sites) · `split_module` (test boundary), `src/review` has `public_fns` (declarations) · `unwired` (declarations vs calls). both line-oriented heuristics over the same text, both already w/ §B rows for reading it wrong (`src/tdd:B13`, `src/tdd:B18`, `src/review:B2`) ∴ the founding defect §C names, in the repo that exists to end it. unseen until the T85 density map crossed a node boundary — no per-file gate can see a duplication that spans two files|T93 promotes `src/code` as the ONE owner. `split_module`'s own doc already says "two readings of one rule is the defect this project exists to end"
B14|2026-08-19|writing `src/assay/SPEC.md` in T94 I RESTATED 3 invariants that already existed — `V1`≡`.:src/tdd:V27`, `V3`≡`.:V103`, `V4`≡`.:V108` — instead of moving them ∴ two readings of one rule, the founding defect §C names, introduced ONE COMMIT after B13 recorded it & inside a node created to END a duplication. a new node's spec is written from the concern, ⊥ from the rows the concern already had, & nothing checks that|V74. T95 deleted the dupes. GENERALLY: promoting a node ! START by listing the rows that already own the concern — authoring fresh guarantees a second reading
B15|2026-08-23|`cargo build --no-default-features` FAILS — 4 errors, `src/assay` & `src/land` reach `crate::tdd` which is `ollama`-gated ∴ the featureless build has been broken since the feature split, while `Cargo.toml` DOCUMENTS it as "the deterministic, networkless core that §C demands". a claim in a manifest w/ no runner, unseen ∵ nothing ever built that configuration|T99 moved the gate cluster to `src/land` — its owner: `land` DECIDES whether work earned its merge — & gated `cargo build --no-default-features` in `hk.pkl`. `src/assay` is now feature-gated w/ the loop it measures. V74 again — & found by writing `default-features = false` in `dev/Cargo.toml`, ⊥ by any check
B16|2026-08-23|`.:README` Architecture said "generated by `sherd graph` — never hand-drawn, so it cannot drift" & had drifted: 7 nodes drawn, 17 real, missing `plan`·`review`·`state`·`slice`·`land`·`cli`·`code`·`assay`·`dev`. GENERATED once, by hand, then never again — the claim named a PROPERTY of the output & no runner held it. same shape as the badges one commit earlier, in the same file, w/ the sentence asserting the opposite ∴ a reader was told to trust the stalest thing on the page|V113. `sherd-dev readme` splices all 4 blocks from `fed::tree`/`mermaid`/`table` + the badge facts, gated by `readme-generated`. GENERALLY: "generated" is a claim about the LAST run, ⊥ about the artefact — it needs a checker like every other rule (V74)
B17|2026-08-23|§I specifies 9 cmds & the binary dispatches 14: `init`·`route`·`split`·`sync`·`validate` are SPECCED & unbuilt, while `apply`·`ask`·`fed`·`land`·`oneshot`·`outcome`·`plan`·`review`·`slice`·`tdd` SHIP & §I never names them. drift in BOTH directions, in the section a spec-driven repo can least afford wrong. `src/cli:V7` gained a runner for usage-vs-dispatch the same day & §I-vs-dispatch — one layer up — still had none ∴ 10 verbs shipped unannounced|V115. T102 builds the runner in `sherd-dev`, which already reads dispatch arms for V7. GENERALLY: a fix applied at one layer ! be followed by asking which layer above it has the same hole (`src/fed:B9` again)
B18|2026-08-23|the LINT RATCHET counted a BROKEN BUILD as an improvement. `cargo clippy` emits no warnings for a target that fails to COMPILE ∴ when a test target broke, the count fell 270 → 180 & the step read that as debt paid. MEASURED live: a `write_spec` rename left one call site dangling, & 90 warnings from `#[cfg(test)]` code in `src/*` silently stopped being counted. the `fix` half is worse — it would have RECORDED 180 as the new floor, filing the ratchet's teeth down w/ a number nobody earned|V116. both halves now capture clippy's output, refuse on `^error`, & say the count is ⊥ a measurement. PROVEN by planting a broken test: exit 101 naming the cause, exit 0 when restored. GENERALLY: a metric derived from a TOOL'S OUTPUT ! first check the tool RAN — `.:V26` for a subprocess, & the 4th recording of that shape (B17/B20/B24 in the siblings)
B19|2026-08-23|1st run against a FOREIGN repo (`rekall`) & the ignore list was tuned to OUR tree: `result/` (a nix build symlink) & `pkl/` (the vendored hk schema) were advised as missing `§F` rows ∴ a stranger is told to federate their build dir. `route` on an UNFEDERATED repo answered "no node matched" — true & useless, ∵ w/ 1 node & root excluded ⊥ query can EVER hit ∴ the asker rephrases a query that cannot succeed. neither is visible from inside a repo shaped by the tool that reads it|both fixed + `result-*`·`vendor` added. `route` names the real cause & points at `sherd init`. GENERALLY: a default tuned on 1 tree is a claim about ALL trees — the 2nd repo is where it gets tested, & `.:T103`'s rung 0.5 exists ∵ of exactly this
B20|2026-08-23|the lint ratchet counts `--all-features` & we SHIP `default = []` ∴ the measured configuration is ⊥ the installed one. MEASURED: `cargo clippy --all-features` = 0 warnings, plain `cargo clippy` = 131 — incl. `plan::preflight` DEAD (reached only under `ollama`) & an unneeded `mut` in `cli::run_args`. the featureless step asks only whether it BUILDS (B15), which it does ∴ the shipped binary's warnings have been invisible since the default flipped to `[]` (V113) & the ratchet has been green over a config nobody installs. found by reading `cargo run` stderr while measuring something else, ⊥ by any gate|V118. T104 — count both configs. B18 stopped the ratchet reading a BROKEN build as progress; this is the same fault one level out: reading the WRONG build
B21|2026-08-23|the lint ratchet COUNTS warnings ∴ `too_many_lines` PUNISHES the split it exists to force: `tdd::drive_run` is 269 lines & ONE warning, & 5 pieces of 50 would be FIVE ∴ every honest refactor of the worst function in the tree makes the number WORSE & the gate refuses it. found by looping paydown to exhaustion & asking why no round paid — ⊥ by any run. MEASURED: 76 fns over the limit, 2,822 lines in them, 1,682 EXCESS. 17 of the 76 are 16-19 lines ∴ a count also weighs "1 line over" the same as 269|V118. a 2nd ratchet, `shape` = Σ(len−15), beside `total`; NEITHER may rise ∴ strictly stronger than one number. splitting 269 into 5×50 reads 254 → 175, which is the progress it is. the count STAYS ∵ it still catches a new site; the 2 answer different questions & 1 number let a safety fix pay for a shape regression. GENERALLY: a ratchet ! measure the thing it wants MOVED, & `.:B20` is the same fault on the other axis — measuring the wrong build vs measuring the wrong property
B22|2026-08-23|the lint ratchet was an ABSOLUTE count ∴ it taxed ADDING code: a commit of 200 CLEAN lines + 1 `unwrap` FAILED, which is backwards — the tree got cleaner per line & the gate refused it. MEASURED at the switch: 271 warnings & 1,682 excess lines over 15,909 lines of Rust = 1 warning per 58 lines, & 1 line in 10 inside a fn past the limit. the same 200-line commit reads 17.0 → 16.8 as a RATIO & passes|V118,B21. `.lint-debt` now gates 2 RATIOS — `density` (warnings/KLoC) & `shape` (excess lines as a share of the tree) — & records `count`/`excess`/`loc` as context. `.coverage` has worked this way since it existed: a percentage that may ⊥ fall, w/ the line count unbounded ∴ the 2 ratchets in this repo now agree on their own shape. RISK stated ⊥ hidden: a ratio can be DILUTED by clean-but-pointless code; the recorded absolutes make that visible & `.:V50`'s code ceiling is the answer if it ever happens. `src/land::lint_debt_ok` compares the SAME ratio ∵ a loop predicting the gate ! read the number the gate reads (`src/tdd:B30`)
B23|2026-08-24|`V50` sets a CODE CEILING — 4,000 tok impl, 2,000 tests, counted separately — & NOTHING computes it: `check` reports 0 violations & has since the row was written. it is cited in 5 source files as the rule that forced a split, ∴ it has been enforced by being QUOTED at an author, which is `V100`'s PROMPT tier, the weakest of the 3 it ranks by measurement. MEASURED once asked: 4 of 14 nodes over the impl ceiling (`assay` 13,413 · `cli` 12,510 · `tdd` 10,534 · `ollama` 7,695) & 11 of 14 over the test ceiling, `plan` @ 15,578 = 7.8x. FOUND by a question about 2 nodes' sizes, ⊥ by any gate|V119. T105 builds it. NOTE before enforcing: 11 of 14 over the TEST ceiling says 2,000 was set before the suite reached this size ∴ re-derive that number from measurement rather than reshaping the tree to fit a figure nobody has revisited. same class as `src/plan:B16` — root POLICY no gate reads
