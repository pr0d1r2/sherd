---
description: Sit with the local 20B while it works -- plan, apply, review the diff, plant anchors, commit -- at one fixed granularity. Halts with a recorded reason. Ends with a summary commit.
argument-hint: [duration, e.g. 1h or 30m -- omit for a single cycle]
---

# /sit $ARGUMENTS

A local 20B writes code; **you** do the judgement it cannot. You are the
sitter: sober, present, not doing the work, intervening only when it goes
wrong — and the one who remembers afterwards what happened.

Everything here runs at **one fixed granularity** — a `§T` row becomes one
function in one node's `mod.rs`. Nothing varies. That is the point: when an
attempt fails here, something external moved, and the failure is a signal
rather than an experiment. `/titrate` is the loop that varies granularity on
purpose; this one exploits what that loop already found.

Designed to run with **nobody watching**. So: every halt is recorded as a
commit, never only printed, and nothing is ever merged or pushed to `main`.

## Duration

`$ARGUMENTS` is a floor, not a ceiling — `1h`, `30m`, `90m`. Omitted means one
cycle.

Fix the deadline once, at the start, and never recompute it:

```sh
START=$(date +%s); DEADLINE=$((START + 3600))   # for 1h
```

Then, **before starting each cycle**, check `date +%s` against it.

- Before the deadline → start another cycle.
- At or past it → stop and write the summary.
- **Never abandon a cycle in flight.** A run killed between `apply` and its
  review leaves generated code uncommitted in the tree, which is how a
  previous run left `scopeguard::guard` sitting in `src/fed`. Finish the
  cycle you started, then check the clock.

A cycle has run 10–25 minutes in practice, so `1h` is 3–5 cycles, not 30.
Halt conditions override the clock: a halt stops the run with time remaining.

## Why you review at all

`sherd apply` is gated by `cargo test` + `sherd check`. Those gates have **passed
wrong code three times**, all in §B:

- `check_edge_depth` filtered on `not_owns` — the prose column — as a path.
  Test and implementation agreed with each other and neither matched the
  invariant.
- `sanitize_first_cell` stripped `/` out of `dir` at parse time so the
  violation would vanish. Data laundering to turn a gate green.
- `detect_cycles` returned `Vec::new()` under a doc comment reading *"stub …
  satisfies the current test suite"*. `apply` committed it unattended.

A green gate is not correctness. If you skip the diff, this loop writes code
that satisfies gates instead of invariants and documents itself doing it.

## One cycle

**1. `sherd plan`.** Read the horizon. Note `UNMANAGED` — it is scope, not noise.

**2. `sherd apply`.** One step, then it stops. A refusal is usually right; read
it rather than routing around it.

**3. `git show HEAD` — review.** The job. In order:

- **Uses its inputs?** An unused parameter is how a stub announces itself.
- **Positive case?** A detector tested only on "nothing found" is satisfied by
  a function that always finds nothing.
- **Right quantity?** Check field names against the data model. A test on the
  wrong field proves nothing and looks fine.
- **Composes or duplicates?** `grep -c` the helper it should have called.
- **Replaces or merely adds?** A helper landed but never wired in leaves the
  duplication the task existed to remove.
- **Satisfies the §V, or the test?** Those differ. The §V wins.

**4. Judge — keep · fix · revert.** Say which and why. Fixing it yourself is
usually cheaper than another cycle. `git revert HEAD` when wrong: a wrong
function is worse than none, because it reads as coverage.

**5. Plant anchors.** Only you can do this. Into the node that owns it:

- **§B** — every defect, cause and fix. Crossing nodes → common ancestor.
- **§V** — a new invariant when it would catch recurrence. Prefer over a bug
  row alone.
- **§T** — remaining work as remaining work, **never history**. "`X` landed,
  `Y` still open" makes a machine test `X`, which already passes.

Cross-node citations need the namespaced form (`` `src/fed:V9` ``) or microlith
reads them as dangling. `sherd check` must be clean before you commit.

**6. Commit** the reasoning, not just the change. Git is the memory: what was
tried, what it cost, what it taught.

## Halting

On any of these, **stop the cycle and record why**:

- `sherd plan` shows nothing actionable → **maintenance mode**, below.
- The same §T row fails twice → the row is wrong, not the model.
- `sherd check` cannot be made clean → the spec is inconsistent; guessing worsens it.
- Two aborts at 10x → report the telemetry. Never raise a ceiling to pass.
- Two reverts in a row → the loop is producing worse than nothing.

Record it where a human will find it later, because nobody is watching now:

```sh
git commit --allow-empty -m "halt(sit): <one line>

<what was attempted, what the gate or review said, what you tried,
 and what a human should decide>"
```

Then stop. Do not start another cycle.

## Summary

At the end -- deadline reached or halted -- write one commit so the run is
auditable by someone who was not watching:

```sh
git commit --allow-empty -m "sit: <n> cycles over <duration>

applied:   <node> <id> -- kept | fixed | reverted, one line each
anchors:   <§B/§V/§T rows planted, and where>
halted:    <condition, or 'deadline'>
cost:      <cycles, local calls, tokens sent>
next:      <what the following run should start on>"
```

`sherd plan` and `git log --oneline` are the inputs; do not estimate what you
can read.

## Maintenance mode

"Nothing actionable" is not "done" — most rows are root-level and no loop will
ever drive them. Work in this order:

1. **§B rows with no §V** — a bug with no invariant recurs.
2. **Claims with no runner** — a §C or §V nothing checks is a comment.
3. **Duplication** — two readings of one rule is the founding defect here.
4. **Unmanaged rows** — most need a `needs` column or a machine-actionable
   marker in §T; some need splitting to fit one node.
5. **Budgets** — `sherd budget`. Over ceiling wants splitting, not a raise.
6. **Stale §R** — every row carries its source. Re-run the cheap ones; an
   unchecked number becomes folklore.

## Rules

- **Never `--no-verify`.** The gate refusing is the system working.
- **Never raise a ceiling, edit a test, or weaken a judge to make something
  pass.** Loosening the judge is how the stub got in.
- **Branch only.** Never `main`, never merge, never force-push.
- **Report what you examined**, not only what failed.
- A cycle is 3–7 local calls plus your review. Under `/loop`, report the
  running total.
