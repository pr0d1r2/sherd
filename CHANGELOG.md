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
