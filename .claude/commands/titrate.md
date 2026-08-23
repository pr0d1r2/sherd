---
description: Find the granularity at which the local 20B is net-positive -- attempt, judge, and on failure either enrich the pack or split the task, recording the frontier. Halts with a recorded reason.
argument-hint: [duration, e.g. 1h or 30m -- omit for a single cycle] [--dry]
---

# /titrate $ARGUMENTS

Dose-finding on task size.

Plan, attempt, review the diff — and **when the attempt fails, retry smaller
rather than give up**, recording the granularity that finally worked.

The question it answers over many runs: how small does a task have to be cut
before the local 20B can do it, and does cutting it that small cost less than
writing it yourself.

The answer may be "it doesn't." That is a result, not a failure of the run.
`src/tdd:T13` currently reads *0 merit wins observed* across two N=3 runs — as
of this writing the model has never beaten the deterministic baseline on this
repo. A run that establishes the frontier is empty for a task class, and says
so with numbers, has done its job.

## Two limits, and only one of them is solved

sherd already descends recursively on **context**: federation splits the
spec by directory until each node's pack fits the window. `budget`, ceilings,
`lens` and `split` are all that one move, and it is deterministic.

This loop descends on **competence**: split the task until the model can
actually do it. That limit is stochastic, cannot be computed, and is discovered
only by attempting. Do not confuse a pack that is too thin with a task that is
too big — the decision rule below exists entirely to tell them apart.

## The ladder

Rungs, coarse to fine:

1. a whole `§T` row
2. one function, named by one `§V`, signature given
3. one function body, signature **and** tests given — where `sherd tdd` sits today
4. one match arm or branch inside an existing function
5. one expression or constant

**The floor is verifiability, not size.** Split below one `§V` and the piece has
no correctness criterion — "does this prove the invariant" is unanswerable about
a fragment that touches no invariant. You may split the work further; you can
then only verify the sum, never the parts. Reaching this floor with the model
still failing is a real answer: *this task cannot be made small enough to
delegate while remaining checkable.* Record it and stop descending.

## One cycle

**1. Pick a target.** `sherd plan` for the horizon. Prefer a row whose shape has
a frontier record — a repeat measurement on a known shape is worth more than a
first sample on a new one.

**2. Predict the starting rung.** Read `.sherd-frontier` for this shape. Start at
the coarsest rung whose recorded `kept/tried` is above 0.5, or rung 2 if the
shape is unrecorded. Never start at rung 1 for a shape that has failed there
twice — the point of the record is to stop re-buying knowledge.

**3. Attempt.** `sherd tdd <node> <Vn> "<task>"`, or the narrowed prompt if you
are below rung 3. Note prompt tokens and wall clock; the endpoint at
`SHERD_ENDPOINT` is one shared box and a careless sweep monopolizes it.

**4. Judge.** Three outcomes, and only the first counts as success:

- **kept** — survived *your* diff review, not merely the gate. Gates have passed
  wrong code three times in this repo, all recorded in `§B`.
- **gate red** — deterministic failure, free to observe.
- **review finding** — gate green, defect found by eye or by `sherd review`.

**5. Decide: enrich, split, or stop.** This is the cycle's whole intelligence.
Enrich before splitting — tokens are cheaper than your judgement:

| symptom | reading | move |
|---|---|---|
| compile error naming something absent from the pack | pack too thin | same rung, `--depth why` or `all` |
| output truncated, or pack near the ceiling | pack too fat | same rung, narrower facet |
| test fails against code that contradicts it | competence | split one rung |
| unused parameter, stub, detector with no positive case | purpose not understood | split one rung, sharper criterion |
| judge rejects the test 3x | the `§V` row is unclear, not the model | **halt** — fix the row |
| at the verifiability floor, still failing | frontier reached for this shape | record, stop descending |

**6. Record the shape.** Append to `.sherd-frontier` — tracked, unlike
`.sherd-state`, because learning that dies at the clone boundary is not learning:

```
shape rung=2 kind=newfn pack=b8 sig=1 tests=0 tried=4 kept=1
```

`kind` ∈ `newfn` · `edit` · `detector` · `parser` · `io`. Counts only ever
increment; never hand-edit one to make a frontier look better.

**7. Record the cost, honestly.** Every success carries what it cost to set up:
calls, prompt tokens, wall clock, **and** the decomposition work you did to make
the task delegable. Net value is *did this save work, counting the work to make
it delegable*.

`§B4` is the precedent: a 95x claim became 2.1x once the denominator named
something a person would actually do. A cycle that reports a win without its
setup cost is repeating that error with a smaller number.

**8. Commit.** Branch only. Gate green. Never `--no-verify`, never push.

## The failure mode this loop has by construction

Left unconstrained, a granularize-until-success loop **converges on trivia**. It
splits until something passes, and "passes" drifts toward "too small to be
wrong." `src/tdd:B2` is precisely this: a judge that loosened from *"does this
prove the invariant"* to *"would this compile."*

Two guards, neither optional:

- The criterion never weakens as the rung gets finer. A rung-4 task is judged
  against the same `§V` as a rung-1 task, or the descent proves nothing.
- A success below the verifiability floor is recorded as **unverified**, never
  as `kept`.

## Who splits

**Deterministic first** — one function per step, one invariant per step. Free,
structural, cannot hallucinate.

**You second** — judgement, and it costs the tokens the ledger in step 7 counts.

**The 20B never.** It just failed the task; decomposition is strictly harder
than execution, since it means holding the whole while proposing the parts.
`src/plan:B4` records this repo getting burned by looser classification than
that.

## Halting

Stop the cycle and record why:

- The same `§V` row fails at two adjacent rungs → the row is unclear, not the
  model.
- Two consecutive shapes reach the verifiability floor still failing → the
  frontier is empty here; report it rather than descending into fragments.
- `sherd check` cannot be made clean → the spec is inconsistent; guessing worsens it.
- Net value negative on three consecutive successes → the model is winning
  tasks that cost more to prepare than to write. This is the most important
  halt and the easiest to talk yourself out of.

```sh
git commit --allow-empty -m "halt(titrate): <one line>

<shape, rung reached, what the judge or gate said, and what a human
 should decide>"
```

## Summary

At the end — deadline or halt — one commit, auditable by someone who was not
watching:

```sh
git commit --allow-empty -m "titrate: <n> cycles over <duration>

frontier:  <shape> rung <n> -- kept <k>/<t>, one line each
moved:     <shapes whose recorded rung changed this run>
floor:     <shapes that reached the verifiability floor still failing>
cost:      <calls, prompt tokens, wall clock, decomposition effort>
net:       <did delegation save work -- with the denominator named>
next:      <the shape the following run should sample>"
```

`.sherd-frontier`, `sherd plan` and `git log` are the inputs. Do not estimate what
you can read.

## Flags

- `--dry` — pick the target, predict the rung, state the falsifier and the
  estimated cost, then stop. No endpoint calls, no commits. Run this first.

## Rules

- **Never `--no-verify`.** The gate refusing is the system working.
- **Never weaken the judge to make a finer rung look successful.** That is the
  one move that makes this entire loop lie.
- **Branch only.** Never `main`, never merge, never force-push.
- **A success above the verifiability floor is real work — keep it.** It was
  attempted against a `§V`, reviewed by eye, and gated. Discarding a reviewed
  diff because it arrived through a measurement run wastes the one outcome this
  loop exists to produce. Only an unverified success — below the floor, where
  no criterion applies — is recorded and thrown away.
- **Report what you examined**, not only what failed. A vacuous pass and a real
  one look identical.
