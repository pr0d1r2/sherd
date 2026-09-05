# Built by an LLM, deliberately and in the open

This repository — code, spec, tests and prose — was written by
[Claude Code](https://claude.com/claude-code) running Anthropic's **Claude
Opus 5**. Most commits carry a `Co-Authored-By: Claude Opus 5` trailer; the
current ratio is whatever these two commands say, which is the point of not
writing it down here:

```sh
git log --format=%B | grep -c 'Co-Authored-By: Claude'
git rev-list --count HEAD
```

A human owns every decision, reviews every diff, and is accountable for what
ships.

That is the disclaimer. The rest of this file is why it is stated as a design
note rather than as an apology, and what a reader can check for themselves.

## Why say it at all

A model writes plausible code, and plausible is not correct. A reader who does
not know how a repository was produced cannot calibrate how hard to look at it.
Saying so is the minimum, and for this project it is not close to enough —
because the claim `sherd` makes is *about* models writing software.

A tool arguing that a 20B running on your own hardware can do real work cannot
be evaluated on its output alone. You have to know which parts a frontier model
wrote, which parts the small one wrote, and what happened to each. That is why
the next section exists and why it is second rather than buried.

## Two of the functions here were written by the 20B this project is about

`src/fed/` contains work authored by **gpt-oss:20b** through `sherd tdd`, kept
with its defects recorded in that node's `§B` rather than smoothed over —
because a tool that claims small models can build software has to show what
happens when one does.

Smoothing those over would have been easy and would have made the repository
worse as evidence. The defects are the data.

## The method is spec-driven development, federated

[`SPEC.md`](../SPEC.md) is the law rather than a description written afterwards,
and there are seventeen of them: one per node, each owning the rules for its own
directory. 88 `§B` rows across the tree record every defect found so far paired
with the rule that now catches it.

A rule and its checker land in the same commit, because a rule with no runner is
a comment (`§V74`).

## The guardrails are git hooks that also run on CI

Entering the dev shell (`nix develop`, or `direnv allow`) installs `pre-commit`
and `pre-push`, which run [hk](https://github.com/jdx/hk) against one definition
of the gate in [`hk.pkl`](../hk.pkl) — 23 steps on commit, 24 on push, the slow
one being coverage. [`ci.yml`](../.github/workflows/ci.yml) calls that same
definition on three platforms, so a laptop and a runner cannot disagree.

The architecture diagram in the [README](../README.md) is `sherd graph` output
for the same reason: generated from the `§F` tables, so it cannot drift from
what the specs declare.

## The record is deliberately unflattering

`§B12` records that `AGENTS.md` says "never commit to `main`", that nothing
enforced it, and that ~20 commits landed on `main` in one session anyway — **the
rule was read by the agent it governs and still lost to convenience.** That is
the most useful row in the tree, and it is an argument for runners over prose
rather than an argument against agents.

`§B4` records a "95x" improvement claimed across six commit messages that
measured **2.1x** against a denominator anyone would actually use. A benchmark
number that survives six commits without anyone checking its denominator is a
number the process produced, not one a person chose.

Both are here rather than in a footnote because a repository about what models
can build is worth exactly as much as its record of what they got wrong.

## What a reader should actually check

In the order it matters:

1. **Does the gate run for you?** `nix develop` then `hk check`. If a claim
   here is false, that is where it shows.
2. **Read `src/fed/`'s own `§B` first.** It is the 20B's work with its defects
   intact, which is the evidence this project stands or falls on.
3. **Do the `§B` rows look real or curated?** `B12` and `B4` are above,
   unedited. Judge the other 86 by them.
4. **Do the invariants have runners?** An invariant nothing executes is a
   comment, which is `§V74`'s whole point.
5. **Does the generated diagram match the `§F` tables?** Run `sherd graph`. A
   generated artifact that has drifted is worse than a hand-drawn one, because
   it carries the authority of having been generated.

## Accountability

The human named in [`LICENSE`](../LICENSE) is responsible for this code,
including the parts a frontier model wrote, the parts a 20B wrote, and the parts
nobody caught. "The LLM wrote it" is an explanation of provenance, never a
transfer of responsibility.

Bug reports are welcome and unflattering ones are more useful — see
[`SECURITY.md`](SECURITY.md) for the ones that should not be public, and
[`CONTRIBUTING.md`](CONTRIBUTING.md) for everything else.

## Deeper

[`AGENTS.md`](../AGENTS.md) is the working guide ·
[`CONTRIBUTING.md`](CONTRIBUTING.md) is the loop ·
[`SPEC.md`](../SPEC.md) is the law and the backlog.
