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
V5: repo root = the dir holding `.git` AND `SPEC.md`, ⊥ CWD. `bbx` is a shim over `cargo run` ∴ CWD is wherever it was typed

## §T TASKS

id|status|task|cites
T1|x|verb dispatch, usage, exit codes|V1,V3
T2|x|repo-root discovery walking up from CWD|V5
T3|.|`bbx validate` — compose DAG, id, budget & coverage checks, examined counts|`.:V48`
T4|.|`bbx init` — scaffold a `SPEC.md` w/ `§F` rows from child dirs|`.:V5`
T5|.|`bbx route "<query>"` — resolve a query to a node, exit 3 ambiguous|V2
T6|.|`bbx check` drift spec↔code|`.:V21`
T7|.|`bbx graph --json`|`.:V83`
T8|.|`bbx review` verb over the last commit|`.:V48`
