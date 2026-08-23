# AGENTS.md

Federated `SPEC.md` for spec-driven development on **local** models. A 20B
writes code from a narrow context; you do the judgement it cannot.

## Read the diff

`cargo test` + `sherd check` have **passed wrong code three times**, all in §B:

- a filter on `not_owns` — the prose column — treated as a path
- a sanitiser that stripped `/` out of `dir` so a violation would vanish
- `Vec::new()` under a doc comment reading *"stub … satisfies the current test
  suite"*, committed unattended

A green gate is not correctness. It is the floor.

## Rules

- **Never `--no-verify`.** The gate refusing is the system working.
- **Never raise a ceiling, edit a test, or weaken a judge to make something
  pass.** Loosening a judge is how the stub got in.
- **Branch only.** Never commit to `main`. `sherd apply` enforces this.
- **`sherd check` clean before every commit.** The gate is `hk`; its ops live in
  `hk.pkl` — `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`,
  `sherd slice --check`, `sherd check`, in that order, cheapest first. They are
  file-scoped, so a SPEC-only commit skips the compile. Run the whole set by
  hand with `hk check --all`.
- **Commit from inside the dev shell.** The hooks REFUSE when `hk` is not on
  PATH rather than skipping — `direnv allow`, or `nix develop`. A gate that
  cannot run has not passed, and B17/B20/B24 are three recordings of exactly
  that being read as green.
- **Report what you examined**, not only what failed. A vacuous pass and a real
  one look identical.
- Commit the reasoning, not just the change. Git is this project's memory.

## Reading a SPEC.md

Caveman-encoded; symbols are load-bearing.

```
→ leads to    ∴ therefore    ∀ for all      ! must
⊥ never       ? optional     ≤ at most      ∈ in
```

Sections: `§G` goal · `§C` constraints · `§I` interfaces · `§R` research ·
`§V` invariants · `§T` tasks · `§B` bugs. `§R` and `§B` hold the evidence —
every measurement carries its source.

Ids are node-scoped. Cite across nodes with the namespaced, backticked form
`` `src/fed:V9` `` or microlith reads it as dangling.

## Writing a SPEC.md

- **§T states remaining work, never history.** "`X` landed, `Y` still open"
  makes a machine test `X`, which already passes.
- **§B for every defect**, at the node that owns it; crossing nodes → common
  ancestor. Prefer adding a §V that catches recurrence over a bug row alone.
- **Ids are monotonic and never reused.** Append; inserting moves every
  citation below it.
- Keep rules inline and rationale by reference. Rationale is ~80% of a mature
  §V, and it degrades the prompts built from that spec.

## Asset audience

- **worker** — reaches the local model's prompt: `NOTATION`, a node's
  `SPEC.md` at rule depth, signatures, tests.
- **supervisor** — this file, `.claude/`. Never reaches a worker prompt;
  enforced by discovery, not convention.
- **human** — `README.md`.

## Commands

```sh
sherd plan        next 3 steps, with what invalidates each
sherd apply       execute step 1, commit it, stop
sherd lens <dir>  the context pack for one node
sherd budget      token cost of every node
sherd check       structural check of every node
sherd graph       federation DAG, generated from §F
```

`sherd plan` **is** replan — stateless, re-derived every run.

Endpoint: `SHERD_ENDPOINT` (default `http://localhost:11434`), `SHERD_MODEL`
(default `gpt-oss:20b`). `sherd` comes from the dev shell (`direnv allow`).

## Claude Code

`/sit` runs one oversight cycle and halts with a recorded reason.
