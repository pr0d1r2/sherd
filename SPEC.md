# SPEC

## §G GOAL

`blackbox` (cmd `bbx`) — two-axis federation of `SPEC.md` + Rust source over a dir DAG, so a 128k local model works at one altitude & pulls deeper only when measured cost says it must.

MOTIVATING NUMBER: `itok` = 135,096 tok (28,462 spec + 106,634 code) vs 102,529 working on the target box. An 11,291-line CLI ⊥ fit its own best-case hardware.

## §F FEDERATION

dir|owns|⊥owns|tokens
src|code nodes — tokens, spec, fed, lens facades & logic|inference harness, endpoint config|-
scripts|inference harness driving the local endpoint, premise gate|Rust code, spec format|-

## §C CONSTRAINTS

- lang: Rust. stable. MSRV 1.82 (matches `itok`/`cavespec`).
- target model: `gpt-oss:20b`, 131,072 ctx, local. ⊥ cloud fallback.
- inference: local HTTP (Ollama) only. ⊥ network otherwise.
- deterministic core: parse/DAG/budget/ceiling = pure Rust, ⊥ model. model ? prose gen & drift judgement only.
- separator = **directory**. dir tree ! source of truth. ⊥ manifest, ⊥ name-encoded grouping (`core-parse` ⊥ imply parent).
- federation edge = parent dir → child dir, depth **+1 exactly**. ⊥ skip.
- graph ! DAG. cycle ⊥. re-parent (2+ parents) OK.
- intra-file spec ops = `cavespec` lib dep (`../nanokit`, 0.4.0, zero-dep, pure fn over `&str`). ⊥ reimpl parse/fmt/check/anchors.
- token counting = `itok` lib dep (`../itok`, 0.2.0). ⊥ own tokenizer, ⊥ own bytes/4.
- fs walk + ignore globs = `itok::walk`/`itok::glob`. ⊥ reimpl.
- SPEC syntax = FORMAT **4.1.0**, sections `G C I R V T B` fixed & ordered + `§F`/`§N`. `§F`/`§N` ! land in FORMAT/cavespec upstream, ⊥ invented locally (V47).
- layout: **one crate**. module = **dir + `mod.rs`** (the `default.nix` shape — dir is the unit, entry is conventional). ⊥ 2018 `foo.rs`+`foo/`: that puts the facade OUTSIDE the dir it fronts ∴ module entry & its `SPEC.md` land in different federation nodes.
- node = dir = Rust module. all three aligned or a flat `.rs` owns spec it cannot hold.
- caveman encoding ∀ generated spec text.
- ⊥ global index file. discovery by walk.

## §I INTERFACES

- cmd: `bbx init [dir]` → scaffold `SPEC.md` @ dir, `§F` rows from child dirs
- cmd: `bbx lens <dir> [--depth rule|why|all]` → context pack. default `rule`
- cmd: `bbx lens <dir> --json` → `{chain:[],body:{},children:[],tokens:{},examined:{}}`
- cmd: `bbx route "<query>"` → dir + reason. 0 hit / 2 miss / 3 ambiguous
- cmd: `bbx check [dir]` → drift spec↔code + file ceilings. 0 clean / 1 violation / 2 usage
- cmd: `bbx split <path>` → propose split of over-ceiling file|node. ⊥ write w/o `--apply`
- cmd: `bbx sync [dir]` → regen `§N` from parent `§F`. exit 1 if wrote
- cmd: `bbx graph [--dot|--json]` → federation DAG
- cmd: `bbx budget [dir]` → node/chain/lens/file token table. exit 1 over
- cmd: `bbx validate` → DAG + ids + budget + coverage + examined-count. exit 1 fail
- file: `SPEC.md` ∀ dir any depth. `§G §C §I §R §V §T §B` + `§F` + `§N`
- file: `§F FEDERATION` pipe table `dir|owns|⊥owns|tokens` — child dir depth +1. `⊥owns` = what it does NOT own + where that lives
- file: `§N NAV` pipe table `rel|path|lens`, `rel` ∈ `up`|`self`|`sib`. generated, ⊥ hand-edit
- file: `SPEC.why.md` ∀ dir w/ rationale — `<id>|<rationale>`, addressed by `§V`/`§B` id
- file: `.context-limits` — per-path ceilings, `itok` format, reused ⊥ reinvented
- file: `.spec-records` — closed-option baseline, `cavespec --records`
- env: `BBX_MODEL` (`gpt-oss:20b`), `BBX_ENDPOINT` (`http://localhost:11434`)
- lib: `cavespec::check_spec(&text,&records)`, `cavespec::fmt`, `::anchors`
- lib: `itok::estimate`, `itok::walk`, `itok::glob`
- violation: `<file>:<line>: bbx/<Vn>: <msg>` + `why` + `mechanical`|`judgment`. `--format json` carries `kind` as data

## §R RESEARCH

id|topic|finding|src
R1|crates.io name|`bbx` taken (BBCode parser, lib-only, `bin_names: []`). BIN name `bbx` free on brew, debian, PATH ∴ package≠bin|crates.io API, formulae.brew.sh, sources.debian.org
R2|cargo workspace|`members=["crates/*"]` ERRORS on a hub dir w/o `Cargo.toml`. per-depth globs + cross-nested path deps build clean|cargo 1.96.1, tested
R3|rust module form|`mod.rs` = the `default.nix` shape. private sibling → `error[E0603]` ∴ COMPILER enforces the facade, ⊥ grep|cargo 1.96.1, tested
R4|itok size|`SPEC.md` 28,462 tok + code 106,634 = 135,096 ∴ 103% of a 128k window. an 11,291-line CLI ⊥ fit|itok 0.2.0 `--bpe`
R5|spec fatness|itok 656B/rule × 103 rules. rationale = 80% of §V (cavespec 73%) ∴ per-row FATNESS is the multiplier, ⊥ rule count|measured over both §V
R6|federation yield|dir federation moves 41% of itok §V bytes; intermediate hubs absorb 9.5%; bbx own §V 64% stays root|classified 103 + 55 rules
R7|inline tests|42% of itok+cavespec `src/` is `#[cfg(test)]`; `tracecmd.rs` 65% ∴ whole-file ceiling ranks a SMALL module worst|measured per file
R8|tokenizer drift|bytes/4 measured 48% LOW on caveman-encoded `SPEC.md` (5,761 real vs 3,890 est) ∴ ⊥ ever a gate|itok `--bpe` vs bytes/4
R9|gpt-oss:20b arch|24 layers, 8 KV heads, k/v len 64, sliding_window 128, ctx 131,072, 32 experts/4 used, MXFP4, 20.9B|ollama `/api/show` @ 192.168.0.181
R10|target box|24GB M5 Pro: ctx 131,072 ALLOCATED, 11.98G of 24G resident, 100% GPU, ⊥ CPU spill, ollama 0.32.3|ollama `/api/ps`, measured
R11|KV cost|KV/tok = 2 × layers × kv_heads × head_dim × bytes. sliding-window halves it (12 of 24 layers full) ∴ 24GB needs NO KV quant|derived from R9 + R10
R12|32k models|any 32k-ctx model leaves 4,225 working after 28,543 entry cost ∴ unusable for SDD at EVERY hw tier, incl M64|derived from R9/R11
R13|cavespec §R|FORMAT 4.1.0 §R = RESEARCH (`id|topic|finding|src`), ⊥ records. closed options live in `.spec-records`|cavespec 0.4.0 `check.rs`, FORMAT.md

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
V43: `§V`/`§B` statement = rule + rationale. rule inline @ `SPEC.md`, rationale @ `SPEC.why.md` keyed by id. MEASURED: rationale = 80% of `itok` §V, 73% of `cavespec` ∴ vertical buys 5x vs horizontal 1.7x
V44: vertical split lossless **by reference ⊥ by deletion**. ∀ id ∈ `SPEC.md` → row ∈ `SPEC.why.md` | explicit `-`. rationale is where closed-option records live ∴ dropping it is the failure both sibling repos already guard
V45: `lens --depth rule` default. `why` pulled on demand, ⊥ resident. entry cost is re-billed EVERY turn
V46: budget sized against MEASURED entry cost, ⊥ raw window. measured: harness overhead ~28,543 tok before any file ∴ `budget.lens` + entry ≤ 40% of 131,072
— guards learned from `itok`/`cavespec` §B —
V47: `§F`/`§N` are FORMAT extensions. cavespec `check` fixes section set `G C I R V T B` ∴ unknown section ! be negotiated upstream. ⊥ ship a dialect cavespec cannot read
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
V60: premise ! be measured before it is built on. federation's claim = a local model completes from a `lens` pack a task it fails from the monolith. unmeasured → `bbx` is its own unmeasured optimization
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
V74: §C claims ! have a runner. `fed::walk` contradicted §C for a whole session & no gate could see it (B1) — a constraint no check reads is a comment
V75: format facts read from the CHECKER's own source, ⊥ a vendored `FORMAT.md`. the local copy was 6 sections while the dep shipped 7 (B2)
V73: dir promotion has 2 triggers — (a) V50 code ceiling, (b) module owns SPEC worth its own node even under ceiling. vendor facades are (b): few hundred lines carrying V17/V24/V25. ⊥ promote every `.rs` — 30 files → 60 is ceremony

## §T TASKS

id|status|task|cites
T1|x|scaffold single crate `bbx`, module=dir+`mod.rs`, explicit `[[bin]]`, deps `itok`+`cavespec` by path|C,R1,R3
T2|x|bind `cavespec` — `check`, `fmt`, section split. ⊥ reimpl|C
T3|.|capability-parity audit `cavespec` vs what `bbx` needs. write the comparison BEFORE relying on it|V59
T4|x|parse `§F` table → (dir, owns, ⊥owns, tokens), escape-aware|I,V1
T5|~|fs walk — own impl landed, VIOLATES §C (`itok::walk` ⊥ reimpl). see B1|V23,B1
T6|.|build federation DAG, depth+1, cycle detect|V1,V2,V4
T7|.|orphan check: SPEC w/o parent `§F` row|V3
T8|.|bind `itok::estimate`, tier floor `bpe`, method label|V17,V24
T9|.|`.context-limits` reuse for node + file ceilings|I,V52
T10|.|`bbx budget` + over-budget exit 1|V6,V7,V8
T11|.|split hint when node over budget|V9
T12|.|id namespacing + resolver `path:Vn`|V10,V11
T14|x|`bbx lens` render pack, `--depth rule` default|I,V15,V45
T16|.|`bbx init` scaffold from child dirs|I
T17|.|ollama client `BBX_MODEL`/`BBX_ENDPOINT`|I,V25
T18|.|`bbx route` one-edge-per-step|V19
T19|.|route ambiguous exit 3 / miss exit 2|V20
T20|.|`--offline` model-free path|V18
T21|.|`bbx check` drift spec↔code|I
T22|.|`§F`.tokens staleness, tier-tagged|V21,V26
T23|.|coverage gap warn for un-specced source dirs|V16
T24|.|`bbx graph --dot/--json`|I
T25|.|`bbx validate` incl examined-vs-discovered counts|I,V48
T27|.|invariant promotion helper leaf→ancestor|V13,V14
T28|.|backprop: bug → leaf `§B`, decide promote|V14
T29|.|synthetic fixture: 4 deep, shared module 2 parents (self-repo is a tree ∴ ⊥ cover DAG)|V4
T31|.|ollama tier probe + fallback warn|V25
T35|.|parse `§N`, line-anchored emit|V34
T36|.|derive `§N` from parent `§F`, multi-parent union|V36,V38,V39
T37|.|`bbx sync` + exit 1 when wrote|I,V36
T38|.|`check`: `§N` ≠ derived → drift|V36
T40|.|vertical axis: `SPEC.why.md` format + id keying|V42,V43
T41|.|`lens --depth rule\|why\|all`|V45
T42|.|cross-file losslessness proof, asserted pre-write|V49,V44
T43|.|`.rs` ceiling: code vs test counted separately|V50,R7
T44|.|`mod.rs`/`lib.rs` tighter ceiling|V51
T45|.|`bbx split <path>` proposal, `--apply` gated, kind `judgment`|I,V53,V54
T46|.|coupling report after proposed split|V53
T47|.|violation renderer `file:line: bbx/Vn:` + why + mechanical\|judgment + json `kind`|V55,V54
T48|.|sibling-divergence detector for V13|V62
T49|.|`§F`/`§N` upstream to FORMAT/cavespec before shipping a dialect|V47
T50|.|corpus run over 54-spec fleet, FP rate reported|V58
T51|.|`.spec-records` baseline + survival check across split|V44
T52|.|planted-violation test ∀ guard + accepts-real-shapes companion|V61
T53|.|PREMISE GATE: `gpt-oss:20b` completes from `lens` pack a task it fails from monolith. `itok` SPEC.md = pressure fixture|V60,V27
T54|.|self-federate: root `§F`+`§N` + own nodes, once T1-T8 land|V27,V30
T55|.|CI: `bbx validate` self exit 0; globs by data dependency|V27,V57
T56|.|`§F` gains `⊥owns` column — parse, emit, `init` scaffold|I,V66
T57|.|sibling lens exhaustive + disjoint check, overlap → exit 1|V64,V65
T58|.|`cap.row` gate + `SPEC.why.md` reference resolution|V69,V70
T59|.|PAY THE DEBT: record rationale for V1-V63 into `SPEC.why.md` before it accretes inline. rationale currently lives only in the design conversation|V69,V70,V44
T60|.|`src/tokens/mod.rs` sole `itok::` call site; `src/spec/mod.rs` sole `cavespec::`. siblings private|V71,V72
T61|.|facade-leak test: planted `itok::` outside `src/tokens/` ! fail to compile|V72,V61
T62|.|dir-promotion check: flat `.rs` owning node-local invariants → promote|V73,V50

## §B BUGS

id|date|cause|fix
B1|2026-08-01|`fed::walk` hand-rolled while §C says fs walk = `itok::walk`/`itok::glob`, ⊥ reimpl. wrote it w/o reading itok's walk API — the exact belief-⊥-measurement trap V59 names, committed in the first commit that could commit it. ⊥ caught by `check`: no runner reads §C|V74. port to `itok::walk` or amend §C w/ the measured reason itok's walk ⊥ fit
B2|2026-08-01|`§R` written as RECORDS w/ `id|state|record`. FORMAT 4.1.0 §R = RESEARCH `id|topic|finding|src`. assumed from a stale local `FORMAT.md` (6 sections) while the dep shipped 4.1.0 (7). 16 violations on first `bbx check`, all real|read the CHECKER's own `SECTIONS`/`CANONICAL_WORDS`, ⊥ a vendored copy. §R now RESEARCH; closed options → `.spec-records` (R13)
