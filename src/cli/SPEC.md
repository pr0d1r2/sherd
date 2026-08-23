# SPEC

## §G GOAL

Arg dispatch and exit codes. `main.rs` is a shim over `run`.

## §C CONSTRAINTS

- dispatch only. logic lives in the node that owns it (`.:V41`).
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

## §T TASKS

id|status|task|cites
T1|x|verb dispatch, usage, exit codes|V1,V3
T2|x|repo-root discovery walking up from CWD|V5
T3|.|`sherd validate` — compose DAG, id, budget & coverage checks, examined counts|`.:V48`
T4|.|`sherd init` — scaffold a `SPEC.md` w/ `§F` rows from child dirs|`.:V5`
T5|.|`sherd route "<query>"` — resolve a query to a node, exit 3 ambiguous|V2
T6|.|`sherd check` drift spec↔code|`.:V21`
T7|.|`sherd graph --json`|`.:V83`
T8|.|`sherd review` verb over the last commit|`.:V48`
T9|x|hand `repo_root_finds…` & `review_of_a_real_revision…` a FIXTURE repo, + a test asserting the cwd is ⊥ inside one. then the flake package w/ `doCheck` ON|V6,B1

## §B BUGS

id|date|cause|fix
B1|2026-08-22|2 tests pass ONLY ∵ the runner sits in this checkout: V5's walk finds THIS repo when handed none. MEASURED proving T71's dep swap — `git archive HEAD` to a non-repo dir, same tree: 280 pass, `repo_root_finds…` & `review_of_a_real_revision…` fail ∴ the suite in the crate TARBALL is red & a nix sandbox build cannot run it, which is why `packages.default` stayed absent|V6. T9 hands both a fixture & asserts the cwd is ⊥ a repo. GENERALLY: a test that DISCOVERS its input is green about WHERE it ran
B2|2026-08-23|`oneshot` DISPATCHES & is absent from usage. V3 binds one direction — usage ! name only verbs that EXIST — & the converse went unwritten ∴ a verb reachable, documented in `.:README` & measured in `.:R30` was invisible to `sherd` w/ no args, the one place a user looks. found by reading the README against the binary while writing `dev:T2`, ⊥ by any check|V7 states the converse & `dev:T2` GENERATES the list from dispatch ∴ neither direction can drift again. GENERALLY: a rule written as one implication leaves the other half unguarded, & the unguarded half is where the defect goes
T10|x|`sherd validate` — 1 verdict over structural + edges + ceilings + slice drift, COMPOSED from the owners (`.:V72`), & REPORTS what it examined ⊥ only what failed|V1,`.:V48`
T11|x|`sherd route <query>` — exit codes ARE the answer: 0 one node · 2 none · 3 several. rounding ambiguous to its 1st match makes the interesting case identical to the certain one|V1,`.:plan:V15`
B3|2026-08-23|`validate` counted a MISSING `.sherd-slices` as 1 drift ∴ a repo `sherd init` had just scaffolded FAILED `sherd validate` — the 2 verbs of rung 0.2 & 0.3 contradicting each other on a fresh tree. absence is LEGAL & was read as a finding; the error path counted 1 w/o asking WHICH error|V12. a registry that cannot be READ stays a failure; one that does ⊥ exist reports "none required" & counts 0. GENERALLY: an error branch that counts a failure ! distinguish "could ⊥ look" from "⊥ there"
