# SPEC

## §G GOAL

`blackbox` (cmd `bbx`) — two-axis federation of `SPEC.md` + Rust source over a dir DAG, so a 128k local model works at one altitude & pulls deeper only when measured cost says it must.

MOTIVATING NUMBER: `itok` = 135,096 tok (28,462 spec + 106,634 code) vs 102,529 working on the target box. An 11,291-line CLI ⊥ fit its own best-case hardware.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|code nodes — tokens, spec, fed, lens facades & logic|inference harness, endpoint config|-

## §C CONSTRAINTS

- lang: Rust **edition 2024**. stable. MSRV **1.95** = the FLEET PIN (`nixpkgs-lock` → nixos-26.05). ⊥ a number copied from a sibling: `itok`/`microlith` declare 1.96 & MEASURED compile clean on 1.95 ∴ their floor is a mirror of an old pin, ⊥ a minimum.
- nixpkgs rev FOLLOWED from `nixpkgs-lock`, ⊥ spelled here. one rev, ~80 repos.
- target model: `gpt-oss:20b`, 131,072 ctx, local. ⊥ cloud fallback.
- inference: local HTTP (Ollama) only. ⊥ network otherwise.
- deterministic core: parse/DAG/budget/ceiling = pure Rust, ⊥ model. model ? prose gen & drift judgement only.
- separator = **directory**. dir tree ! source of truth. ⊥ manifest, ⊥ name-encoded grouping (`core-parse` ⊥ imply parent).
- federation edge = parent dir → child dir, depth **+1 exactly**. ⊥ skip.
- graph ! DAG. cycle ⊥. re-parent (2+ parents) OK.
- intra-file spec ops = `microlith` lib dep (crates.io 0.6, zero-dep, pure fn over `&str`). ⊥ reimpl parse/fmt/check/anchors.
- token counting = `itok` lib dep (`../itok`, 0.3.0). ⊥ own tokenizer, ⊥ own bytes/4.
- fs walk + ignore globs = `itok::walk`/`itok::glob`. ⊥ reimpl.
- SPEC syntax = FORMAT **4.1.0**, sections `G C I R V T B` fixed & ordered + `§F`/`§N`. `§F`/`§N` ! land in FORMAT/microlith upstream, ⊥ invented locally (V47).
- layout: **one crate**. module = **dir + `mod.rs`** (the `default.nix` shape — dir is the unit, entry is conventional). ⊥ 2018 `foo.rs`+`foo/`: that puts the facade OUTSIDE the dir it fronts ∴ module entry & its `SPEC.md` land in different federation nodes.
- node = dir = Rust module. all three aligned or a flat `.rs` owns spec it cannot hold.
- repo partitions **set** \| **setting** \| **human** (`set-and-setting` vocabulary). default pack = set.
- caveman encoding ∀ generated spec text. MEASURED 22% saving ⊥ 75% (R23) — it disciplines saying LESS, ⊥ encodes denser.
- gate runner = `hk` (from `nix-hk`; nixos-26.05 ships none — landed on master after branch-off). ops DECLARED in `hk.pkl`, ⊥ a shell body in `.githooks`. schema VENDORED `pkl/Config.pkl` ∴ the gate runs w/ ⊥ network.
- SETTING as contract (V82), one line each: `rustfmt` 80 col + edition 2024 · `clippy -D warnings` AFTER `--` ∴ this crate ⊥ its path deps · `cargo test` · `bbx slice --check` · `bbx check`.
- ⊥ global index file. discovery by walk.

## §I INTERFACES

- cmd: `bbx init [dir]` → scaffold `SPEC.md` @ dir, `§F` rows from child dirs
- cmd: `bbx lens <dir> [--depth rule|why|all]` → context pack. default `rule`
- cmd: `bbx lens <dir> --json` → `{chain:[],body:{},children:[],tokens:{},examined:{}}`
- cmd: `bbx route "<query>"` → dir + reason. 0 hit / 2 miss / 3 ambiguous
- cmd: `bbx check [dir]` → drift spec↔code + file ceilings. 0 clean / 1 violation / 2 usage
- cmd: `bbx split <path>` → propose split of over-ceiling file|node. ⊥ write w/o `--apply`
- cmd: `bbx sync [dir]` → regen `§N` from parent `§F`. exit 1 if wrote
- cmd: `bbx graph [--dot|--json|--mermaid]` → federation DAG. `--mermaid` = the generated architecture diagram
- cmd: `bbx lens <dir> [--facet set|setting|human|all]` → default `set`
- cmd: `bbx budget [dir]` → node/chain/lens/file token table. exit 1 over
- cmd: `bbx validate` → DAG + ids + budget + coverage + examined-count. exit 1 fail
- file: `SPEC.md` ∀ dir any depth. `§G §C §I §R §V §T §B` + `§F` + `§N`
- file: `§F FEDERATION` pipe table `dir|owns|⊥owns|tokens` — child dir depth +1. `⊥owns` = what it does NOT own + where that lives
- file: `§N NAV` pipe table `rel|path|lens`, `rel` ∈ `up`|`self`|`sib`. generated, ⊥ hand-edit
- file: `SPEC.why.md` ∀ dir w/ rationale — `<id>|<rationale>`, addressed by `§V`/`§B` id
- file: `.context-limits` — per-path ceilings, `itok` format, reused ⊥ reinvented
- file: `.spec-records` — closed-option baseline, `microlith --records`
- file: `.claude/commands/sit.md` — `/sit`, ONE oversight cycle. loop-safe, halts w/ a recorded reason
- file: `.claude/commands/titrate.md` — `/titrate`, granularity descent. attempt → enrich pack | split task. floor = VERIFIABILITY ⊥ size
- file: `.bbx-frontier` — `shape rung= kind= pack= sig= tests= tried= kept=`. TRACKED, ⊥ `.bbx-state`: learning that dies at the clone boundary ⊥ learning
- file: `AGENTS.md` — supervisor class. auto-loaded by Codex & Claude, ⊥ reachable by a worker prompt (V97)
- env: `BBX_MODEL` (`gpt-oss:20b`), `BBX_ENDPOINT` (`http://localhost:11434`)
- lib: `microlith::check_spec(&text,&records)`, `microlith::fmt`, `::anchors`
- lib: `itok::estimate`, `itok::walk`, `itok::glob`
- violation: `<file>:<line>: bbx/<Vn>: <msg>` + `why` + `mechanical`|`judgment`. `--format json` carries `kind` as data

## §R RESEARCH

id|topic|finding|src
R1|crates.io name|`bbx` taken (BBCode parser, lib-only, `bin_names: []`). BIN name `bbx` free on brew, debian, PATH ∴ package≠bin|crates.io API, formulae.brew.sh, sources.debian.org
R2|cargo workspace|`members=["crates/*"]` ERRORS on a hub dir w/o `Cargo.toml`. per-depth globs + cross-nested path deps build clean|cargo 1.96.1, tested
R3|rust module form|`mod.rs` = the `default.nix` shape. private sibling → `error[E0603]` ∴ COMPILER enforces the facade, ⊥ grep|cargo 1.96.1, tested
R4|itok size|`SPEC.md` 28,462 tok + code 106,634 = 135,096 ∴ 103% of a 128k window. an 11,291-line CLI ⊥ fit|itok 0.2.0 `--bpe`
R5|spec fatness|itok 656B/rule × 103 rules. rationale = 80% of §V (microlith 73%) ∴ per-row FATNESS is the multiplier, ⊥ rule count|measured over both §V
R6|federation yield|dir federation moves 41% of itok §V bytes; intermediate hubs absorb 9.5%; bbx own §V 64% stays root|classified 103 + 55 rules
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
R28|tdd loop cost|3 round-trips · 3,707 tok · max single call 1,651 vs 157,071 monolithic = 95x. step 3 (gates) = 0 tok. +15% max-call for notation+docs turned 3 failed runs into a correct one|measured, `bbx tdd` on src/fed V2
R29|premise gate|HEAD-TO-HEAD same node/invariants. monolith 1/2 green (V2 `E0425`, V3 green) · decomposed 2/2. TOTAL tok comparable (2,659 vs 2,712) ∴ decomposition ⊥ save total. MAX SINGLE CALL 2,659 vs 1,243 = 2.1x — that is the whole benefit, & it is the binding constraint|measured, `bbx oneshot` vs `bbx tdd`
R30|quality parity|when the monolith SUCCEEDS its output matches the decomposed form — `missing_not_owns(&e)`, `in_f`=3, composed ⊥ duplicated ∴ decomposition's win is FITTING, ⊥ quality, at this task size|measured
R31|scale curve|MONO call grows w/ BODIES, DECOMP max bounded by SIGNATURES. impl 375→5,218 tok: ratio 1.1x · 1.1x · 1.2x · 1.6x · 1.9x · 3.4x (predicted, no inference)|computed over 6 nodes
R32|scale, measured|large node (impl 5,218): MONO 9,029 tok one call, FAILED `E0425` · DECOMP max 3,155, GREEN = 2.9x on max call. small node (impl 1,118): 2.1x ∴ the gap WIDENS w/ node size|measured
R33|total ⊥ cheaper|DECOMP total varies w/ retries — 8,558 (5 trips) & 12,909 (7 trips) vs MONO 9,029 ∴ total cost is ⊥ a reliable win. MAX CALL is the consistent one|measured
R34|target box moved|target = 192.168.0.24, ollama 0.32.1, `gpt-oss:20b` 11.98G resident, `size_vram` = `size` ∴ 100% GPU ⊥ CPU spill, ctx 131,072 ALLOCATED. same residency profile as R10 (.181, 0.32.3) ∴ the KV arithmetic of R11 carries over unchanged|`/api/ps` + `/api/version` @ .24, measured
R35|prefill is 4x slower here|373 tok/s @ 9,048 · 320 @ 17,846 · 247 @ 36,070 vs R17's 1,519 / 1,233 / 941 ∴ .24 is 3.8-4.1x SLOWER at every size. superlinearity HOLDS: 3.99x tokens costs 6.03x time (R17: 4.09x → 6.59x) ∴ R17's SHAPE generalizes across hardware, its RATE ⊥. a rate is a fact about ONE box|`/api/generate` `prompt_eval_*`, 3 sizes, cache defeated by a unique prefix
R36|prefill dominance rises|decode 26-33 tok/s (`pace decode` carried 50 from .181). 36k pack = 146.3s prefill vs ~13s for a 400-tok reply = 92% prefill, where R16 measured 83% @ 28k on .181 ∴ SLOWER hardware makes the small-pack thesis STRONGER, ⊥ weaker — the penalty for a fat pack scales with how slow prefill is|derived from R35 + measured `eval_duration`
R37|judge is STABLE|blind lens, 3 runs × 2 arms × 5 items = 30 calls @ ~420 tok: 5/5 stubs REJECTED & 5/5 working fns ACCEPTED, identical ∀ 3 runs. zero variance ∴ `P(kept\|shape)` is a REAL parameter at this shape, ⊥ a coin flip — a known-good rung STAYS known-good ∴ exploit (`/sit`) is distinct from search (`/titrate`)|`cargo test -- --ignored`, 3 runs @ .24, both arms
R38|the corpus is BELOW the frontier|perfect separation on BOTH arms w/ ⊥ a single miss ∴ this task sits comfortably inside competence & LOCATES NOTHING. a test that never fails measures no boundary. next titration ! go UP (harder judge: longer fn, weaker invariant, ⊥-obvious stub) or SIDEWAYS (generation, where `src/tdd:T13` still reads 0 merit wins), ⊥ repeat this one|derived from R37

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
V10: ids namespaced by dir path — `src/parse:V3` ≠ `src/route:V3`. bare id = current node
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
V27: this repo ! valid federation. `bbx validate` on self exit 0, CI gate
V30: bootstrap — parse/DAG/budget land before self-spec written. ⊥ claim dogfood til self-validate green
V34: ∀ non-root `SPEC.md` ! carry `§N` — `up` ≥1, `self` = 1, `sib` = ∀ co-child
V35: root `§N` — `up` = `-`, `self` = `.`, ⊥ sib
V36: `§F` authoritative, `§N` generated. mismatch → `§F` wins, `sync` rewrites. ⊥ hand-edit `§N`
V38: `§N`.lens = verbatim copy of that dir's `§F`-row lens. single source
V39: multi-parent → `up` 2+ rows. `sib` = union ∀ parent, deduped
V40: `§N` alone ! answer "where am I, what is beside me" ⊥ opening another file
— two axes —
V42: federation has 2 axes. **horizontal** = dir depth. **vertical** = detail (`rule` → `why` → evidence). node over budget → vertical FIRST, horizontal only if rule-only still over. MEASURED 3x that horizontal ⊥ the lever: itok 59% of §V bytes stay @ root · itok 66% of stmts · bbx own 64%
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
V55: ∀ printed id qualified (`bbx/V13`) — an unqualified id lands in the CONSUMER's namespace where it names a different rule. coordinates stay `file:line:`, id beside the message ⊥ inside them
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
V73: dir promotion has 2 triggers — (a) V50 code ceiling, (b) module owns SPEC worth its own node even under ceiling. vendor facades are (b): few hundred lines carrying V17/V24/V25. ⊥ promote every `.rs` — 30 files → 60 is ceremony

## §T TASKS

id|status|task|cites
T1|x|scaffold single crate `bbx`, module=dir+`mod.rs`, explicit `[[bin]]`, deps `itok`+`microlith` by path|C,R1,R3
T2|x|bind `microlith` — `check`, `fmt`, section split. ⊥ reimpl|C
T3|.|capability-parity audit `microlith` vs what `bbx` needs. write the comparison BEFORE relying on it|V59
T4|x|parse `§F` table → (dir, owns, ⊥owns, tokens), escape-aware|I,V1
T6|x|superseded — edges/chain/discover land; depth & cycle are `src/fed:T4`|V1,V2,V4
T8|x|bind `itok::estimate`, tier floor `bpe`, method label|V17,V24
T10|.|`bbx budget` + over-budget exit 1|V6,V7,V8
T12|.|id namespacing + resolver `path:Vn`|V10,V11
T14|x|`bbx lens` render pack, `--depth rule` default|I,V15,V45
T16|x|moved — `src/cli:T4`|I
T17|x|ollama client `BBX_MODEL`/`BBX_ENDPOINT`|I,V25
T19|.|route ambiguous exit 3 / miss exit 2|V20
T20|x|model-free path — `--no-default-features` drops `ollama` & its deps|V18
T21|x|moved — `src/cli:T6`|I
T23|.|coverage gap warn for un-specced source dirs|V16
T24|x|moved — `src/cli:T7`|I
T25|x|moved — `src/cli:T3`|I,V48
T28|.|backprop: bug → leaf `§B`, decide promote|V14
T36|x|superseded — `§N` derivation is `src/fed:T6`, at the node that owns it|V36,V38,V39
T37|.|`bbx sync` + exit 1 when wrote|I,V36
T41|.|`lens --depth rule\|why\|all`|V45
T42|.|cross-file losslessness proof, asserted pre-write|V49,V44
T46|.|coupling report after proposed split|V53
T47|.|violation renderer `file:line: bbx/Vn:` + why + mechanical\|judgment + json `kind`|V55,V54
T48|.|sibling-divergence detector for V13|V62
T49|.|`§F`/`§N` upstream to FORMAT/microlith before shipping a dialect|V47
T50|.|corpus run over 54-spec fleet, FP rate reported|V58
T52|.|planted-violation test ∀ guard + accepts-real-shapes companion|V61
T53|x|PREMISE GATE run — `bbx oneshot` vs `bbx tdd`, R29/R32/R33. bounds MAX CALL 2.1x→2.9x, ⊥ total, ⊥ quality|V60,V27
T54|x|self-federate: root `§F` + per-node `SPEC.md`|V27,V30
T55|.|CI: `bbx validate` self exit 0; globs by data dependency|V27,V57
T56|x|`§F` gains `⊥owns` column — parse & emit|I,V66
T57|x|superseded — `src/fed:T5`/`T7`: duplicate rows fail, missing rows advisory|V64,V65
T58|.|`cap.row` gate — inline `§V`/`§R`/`§B` text over 200B|V69,V70
T59|.|PAY THE DEBT: record rationale for V1-V63 into `SPEC.why.md` before it accretes inline. rationale currently lives only in the design conversation|V69,V70,V44
T60|x|facades land — `itok::` only in `src/tokens`, `microlith::` only in `src/spec`|V71,V72
T64|.|setting-as-contract extraction — guard files → one line each|V82
T65|x|`graph --mermaid` generated diagram|V83
T67|.|materializability audit: which guard files are fleet standard vs repo facts|V84,R20
T68|.|report caveman 22%-⊥-75% upstream to cavekit FORMAT.md|R23
T69|.|profile declaration — flag > §T row > `bbx.toml` default. ⊥ inference|V88
T70|x|V101 runner — path deps limited to the one recorded exception (`itok`)|V101
T71|.|publish or public-mirror `itok` ∴ ⊥ path dep left, & `packages.default` becomes buildable|V101,R20
T72|x|gate → `hk` from `nix-hk`, ops in `hk.pkl`, `pre-commit` + `pre-push`, fmt & clippy gated for the 1st time|V26,V82,V102
T73|.|`/titrate` machinery — `.bbx-frontier` record, believability re-keyed node → SHAPE, cost ledger w/ the denominator named (`.:B4`)|V103,V60
T74|x|titrate the JUDGE upward until it FAILS — longer fn, weaker `§V`, ⊥-obvious stub. R38: a corpus that never misses locates no boundary|V103,R37,R38

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`fed::walk` hand-rolled while §C says fs walk = `itok::walk`/`itok::glob`, ⊥ reimpl. wrote it w/o reading itok's walk API — the exact belief-⊥-measurement trap V59 names, committed in the first commit that could commit it. ⊥ caught by `check`: no runner reads §C|V74. port to `itok::walk` or amend §C w/ the measured reason itok's walk ⊥ fit
B2|2026-08-01|`§R` written as RECORDS w/ `id|state|record`. FORMAT 4.1.0 §R = RESEARCH `id|topic|finding|src`. assumed from a stale local `FORMAT.md` (6 sections) while the dep shipped 4.1.0 (7). 16 violations on first `bbx check`, all real|read the CHECKER's own `SECTIONS`/`CANONICAL_WORDS`, ⊥ a vendored copy. §R now RESEARCH; closed options → `.spec-records` (R13)
B3|2026-08-01|SPEC defect, mine: V78 written as a STATIC partition — SET = impl+spec+agents, SETTING = tests+guardrails — & committed. TDD breaks it: the test IS the work ∴ `tests` ∈ set, & nothing about the file changed. facet (property of the FILE) conflated w/ profile (property of the TASK). MEASURED after: `tdd` loads 81.4% of itok, `refactor` 80.9% ∴ the axis nearly collapses for 2 of 5 profiles|V78 now names the classification only; V86 adds PROFILE as the task-dependent selection; V87 records that facet × horizontal = 6.1% where facet alone = 81.4%. found by a READER asking about TDD, ⊥ by any check — no gate reads a partition's fitness for a workflow
B4|2026-08-01|SPEC defect, mine, repeated across ~6 commit messages: claimed "95x on the binding constraint" comparing our max call to 157,071 — which is `itok`'s WHOLE-REPO tdd profile, ⊥ what a one-call prompt needs. MEASURED baseline for the same task = 2,659 tok ∴ real ratio 2.1x. compared against a straw man nobody would build|`bbx oneshot` built as the honest monolith arm; R29/R30 carry the measurement; V60 restated. GENERALLY: a ratio ! name what is in the DENOMINATOR & that thing ! be something someone would actually do
B5|2026-08-05|HEAD stopped COMPILING w/ ⊥ blackbox commit. `67fa9ad` (08-02 10:26) imported `microlith`'s inner `violation` module & the gate passed; microlith privatized it 9h later (`421ab02`, 08-02 19:16) ∴ E0603 on a tree already judged green, & already pushed. `../microlith` was a path dep ∴ no version could hold it still, & `Cargo.lock` read `0.4.0` for a sibling saying `0.5.0`|`microlith` = crates.io `0.5` + lock checksum (V101). `itok` stays a path dep & is now the ONLY one — recorded, runner-checked (T70), removed by T71. `src/spec:B1` carries the import half
B6|2026-08-18|gate ran `build` + `test` ONLY, from the first commit that had a hook — no fmt, no clippy — ∴ an entirely unformatted tree & 18 clippy findings accrued behind a verdict that read green every time. the ops lived as a shell BODY in `.githooks/pre-commit` ∴ the SET of checks was never reviewable data & nobody could see what was ⊥ there|ops → `hk.pkl`, file-scoped & readable; fmt + clippy gated & the debt paid (`abd2bb1`). V102. `-D warnings` moved off `RUSTFLAGS` so it stops reaching `../itok`
