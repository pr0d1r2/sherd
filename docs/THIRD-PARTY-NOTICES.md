# Third-party notices

`sherd` carries three kinds of other people's work: two documents vendored
into the tree, and a dependency closure. Each is acknowledged below.

## `FORMAT.md` — vendored from cavekit

[`FORMAT.md`](../FORMAT.md) is not this project's work. It is the cavekit
`SPEC.md` format specification, vendored so that a tool which reads the format
also ships the format it reads.

- Upstream: <https://github.com/JuliusBrussee/cavekit>
- Copyright © 2026 Julius Brussee
- Licensed under the MIT License, reproduced in full below

What `sherd` *enforces* lives in its own [`SPEC.md`](../SPEC.md); the
vendored document is an input, not the identity.

> **This copy has drifted from upstream.** It is 119 lines against the 147 of
> the newest vendored copy known to us (in `microlith`, which owns the format
> check), and is missing the `§R RESEARCH` section along with the sectioned
> ownership rules under `WRITES`. Re-syncing is a deliberate job, not a
> silent one, and it is recorded here rather than discovered later.

## `vendor/principles/` — vendored from set-and-setting

Thirteen principle documents under [`vendor/principles/`](../vendor/principles),
copied 2026-08-01 from `set/skills/principles`.

- Upstream: <https://github.com/pr0d1r2/set-and-setting>
- Licensed under the MIT License

They are sliced into `src/tdd/principles.txt` by `sherd slice`, and
`sherd slice --check` gates the drift — so the copy in the binary cannot
silently diverge from the copy in the tree.
[`vendor/README.md`](../vendor/README.md) records *why* these thirteen and not
others, which is a judgement worth keeping next to the files.

### MIT License

Applies to both vendored sources above, and to `sherd` itself.

```text
MIT License

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Dependencies

**45 runtime packages** with `--all-features` (`cargo tree -e normal`).
Dev-dependencies are excluded: they are not distributed in anything you run.

Two are siblings, and their own notices apply in turn:

- **`itok`** — the token and context estimator. `sherd` uses its `bpe`
  tier to measure slices. See its
  [`THIRD-PARTY-NOTICES.md`](https://github.com/pr0d1r2/itok/blob/main/docs/THIRD-PARTY-NOTICES.md).
- **`microlith`** — owns the `SPEC.md` format rules.

The remaining closure is permissive throughout:

| licence | packages |
|---|---|
| `MIT OR Apache-2.0` (and spelling variants) | 31 |
| `MIT` | 6 |
| `Unlicense OR MIT` | 2 |
| `ISC` | 2 |
| `Apache-2.0 OR ISC OR MIT` | 1 |
| `Apache-2.0 AND ISC` | 1 |
| `BSD-3-Clause` | 1 |
| `CDLA-Permissive-2.0` | 1 |

Where an expression offers a choice, `sherd` is distributed under MIT and
takes the MIT option.

Worth naming individually, because they are the ones that are not plain
permissive-Rust:

- **`ring`** (`Apache-2.0 AND ISC`) — the cryptographic primitives behind
  `rustls`. Contains assembly and C, unlike everything else here.
- **`webpki-roots`** (`CDLA-Permissive-2.0`) — the Mozilla CA certificate
  set. Data rather than code. Copyright © the Mozilla Foundation.
- **`untrusted`**, **`rustls-webpki`** (`ISC`), **`subtle`**
  (`BSD-3-Clause`) — the rest of the TLS path.

All four arrive with the `ollama` feature, which is **on by default**. There
is no `Unicode-3.0` obligation: `ureq 3` avoids the `url` → `idna` → ICU
chain that `ureq 2` pulled in.

## Reproducing these numbers

Nothing here is hand-maintained, and you should not trust it because it is
written down:

```bash
cargo tree -e normal --all-features
cargo tree -e normal --no-default-features
```

## Trademarks

Nominative use only; no affiliation or endorsement is implied.

- **Rust** and **Cargo** are trademarks of the Rust Foundation.
- **GitHub** is a trademark of GitHub, Inc.
- **NixOS** and **Nix** are trademarks of the NixOS Foundation.
- **Ollama** is a trademark of Ollama Inc.
- **Claude** and **Anthropic** are trademarks of Anthropic PBC.

## `sherd` itself

Everything not covered above is licensed under the MIT License — see
[`LICENSE`](../LICENSE).
