# Changelog

All notable changes to `blackbox` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com), and this project adheres to
[Semantic Versioning](https://semver.org).

The crate is `bbx-cli`; the binary it installs is `bbx`. Those are two names
on purpose — `bbx` was taken on crates.io by an unrelated BBCode parser, and
the binary name was verified free rather than renamed to match.

## Versioning

Pre-`1.0`, a minor bump may change behaviour. `blackbox` is early: what works
and what is specced-but-unbuilt is listed in the README's Status section
rather than implied by the version number.

## [Unreleased]

### Added

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
  (`cargo fmt`, `bbx slice`) is declared next to the check half rather than
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

## [0.1.0] - 2026-08-07

First public release. Early, and honest about it.

`blackbox` splits a repository so that no single model call has to hold all
of it — a directory DAG where every directory may carry its own `SPEC.md`,
with a `§F` table naming its children.

### Added

- **Federation**: `budget` (token cost per node), `lens` (the context pack
  for one node), `fed`, `graph`, `check` (structural check across every
  node).
- **`bbx tdd`** — red → judge → green → gate → repair, each call given a
  deliberately narrow context. The gate step is local, deterministic and
  costs zero tokens: correctness is decided there, not by the model.
- **`bbx oneshot`** — the monolith arm, so the federated path can be
  compared against it rather than merely asserted better.
- **`bbx land`** — a merge that asks for evidence.
- **`bbx slice`** — distils `vendor/principles/` into `src/tdd/principles.txt`;
  `bbx slice --check` gates the drift, so the copy in the binary cannot
  diverge from the copy in the tree.
- Public documentation: `LICENSE`, `docs/SECURITY.md`,
  `docs/CODE_OF_CONDUCT.md`, `docs/CONTRIBUTING.md` and
  `docs/THIRD-PARTY-NOTICES.md`.

### Security

- **TLS on the model endpoint.** `ureq 3` with `rustls`, so `BBX_ENDPOINT`
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
  `bbx tdd`. Their defects are recorded in that node's `§B` rather than
  smoothed over.

[0.1.0]: https://github.com/pr0d1r2/blackbox/releases/tag/v0.1.0
