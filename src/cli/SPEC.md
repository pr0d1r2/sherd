# SPEC

## §G GOAL

Arg dispatch and exit codes. `main.rs` is a shim over `run`.

## §N NAV

rel|path|lens
up|.|-
up|src|code nodes — tokens, spec, fed, lens facades & logic
self|src/cli|arg dispatch, usage, exit codes
sib|src/tokens|`itok` facade, counts w/ method label, entry cost, working budget
sib|src/spec|`microlith` facade, §-section split, structural check, fmt
sib|src/fed|`§F` parse, edges, chain root→node, `SPEC.md` discovery
sib|src/lens|pack assembly, depth `rule`|`why`, budget verdict
sib|src/ollama|local endpoint client, `num_ctx`, fence extraction
sib|src/tdd|red→judge→green→gate→repair loop, source region edits
sib|src/plan|open `§T` rows, horizon, confidence, `apply` one step
sib|src/review|mechanical checks on what `apply` committed
sib|src/state|one idempotent cached store — pace, telemetry, applied rows
sib|src/slice|distil a document to the part needed to ACT, generated
sib|src/land|run branch → `main` when believability earns it
sib|src/code|read Rust source as text — split, public fns, call detection, signatures
sib|src/debt|a ratchet — measure, compare to a recorded floor, refuse the wrong way
sib|src/assay|a corpus + a compiler grader — measure WHETHER the model can, ⊥ make it

## §C CONSTRAINTS

- dispatch only. logic lives in the node that owns it (`src/cli:V14`).
- exit codes are a CONTRACT — scripts read them.

## §V INVARIANTS

V1: exit 0 clean · 1 violation · 2 usage. ∀ verb obeys it
V2: a MISS is exit 2 naming what was tried, ⊥ exit 0 empty (`.:V20`)
V3: usage names commands that EXIST. inventing a verb in help or a message gives one operation two names (`.:plan` B3)
V4: `-v`/`--verbose` is a MODE, positional-agnostic ⊥ an argument
V5: repo root = the dir holding `.git` AND `SPEC.md`, ⊥ CWD. `sherd` is a shim over `cargo run` ∴ CWD is wherever it was typed
V6: a test reaches only the repo it was HANDED. V5 walks up ∴ a test naming no fixture finds whatever tree the runner sits in, & the crate root IS one — green about its LOCATION, ⊥ the code (B1)
V7: usage names EVERY verb that dispatches, ⊥ merely only verbs that exist (V3's converse, B2). a reachable verb missing from usage is undiscoverable @ the one place a user looks
V12: ABSENCE is ⊥ a finding. a check over an OPTIONAL artefact reports "none required" & counts 0 when it is missing, & fails only when it exists & cannot be read. B3 is the 2 halves conflated: a fresh `init` tree failed `validate` ∵ it had no slice registry to drift from
V14: the binary is a SHIM — `main` holds nothing & dispatch carries no logic; every verb calls the node that owns the work. cited from 4 files as a root `V41` that was never written (`src/spec:B2`)
V13: a GENERATOR is tested by running it TWICE. once proves it writes; the 2nd run is what proves it wrote the SAME thing, & `sync`·`slice`·`sherd-dev readme` are all gates whose `--check` half is meaningless if the fix half is ⊥ a fixed point (B4)

## §T TASKS

id|status|task|cites
T3|.|`sherd validate` — compose DAG, id, budget & coverage checks, examined counts|`.:V48`
T4|.|`sherd init` — scaffold a `SPEC.md` w/ `§F` rows from child dirs|`.:V5`
T5|.|`sherd route "<query>"` — resolve a query to a node, exit 3 ambiguous|V2
T6|.|`sherd check` drift spec↔code|`.:V21`
T7|.|`sherd graph --json`|`.:V83`
T8|.|`sherd review` verb over the last commit|`.:V48`

## §B BUGS

id|date|cause|fix
B1|2026-08-22|2 tests pass ONLY ∵ the runner sits in this checkout: V5's walk finds THIS repo when handed none. MEASURED proving T71's dep swap — `git archive HEAD` to a non-repo dir, same tree: 280 pass, `repo_root_finds…` & `review_of_a_real_revision…` fail ∴ the suite in the crate TARBALL is red & a nix sandbox build cannot run it, which is why `packages.default` stayed absent|V6. T9 hands both a fixture & asserts the cwd is ⊥ a repo. GENERALLY: a test that DISCOVERS its input is green about WHERE it ran
B2|2026-08-23|`oneshot` DISPATCHES & is absent from usage. V3 binds one direction — usage ! name only verbs that EXIST — & the converse went unwritten ∴ a verb reachable, documented in `.:README` & measured in `.:R30` was invisible to `sherd` w/ no args, the one place a user looks. found by reading the README against the binary while writing `dev`, ⊥ by any check|V7 states the converse & `sherd-dev readme` GENERATES the list from dispatch (`.:V115`) ∴ neither direction can drift again. GENERALLY: a rule written as one implication leaves the other half unguarded, & the unguarded half is where the defect goes
T10|x|`sherd validate` — 1 verdict over structural + edges + ceilings + slice drift, COMPOSED from the owners (`.:V72`), & REPORTS what it examined ⊥ only what failed|V1,`.:V48`
T11|x|`sherd route <query>` — exit codes ARE the answer: 0 one node · 2 none · 3 several. rounding ambiguous to its 1st match makes the interesting case identical to the certain one|V1,`.:src/plan:V15`
B3|2026-08-23|`validate` counted a MISSING `.sherd-slices` as 1 drift ∴ a repo `sherd init` had just scaffolded FAILED `sherd validate` — the 2 verbs of rung 0.2 & 0.3 contradicting each other on a fresh tree. absence is LEGAL & was read as a finding; the error path counted 1 w/o asking WHICH error|V12. a registry that cannot be READ stays a failure; one that does ⊥ exist reports "none required" & counts 0. GENERALLY: an error branch that counts a failure ! distinguish "could ⊥ look" from "⊥ there"
B4|2026-08-23|`sync` was ⊥ IDEMPOTENT: `upsert_section` spliced strings & appended 1 newline per run ∴ every §N grew by a blank line & `sync` reported "rewritten" forever. a 2nd defect underneath: the anchor (`§F`) precedes the section (`§N`) in a well-formed doc ∴ the insert branch fired BEFORE the replace branch & a 2nd `§N` was appended each run. caught by a test asserting the 2nd run is a no-op, ⊥ by running it once|V13. rebuilt on the SECTION LIST — parse, replace-or-insert, re-render w/ exactly 1 blank line between sections ∴ idempotence holds BY CONSTRUCTION. replace is checked BEFORE insert. GENERALLY: a generator ! be tested by running it TWICE; once proves only that it writes
B5|2026-08-24|a `[dir]` naming ANOTHER repository resolved its root from the CWD ∴ `sherd check /tmp/ashlar` reported THIS crate's 18 nodes & 29 `.rs` files & exited 0 — confidently, about the wrong tree, w/ nothing in the output saying so. `budget` had the same cause & a different symptom: it filters `discover(root)` by the given dir ∴ 0 nodes examined, exit 2, & the message blamed the DIR for naming no node when the truth is nobody looked there. `split` survived ∵ it passes the dir straight through. FOUND by dogfooding a 4th foreign repo (`ashlar`, 24 `.rs`, 1 unfederated `SPEC.md`) — the 4th stranger, the 4th distinct defect, & ⊥ by any test|V5. `root_for(args)` — when the 1st argument names an EXISTING DIRECTORY the root is the one ABOVE IT; a non-dir 1st arg (`HEAD` for `review`, `--check` for `sync`) leaves the CWD walk alone, & an in-repo path resolves to the root it always did. 1 change in dispatch fixes every `[dir]` verb at once ∵ they all read the same `root`. GENERALLY: a tool that accepts a PATH accepts a tree it was ⊥ launched from, & every ambient fact it resolves — root, ceilings, config — is then a guess about the wrong one
B6|2026-08-24|2 verbs read an EMPTY answer as a good one, both found on the 4th foreign repo. `fed` on an unfederated `SPEC.md` printed NOTHING & exited 0 ∴ silence reads as "no problems" rather than "no `§F` table" — `src/fed:B6`'s shape (something that finds nothing passes cleanly) met from the other side. `slice --list` w/ no `.sherd-slices` died w/ a bare `No such file or directory (os error 2)`: no path, no hint that the registry is OPTIONAL, & exit 2 — while `validate` has printed "slice: no registry (none required)" for that SAME condition all along ∴ 2 readings of 1 absence, & `V12` was written for exactly this & applied to only 1 of them|V12,`.:V48`. `fed` states what it examined & points at `split`; `slice_cmd` checks `is_file()` first & reports "none required", exit 0, & a real read error now names the PATH. GENERALLY: every verb ! be run against a repo that has NONE of the thing it looks for — our own tree has all of them, so absence is the case only a stranger tests
B7|2026-08-24|`B5`'s fix was INCOMPLETE & the same stranger showed it 1 command later: `root_for` read `args[1]` only ∴ `sherd plan --triage <dir>` — a FLAG at position 1 — found no dir & answered about the CWD, printing THIS crate's `src/plan`·`src/spec`·`src/tdd` rows for a foreign repo, while plain `plan <dir>` had been correct all along. `sync --check <dir>` & `split --apply <dir>` share the shape. ALSO `debt` conflated an ABSENT `.lint-debt` w/ a MALFORMED one — "no `density`/`shape` rows to read" for a file that does ⊥ exist, `B6` again 1 verb over|V5. scan EVERY argument for the 1st existing directory, ⊥ position 1. GENERALLY: a fix expressed as a POSITION is a fix for the invocations someone thought of — `B5` was found & fixed & re-broken inside 1 session ∵ the 1st form tried put the dir where the code looked. the test now pins 4 orderings, ⊥ 1
