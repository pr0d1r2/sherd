# Changelog

All notable changes to `sherd` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com), and this project adheres to
[Semantic Versioning](https://semver.org).

## Version ladder

A minor version here is a level of **guarantee**, not a feature count. Each
rung answers one question: *what can you rely on at this tag?*

**An even minor is stable; an odd minor is functional but not for
production** — the Linux 2.x and GNOME convention, and the rule
[`microlith`](https://github.com/pr0d1r2/microlith) states as its §V34, so
one reading serves both. The parity describes the *release*, not the work
that went into it. `SPEC.md` §V114 is the source; this table is its public
rendering, because a consumer arriving from crates.io never opens our spec.

| version | parity | what you can rely on | status |
|---|---|---|---|
| `0.1` | odd | the deterministic core runs on any repository — `budget`, `lens`, `fed`, `graph`, `check`, `slice`, `review`, `plan` — gated on three platforms and built with its tests in a sandbox | reached |
| `0.2` | even | `§I` is what ships: `init` scaffolds a `SPEC.md`, and a runner closes the interface-vs-binary drift in both directions | next |
| `0.3` | odd | the DAG answers questions — `route` resolves a query to a node, `validate` self-checks the federation | planned |
| `0.4` | even | the federation is maintainable rather than only readable — `split` proposes a split, `sync` regenerates `§N` | planned |
| `0.5` | odd | **first public artifact.** The loop's verdict means what the gate means, measured over more than one attempt, on a repository that is neither `itok` nor this one | planned |
| `0.6` | even | the surface settles: what the first users found, and `§F`/`§N` upstreamed rather than forked | planned |
| `1.0` | — | the contract freezes; every minor after is stable by definition, and the parity retires | planned |

`0.1` through `0.4` are rungs **reached but not published**. They are listed
rather than omitted, because a ladder that hides its unpublished rungs makes
the first release look like a first version instead of a fifth.

Pre-`1.0` SemVer permits a minor to break, and here each rung *is* a
behaviour change, so that permission is used honestly rather than worked
around. crates.io is immutable — yanking hides a version, it does not delete
it — so the first public artifact will be `0.5.0-rc.1`, which cargo does not
select by default.

## [Unreleased]

### Changed

- **The project is `sherd`.** One name for the package, the binary and the
  repository — `cargo install sherd` installs `sherd`. The crate was
  `bbx-cli` with a `bbx` binary because both `bbx` and `blackbox` are taken
  on crates.io, so no spelling of the old name could be shared by the package
  and the command. Environment variables are `SHERD_*`, state files are
  `.sherd-*`, and the dev crate is `sherd-dev`.

### Added

- **`nix build .#default`, with `doCheck` on** (`T71`, `src/cli:T9`). The
  package builds in a sandbox that copies the source without `.git`, which is
  the one environment able to catch a test asserting facts about the tree it
  runs in — `src/cli:B1`, invisible for the project's life because nothing
  ever ran the suite outside a checkout. The two tests that failed there are
  handed fixture repositories now, and the whole suite passes from a non-repo
  tree: 284 tests, 0 failures. CI gains a `nix-build` job on the same three
  platforms. The build produces `sherd` only; `sherd-dev` is compiled and tested
  in the sandbox but never installed.
- **Six linters in the gate**: `actionlint` (the workflow is code no local run
  exercises), `shellcheck` (`.envrc` runs on every shell entry), `nixfmt`,
  `taplo`, `typos` and `lychee --offline` for relative links. All arrive from
  the dev shell, so CI and a laptop run the same versions and none is a
  dependency of the crate. Two found something on their first run: `flake.nix`
  had never been formatted, and five spellings had been read past by every
  reviewer since they were written.
- **Nine hygiene steps in the gate** (`T87`): merge conflicts, private keys,
  oversized files, byte-order marks, case conflicts, broken symlinks,
  trailing whitespace, missing final newlines and mixed line endings. All
  run through `hk util`, so the set costs no new dependency. Each was run
  against a planted violation before landing, because zero findings and no
  possible finding read the same in a log -- and one of the nine only
  rejects a real merge marker when passed `--assume-in-merge`.
- **CI runs the gate, on three platforms.** `.github/workflows/ci.yml` enters
  the dev shell and runs `hk check --all --check` -- the same op set the
  hooks run locally, from the same pinned toolchain, so there is no second
  definition of what "green" means. `x86_64-linux`, `aarch64-linux` and
  `aarch64-darwin` each pay the whole gate; `x86_64-darwin` is declared in
  the flake and named in the workflow as ungated rather than left to look
  covered. There is no `nix build` job yet, because `src/cli:B1` would make
  a sandboxed build fail for a reason that has nothing to do with the
  package.

### Changed

- **`itok` is a registry dependency**, `0.3` from crates.io with a lock
  checksum, and the last path dep in the tree is gone. A clean clone now
  builds with no sibling checkout: the `V101` allow-list in `src/lib.rs` is
  empty, so a path dep added later fails the suite rather than being noticed
  by a reader. What made this possible is upstream, not here -- `itok` was
  published -- and the manifest comment claiming it was unpublished had
  outlived the fact.
- **The gate is now [`hk`](https://hk.jdx.dev)**, with its ops declared in
  `hk.pkl` instead of written as a shell body in `.githooks/pre-commit`. The
  ops are file-scoped, so a SPEC-only commit skips the compile; they are
  reachable by hand and from CI as `hk check --all`; and the fix half
  (`cargo fmt`, `sherd slice`) is declared next to the check half rather than
  described in a refusal message. `hk` comes from the `nix-hk` flake input,
  because nixos-26.05 ships none.
- **`cargo fmt` and `cargo clippy` are gated for the first time.** They had
  never been part of the gate, so an unformatted tree and 18 clippy findings
  had accumulated behind a verdict that read green; both are paid off. The
  formatter is pinned at 80 columns with edition 2024 in `rustfmt.toml`.
- **`-D warnings` moved from `RUSTFLAGS` to a clippy argument after `--`**, so
  it applies to this crate and not to `../itok`, whose own `dead_code`
  warnings had been turning this repo's gate red for code it does not own.
- **A `pre-push` hook exists.** `commit && push` chains where the commit
  aborted and the push shipped the old head are recorded twice in `§B`; the
  push side is now checked on its own.
- **MSRV is 1.95**, the fleet pin (`nixpkgs-lock` → nixos-26.05), reached by
  `microlith` 0.6.1 and `itok` 0.3.0 both declaring it upstream. The previous
  1.96 floor was a mirror of an older pin rather than a measured minimum.

### Packaging

- `hk.pkl` and `pkl/` are excluded from the published crate. They gate this
  working tree and mean nothing in a tarball.

## [0.1.0] - rung reached, not published

The first rung of the ladder above, and deliberately not an artifact. There
is no tag and no crates.io entry: `0.1` records that the deterministic core
runs on any repository and is gated, which is a guarantee worth stating and
not yet worth publishing. The first published version is `0.5.0-rc.1`.

This section previously read "First public release" and carried the date the
version number was chosen. It claimed an event that had not happened.

`sherd` splits a repository so that no single model call has to hold all
of it — a directory DAG where every directory may carry its own `SPEC.md`,
with a `§F` table naming its children.

### Added

- **Federation**: `budget` (token cost per node), `lens` (the context pack
  for one node), `fed`, `graph`, `check` (structural check across every
  node).
- **`sherd tdd`** — red → judge → green → gate → repair, each call given a
  deliberately narrow context. The gate step is local, deterministic and
  costs zero tokens: correctness is decided there, not by the model.
- **`sherd oneshot`** — the monolith arm, so the federated path can be
  compared against it rather than merely asserted better.
- **`sherd land`** — a merge that asks for evidence.
- **`sherd slice`** — distils `vendor/principles/` into `src/tdd/principles.txt`;
  `sherd slice --check` gates the drift, so the copy in the binary cannot
  diverge from the copy in the tree.
- Public documentation: `LICENSE`, `docs/SECURITY.md`,
  `docs/CODE_OF_CONDUCT.md`, `docs/CONTRIBUTING.md` and
  `docs/THIRD-PARTY-NOTICES.md`.

### Security

- **TLS on the model endpoint.** `ureq 3` with `rustls`, so `SHERD_ENDPOINT`
  may be `https://`. This matters because what gets POSTed is the prompt —
  slices of your spec and your source — and `ollama` is a *default* feature,
  so plaintext would have been the default path rather than an opt-in one.
  Moving from `ureq 2` also removed the `url` → `idna` → ICU chain, so the
  tree got smaller while gaining a TLS stack.

### Known gaps

Stated rather than left to be discovered:

- `route`, `split`, `sync`, `validate`, `SPEC.why.md` and the file ceilings
  are specced and unbuilt.
- `§F`/`§N` are extensions `microlith` cannot yet parse. They need to go
  upstream rather than fork the format.
- `itok` is a path dependency, so this crate cannot yet be built from a clean
  clone without it.
- Two functions in `src/fed/` were written by `gpt-oss:20b` through
  `sherd tdd`. Their defects are recorded in that node's `§B` rather than
  smoothed over.
