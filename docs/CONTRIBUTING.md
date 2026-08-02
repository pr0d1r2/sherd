# Contributing to blackbox

`blackbox` is built **spec-first**, and federated: the design and the build
queue live in `SPEC.md` files, one per directory that needs one, with a `§F`
table naming the children. A human or an agent can drive the loop the same
way — that is the point of the tool.

## Get set up

```bash
direnv allow                     # or: nix develop
cargo build                      # needs ../itok as a path dep
export BBX_ENDPOINT=http://your-box:11434
export BBX_MODEL=gpt-oss:20b
```

`BBX_ENDPOINT` may be `https://` — TLS is compiled in. Use it whenever the
endpoint is not a machine you own, because what gets sent is slices of your
spec and your source.

Then install the gate:

```bash
git config core.hooksPath .githooks
```

## The loop

```bash
bbx budget                       # token cost of every node
bbx lens src/fed                 # the context pack for one node
bbx check                        # structural check, every node
bbx tdd src/fed V2 "<task>"      # red -> judge -> green -> gate -> repair
```

`bbx tdd` runs five kinds of call, each with a deliberately narrow context.
The **gate** step is local, deterministic and costs zero tokens — that is
where correctness is decided, not in the model.

- **The `SPEC.md` files are the queue.** `§T` rows carry a status: `.` not
  started, `~` in progress, `x` done.
- **`§V` is the law.** A task cites the invariants it must respect. Read them
  before starting.
- **Ids are monotonic and never reused.** Compute the next one; do not read
  `tail -1` as the next free id — it returns the highest existing one. Rows
  sort by id within a section, so append in order.

## The one hard rule

**Never `--no-verify`.** The hook says this itself, and it is the rule the
whole design rests on: a gate that can be stepped around is not a gate.

The same applies to the subtler versions — weakening a test, raising a
ceiling, loosening a judge, or silencing a warning to get past it. An unused
parameter is how a stub announces itself; do not prefix it with an underscore
to quiet the compiler. That evasion is in the `§B` log twice.

If a rule is wrong, that is a real and welcome finding. Change the rule
deliberately, in its own commit, with the reason recorded. The objection is
to routing around a verdict, not to disagreeing with one.

## Never hand-edit a generated slice

`vendor/principles/` is distilled into `src/tdd/principles.txt` by
`bbx slice`, and `bbx slice --check` gates the drift. Editing the generated
file *is* the drift. Regenerate and stage the result.

## Things that will get a patch turned down

- **A bypassed or weakened gate.** See above.
- **A claim with no evidence.** `bbx land` refuses a merge that cannot show
  its working; a pull request is held to the same standard. "This is faster"
  invites a measurement.
- **A stub that documents itself as sufficient.** Writing "sufficient for the
  test" in a docstring does not make it so, and this repo has a `§B` row
  about exactly that.
- **A fix aimed at the symptom.** If a check fires, the question is what it
  found, not how to stop it firing.
- **Context that grew without a reason.** Every call has a narrow context on
  purpose. Widening one is a design change and needs to be argued as one.

## Reporting a bug

Open an issue with what you ran, what you expected, and what happened. A
reproducing `SPEC.md` is worth more than a description of one.

Security issues go privately instead — see [SECURITY.md](SECURITY.md).
`blackbox` runs `git` and `cargo test` under model direction, so anything in
that area should not go in a public issue first.

## Commits

One logical change per commit, and the message carries the *why*. When a task
completes, flip its `§T` status to `x` in the same change.

## License

By contributing, you agree that your contributions are licensed under the MIT
License, the same terms as the rest of the project — see
[LICENSE](../LICENSE).
