# SPEC

## §G GOAL

Move a run branch onto `main` — when the evidence says it earned it.

## §C CONSTRAINTS

- decides on EVIDENCE only. ⊥ read the diff, ⊥ judge intent — that is `review` & the human.
- git via `Command`. ⊥ a git library: 6 plumbing calls ⊥ justify a dep.

## §V INVARIANTS

V1: generated code ⊥ reach `main` w/o passing HERE. `apply` used to REFUSE on main — the right requirement stated as an obstacle ∴ it now BRANCHES instead & `land` holds the bar
V2: landing is EARNED per node — believability ≥ `LAND_MIN` (0.85 = 5 consecutive keeps under Laplace). a property of the TRACK RECORD, ⊥ of the diff: the gate has gone green on 3 stubs & the blind lens is n=10 vs failures we already had names for
V3: believability ⊥ outvote the evidence about THIS branch. a red gate | any finding refuses @ any record — a track record is permission to trust a green, ⊥ permission to skip one
V4: a branch is as trustworthy as its LEAST proven node, ⊥ its average. averaging lets 4 known-good nodes carry 1 untried one
V5: `--ff-only`. main moved → REFUSE & restore the branch. resolving a conflict inside generated code unattended is the operation nobody wants running @ 3am
V6: refusal carries the NUMBER that caused it & what would change it (`.:V24`). "⊥ believable enough" w/o the arithmetic is a verdict, ⊥ a report
V7: EVERY commit pushes to the remote when one exists — the remote runs CI on it ∴ an independent check on a machine that did ⊥ write the code. the RUN BRANCH, never `main`: pushing a branch is ⊥ publishing, & the trunk moves only through `land`. push failure = reported, ⊥ fatal — the commit already exists & losing it to a down network is worse than being unpushed
V8: ONE branch per RUN, ⊥ per `apply`. an overnight run calls `apply` ~20x ∴ per-call naming = 20 branches & `HH-MM` collides anyway. a run is the unit anyone reviews
V9: a refused branch is UNTOUCHED. it is the record of the try & deleting it destroys the evidence the next run needs
V10: the blind lens is ⊥ re-run here — `drive_from` applied it to every commit before it existed. a 2nd identical call is a 2nd READING of one rule, ⊥ a 2nd opinion (`.:V72`)

## §T TASKS

id|status|task|cites
T1|x|`stamp`, `run_branch`, `landable`, `Evidence`|V2,V6,V8
T2|x|`apply` branches instead of refusing; `bbx land` + `apply --land`|V1
T3|.|MEASURE V2: nothing has EVER landed — 0 outcomes recorded, every node @ 0.50 untried. the threshold is UNEXERCISED|V2
T4|.|`bbx runs` — list `bbx/apply-*` branches w/ why each was refused|V9
T5|.|needs a second repo to test against — `--ff-only` refusal is untested on a moved `main`|V5
T6|x|`push_branch` after every `apply` commit, remote-optional|V7
T7|.|`land` ⊥ distinguish SUPERVISOR commits from generated ones ∴ hand-written work on a run branch is blocked by a believability score earned by the LOOP. MEASURED 2026-08-02: refused my own reviewed commit @ 0.33|V2

## §B BUGS

id|date|cause|fix
