---
description: Loop bbx apply under oversight; review each result, plant anchors, switch to maintenance when nothing actionable remains.
---

# /bbx:introspect

Drive `bbx` in a loop. A local 20B model writes the code; **you** do the
judgement it cannot. Those are different jobs and the split is the whole point.

## Why you are in this loop at all

`bbx apply` is gated by `cargo test` + `bbx check`, and those gates have
**passed on wrong code three times**, all recorded in §B:

- `check_edge_depth` filtered on `not_owns` — the prose column — as though it
  were a path. Test and implementation agreed with each other; neither related
  to the invariant.
- `sanitize_first_cell` stripped `/` out of `dir` at parse time so the
  violation would disappear. Data laundering to make a gate green.
- `detect_cycles` returned `Vec::new()` with a doc comment saying *"stub …
  satisfies the current test suite"*, and `apply` committed it unattended.

A green gate is not correctness. You are the part that reads the diff.

## Loop

Repeat until a stop condition fires.

**1. Plan.** `bbx plan`. Read the horizon — three steps, each with what
invalidates it. Note the `UNMANAGED` count; it is not noise.

**2. Apply.** `bbx apply`. One step, then it stops. If it refuses, the refusal
is usually correct — read it rather than working around it.

**3. Review the diff.** `git show HEAD`. This is the job. Check, in order:

- **Does it use its inputs?** An unused parameter is how a stub announces
  itself. The gate now denies warnings, but read anyway.
- **Positive case?** A detector tested only on "nothing found" is satisfied by
  a function that always finds nothing.
- **Right quantity?** Check field names against the data model. A test
  asserting on the wrong field proves nothing and looks fine.
- **Compose or duplicate?** Did it call the existing function, or re-implement
  it alongside? `grep -c` the shared helper.
- **Replace or merely add?** Landing a helper without wiring it in leaves the
  duplication the task existed to remove.
- **Does it do what the §V says**, or what the test says? Those differ.

**4. Judge.** Three outcomes, and say which:

- **Keep.** Move on.
- **Fix.** Amend it yourself. Faster than another loop iteration and you can
  see the whole picture.
- **Revert.** `git revert HEAD`. A wrong function is worse than none: it reads
  as coverage.

**5. Plant anchors.** This is the part only you can do. From what you just
read, write into the *right* node's spec:

- **§B** — every defect found, with its cause and the fix, at the node that
  owns it. A defect crossing nodes goes to the common ancestor.
- **§V** — a new invariant when one would catch recurrence. Prefer this to a
  bug row alone.
- **§T** — remaining work as **remaining work**, never history. "`X` landed,
  `Y` still open" reads to a machine as "test `X`", and `X` already passes.

Then `bbx check` must be clean before you commit. Cross-node citations need
the namespaced form (`` `src/fed:V9` ``) or cavespec reads them as dangling.

**6. Commit** with the reasoning, not just the change. Git is the memory here:
what was tried, what it cost, what it taught. Then loop.

## Stop conditions

Stop and report. Do not push through.

- `bbx plan` shows **no actionable rows** → go to maintenance mode below.
- The same row fails **twice** → the row is wrong, not the model. Rewrite the
  §T text or split it, and say so.
- `bbx check` cannot be made clean → stop. The spec is inconsistent and
  guessing makes it worse.
- The endpoint aborts at 10x twice → the estimate or the endpoint is wrong.
  Report the telemetry; do not raise the ceiling to make it pass.
- You have reverted **twice in a row** → the loop is producing worse than
  nothing. Stop and say why.

## Maintenance mode

When nothing is actionable, the work changes shape. In rough priority:

1. **Open §B rows with no §V** — a bug with no invariant will recur.
2. **Invariants with no runner** — a §C or §V claim nothing checks is a
   comment. Find them and give them a test or a gate.
3. **Duplication** — two readings of one rule is the defect this lineage
   exists to end. `grep` for it deliberately.
4. **Unmanaged rows** — the ones `plan` lists but cannot drive. Most need a
   `needs` column or a machine-actionable marker in §T; some need splitting
   into something a single node can hold.
5. **Budgets** — `bbx budget` against the target tier. A node over its ceiling
   wants splitting, not a raised ceiling.
6. **Measurements that have gone stale** — §R rows carry their source. Re-run
   the cheap ones; a number nobody re-checks becomes folklore.

## Rules

- **Never `--no-verify`.** The gate refusing is the system working.
- **Never raise a ceiling to make something pass.** That trains the reflex the
  ceiling exists to catch.
- **Never edit a test to make an implementation pass.** Fix the implementation
  or revert it.
- **Branch, never `main`.** `apply` enforces this; do not work around it.
- **Report what you examined**, not only what failed. A silent pass and a
  vacuous pass look identical.
- One iteration is 3–7 local calls plus your review. If you are running many,
  say so and report the aggregate cost.
