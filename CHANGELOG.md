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
| `0.2` | even | `§I` is what ships: `init` scaffolds a `SPEC.md`, and a runner closes the interface-vs-binary drift in both directions | reached |
| `0.3` | odd | the DAG answers questions — `route` resolves a query to a node, `validate` gives one verdict over the federation | reached |
| `0.4` | even | the federation is maintainable rather than only readable — `split` proposes a split, `sync` regenerates `§N`, `adopt` migrates a single-file spec onto one | reached |
| `0.5` | odd | **first public artifact.** Every verb that never calls a model is correct and reusable as a library, exercised on a repository that is neither `itok` nor this one | reached |
| `0.6` | even | that surface settles: what the first users found, and `§F`/`§N` upstreamed rather than forked | next |
| `0.7` | odd | the model half resumes — `ask`, `tdd`, `oneshot`, and the loop's verdict meaning what the gate's verdict means, measured over more than one attempt | planned |
| `1.0` | — | the contract freezes; every minor after is stable by definition, and the parity retires | planned |

`0.1` through `0.4` are rungs **reached but not published**. They are listed
rather than omitted, because a ladder that hides its unpublished rungs makes
the first release look like a first version instead of a fifth.

**The model half is frozen until `0.7`.** `ask`, `tdd` and `oneshot` work
today and are not going away, but no further development lands in
`src/ollama`, `src/tdd` or `src/assay` before the mechanical surface is
correct and published. The reason is that they answer different kinds of
question: whether a directory DAG can be parsed, budgeted and validated is
settled by tests, while whether a 20B can write code that survives review is
a research result that may take months to arrive. Tying a release to the
second would hold the first hostage, and the first is the half a consumer
can reuse.

Pre-`1.0` SemVer permits a minor to break, and here each rung *is* a
behaviour change, so that permission is used honestly rather than worked
around. crates.io is immutable — yanking hides a version, it does not delete
it — so the first artifact uploaded was `0.5.0-rc.1`, a pre-release cargo
does not select by default. It did what a release candidate is for: the
publish run surfaced two warnings that eleven gate steps had read past
(`.:B26`). `0.5.0` is that rung as a release.

## [Unreleased]

### Added

- **`sherd wave` schedules shell nodes by the scripts they call**
  (`src/wave:V5`, #82). A script a node owns that invokes a script under a
  sibling node (`source`, `.`, `bash`, `sh`, `exec`, `sh <x.sh`, or the path
  at command position) is a blocking edge, so the caller is built in a later
  round than the node it calls. Paths are resolved from what the file
  itself states: a literal, `$(dirname "$0")` or `BASH_SOURCE`,
  `$(cd X && pwd)`, `$(git rev-parse --show-toplevel)`, a variable the file
  assigned one of those, or the default of `${VAR:-default}`. A call that
  names no tracked script, such as an argument, an inherited variable, or a
  name built at run time, is listed under `UNRESOLVED` rather than guessed.
  A tree with no scripts gets the schedule it got before. Library:
  `wave::unresolved_calls`, `wave::Unresolved`, `fed::owned_script_files`.

### Fixed

- **`sherd plan` no longer writes the state store** (`src/cli:B12`, #79).
  Both the text and the `--format json` form cleared and rewrote the
  `plan` keys in `<git-common-dir>/sherd-state` on every run, although
  nothing read them: `apply` works the plan out again itself. The store is
  shared by every worktree of a repository, so a read-only verb in one
  worktree changed state another one reads. `plan` now only reads the
  store, for believability and the kept/tried record.
- **`sherd sync` leaves one blank line under an empty section**
  (`src/spec:B5`, #106). A node spec with an empty `§V`, `§T` or `§B` got
  two blank lines under the heading, which markdownlint's MD012 refuses,
  and collapsing them by hand made `sync --check` report the node stale.
  `sync` now writes one, and reads the one-line form back unchanged.
- **`sherd adopt <dir>` splits the node it names** (`src/adopt:B6`, #104).
  It read the root `SPEC.md` whatever directory it was given, so a node
  could not be split into its children: every id living in `a/b/SPEC.md`
  was refused with "the source declares no". The source is now
  `<dir>/SPEC.md`, homes are the nodes declared below it, and citations
  of a moved row are rewritten in every node of the tree, including ones
  the move writes no row into: `` `a/b:V9` `` becomes `` `a/b/x:V9` ``,
  and a bare `V9` inside `a/b/x`. `sherd adopt .` reads the root as
  before, and now also rewrites other nodes' `` `.:V9` `` citations of a
  row that moved. Library: `adopt::{propose_at, refusals_at, apply_at,
  unreadable_at}`, `spec::rehome`.
- **`sherd adopt` places a row by the script it cites** (`.:B33`, #94).
  The 0.5.3 entry for this was wrong: the feature shipped as a spec row
  and a test file that nothing compiled, with no code behind it. It is
  now implemented. A row citing scripts that all sit under one declared
  node goes to that node. A row citing scripts in two nodes, or in none,
  stays at the root and is named there. A test now fails the build when
  any `tests/` file is not compiled. Library: `split::cited_scripts`.

## [0.5.3] - 2026-10-03

Still the `0.5` rung, and still a patch: the rung's promise -- every verb that
never calls a model, correct and reusable as a library -- has not moved, and
most of this release is that promise being kept where it was not. `0.6` is
where the surface settles. New here: `--format json` on `budget`, `check` and
`validate`, `split` for shell codebases, `adopt` placing a row by the script
it cites, V44 enforced for `SPEC.why.md`, and a default-build coverage floor.

`cargo semver-checks check-release --baseline-rev v0.5.2` requires no semver
update; every new library item is additive. Unlike `0.5.2`, some exit codes
**do** change, each because the old one reported success or the wrong failure:

- `check` and `validate` exit 1 on a node whose `SPEC.md` cannot be read,
  where they passed it silently. A tree that was green only for that reason
  now fails.
- `budget` exits 1 on a `.context-limits` that exists and cannot be read,
  where it fell back to the default ceilings.
- `lens <dir>` exits 2 on a directory that is not a node, where it printed
  an ancestor's pack and exited 0.
- `adopt` and `sync` exit 2, not 1, when they cannot read their input.
- `land` no longer refuses a repository with no `.sherd-slices`.

Node discovery also changes what it counts: gitignored paths are skipped
inside a git work tree, and a lowercase `spec.md` is no longer a node on a
case-insensitive filesystem. `rule` depth drops finished `§T` rows, so
`budget` totals and lens packs shrink on any node that keeps them.

### Added

- **`sherd adopt` places a row by the script it cites** (`src/adopt:V10`,
  #94). A row citing one or more scripts goes to the deepest declared node
  containing them, with the script paths as its reason, before any lens
  word is read. Scripts are resolved the way `split` resolves them: by
  path, or by a basename naming exactly one script. A row whose scripts
  sit in two nodes, or at the root, stays at root and is named, as a tie
  does. A row citing no resolvable script is placed by the lens as before.
  Library: `split::cited_scripts`.

- **`sherd budget`, `check` and `validate` take `--format text|json`**
  (`src/cli:V19`). `text` is the default and its output is byte-identical
  to before. `json` is one object per run with fixed keys, the same contract
  `plan --format json` has: `version`, an `ok` that always matches the exit
  code, and paths relative to the repository root (the root itself is `.`).
  `budget` reports each node's `chain_tokens`, `own_tokens`, `chain_nodes`,
  `ceiling` and `over_by`, plus the totals and the token-count `method`. A
  node it could not measure is named in `unmeasured`. `check` and
  `validate` report each finding with `file`, `line`, `rule`, `message` and
  `fatal`. `rule` is `null` where the text form names no rule. Exit codes
  do not change with the format, and an unknown format is a usage error
  (exit 2). Library: `lens::own_cost`.

- **`sherd split` proposes a federation for a shell codebase**
  (`src/split:V7`, `src/split:B6`, #81). A direct child directory holding
  a tracked `*.sh` at any depth is a `directory` candidate, and three or
  more flat scripts sharing the name before their first `-` are a
  `family`. A candidate is weighed by the spec rows citing a script it
  would own, by path or by a basename that names exactly one script; a
  basename naming several is counted for none and listed. A bare directory
  word is not a citation. Each candidate shows its script count and the
  busiest script dirs beneath it, so a deep tree can be split one level per
  run. A directory both the Rust and the shell reading find is proposed
  once. A tree with neither keeps `no module declarations found -- nothing
  to propose`. Library: `split::modules` (the Rust reading alone),
  `split::scripts_of`, `split::rank_in`, `split::ambiguous_scripts`,
  `fed::script_files`; `split::structure` now returns both readings, and
  `split::rank` is unchanged.

- **`sherd check` enforces V44 for any node that keeps a `SPEC.why.md`.**
  Every `§V` id in `SPEC.md` must have a row in `SPEC.why.md` (a row of `-`
  counts), and no why row may name a rule that is gone. Each gap is a
  violation. A node without a why file is not checked. Library:
  `spec::why_gaps`.

- **`sherd coverage` measures the default build too** (`src/debt:V4`,
  `src/debt:B8`). A `.coverage` may carry a `lines-default` row next to
  `lines`; when it does, `--check` and `--record` run `cargo llvm-cov`
  without feature flags as well, and a drop in either build fails. Without
  the row the verb reports "no floor recorded (none required)" and behaves
  as before, apart from each output line now naming its row. Library:
  `debt::Build`, `coverage_of`, `recorded_floor_of`, `record_coverage_of`;
  `coverage`, `recorded_floor` and `record_coverage` are unchanged and
  measure `--all-features`. `record_coverage`'s messages now name the row.

### Changed

- **`rule` depth carries only remaining work in `§T`** (`src/spec:V11`). A
  task row marked `x`, and an `ARCHIVED to SPEC-ARCHIVE.md` stub, no longer
  ship in a lens pack or count toward `budget`; open (`.`) and started (`~`)
  rows do. The file is unchanged and `check` still resolves citations to
  finished rows. On this repository: 227,822 -> 226,794 tokens across all
  chains. On a 12-node consumer that keeps archive stubs: 35,680 -> 31,249
  (-12.4%).

- **Contributor-facing: the dev shell fetches its flake inputs over git**
  (`.:T109`, #97). `nixpkgs-lock` and `nix-hk` are `git+https://` inputs
  rather than `github:`, because some sandboxed environments refuse the
  archive tarball `github:` downloads while allowing plain git reads.
  `flake.lock` pins the same revisions and hashes as before.

### Fixed

- **A lowercase `spec.md` is no longer a node on macOS** (`src/fed:V23`,
  `src/fed:B16`). Node checks asked `dir.join("SPEC.md").is_file()`, which a
  case-insensitive filesystem (APFS, NTFS) answers for `spec.md` too, so the
  same tree had more nodes on macOS than on Linux. A node is now a dir with
  an entry named exactly `SPEC.md`. Discovery, chains, `lens`, `split`,
  frozen nodes and the repository-root walk all ask the same predicate.
  Library: `fed::is_node`.

- **Discovery skips what git ignores** (`src/fed:V22`, `src/fed:B15`, #80).
  Inside a git work tree, `discover`, the `§F` exhaustiveness scan and
  `rust_files` now skip every path `git ls-files --others --ignored
  --exclude-standard --directory` names, so a `SPEC.md` in a gitignored
  scratch checkout is no longer a federation node. Every verb reads the
  same walk, so `wave`, `validate`, `check`, `budget`, `plan` and the rest
  change together. On a consumer with one tracked `SPEC.md` and gitignored
  checkouts of other repositories, `wave` went from 884 nodes to the real
  ones. The fixed name list (`target`, `vendor`, `.claude`, ...) still
  applies, tracked or not. Outside a work tree, or without git, discovery
  is unchanged.

- **`sherd land` no longer refuses a repository with no `.sherd-slices`**
  (`src/land:V12`, `src/land:B6`). Its gate treated the missing slice
  registry as an error, so `land` refused with `.sherd-slices: No such file
  or directory` in every repository that never declared a slice. The gate
  now reads that absence the way `sherd slice --check` and `validate`
  already did: it reports `slice: none required` and goes on. A registry
  that exists and cannot be parsed is still an error. Library:
  `land::gate` and `land::gate_with` return `Ok` for such a tree where they
  returned `Err`.

- **`sherd check` and `sherd validate` fail on a node they cannot read**
  (`.:V48`, `.:B31`). A discovered node whose `SPEC.md` could not be read,
  for example because it is not UTF-8, was skipped, and the tree passed:
  `2 nodes examined · 0 violations`, exit 0. It is now one violation,
  printed as `<path>: sherd/V48: cannot read -- <error>`, and exit 1.
  `sherd land`'s gate counts it the same way. A tree that was green only
  because a node could not be read now fails.

- **`sherd adopt` and `sherd sync` exit 2, not 1, when they cannot read
  their input** (`src/cli:V1`, `src/cli:B11`). `adopt` with a missing or
  malformed `--map`, or on a directory with no `SPEC.md`, and `sync` on a
  node whose `SPEC.md` cannot be read, all exited 1, which these verbs use
  for "a migration is pending" and "it wrote". A wrapper acting on 1 now
  sees 2 for these. A map `adopt` reads and refuses still exits 1.

- **`sherd lens <dir>` on a directory that is not a node exits 2** (`src/lens:V5`,
  `src/lens:B2`). It printed the pack of the nearest ancestor under the
  requested name and exited 0, so a mistyped node gave a worker another
  node's rules. It now reports the miss the way `sherd budget` does, with
  the spelling that would have matched. Library: `lens::pack` returns a
  `NotFound` error for a `dir` with no `SPEC.md`, where it returned `Ok`.

- **An unreadable `.context-limits` is an error, not a cold start**
  (`src/tokens:V5`, `src/tokens:B1`). Any failure to read the file was
  taken as "no file", so one that exists and cannot be read silently
  became the default ceilings. Only a missing file is a cold start now;
  `sherd budget` reports any other read error and exits 1. Library:
  `tokens::Ceilings::load` returns `Err` for it.

## [0.5.2] - 2026-09-28

Still the `0.5` rung, and still a patch: the rung's promise -- every verb that
never calls a model, correct and reusable as a library -- has not moved.
`0.6` is where that surface settles. This release adds one machine-readable
output (`plan --format json`), moves federation proposal into its own node
without breaking a `0.5.1` caller, corrects what `sherd check` reports, and
stops writing the state file into whatever worktree sherd runs in.

`cargo semver-checks check-release --baseline-rev v0.5.1` requires no semver
update. Nothing here changes an exit code: every finding whose count moves
(V50 ceilings, `fed:V9` done rows) is advisory. A consumer that parses the
advisory lines, or ratchets on their count, will see the numbers move --
V50 up where suites live in a `tests/` tree, `fed:V9` down where `mth
archive` has run.

### Added

- **`sherd plan --format json`** (and `plan --milestone M --format json`)
  prints the plan as one JSON object for a program to read: each step with
  its rank, kind, node, id, text, believability, kept/tried record, context
  tokens and an `invalidated_by` list, plus every unmanaged row with its
  node, id and reason, and the milestone filter's `outside_milestones`
  count. The root node is spelled `.`. The text form may change; the JSON
  form only gains keys (`src/plan:V25`). An unknown `--format` is a usage
  error (exit 2), never a fallback to text. Library: `plan::to_json`,
  `Confidence::invalidators`. Requested in #36 by a task loop that had to
  parse the text layout.

### Changed

- **The default state file lives in the git common dir, not the cwd**
  (`src/state:V4`, `src/state:B3`, #62). It is now `SHERD_STATE` if set,
  else `<git-common-dir>/sherd-state` inside a repository, else
  `.sherd-state` in the cwd outside one. Before, the fallback was always a
  cwd-relative `.sherd-state`, so sherd run inside another repository wrote
  into that worktree. The consumer's `.gitignore` did not list it and its
  `.crate` shipped it: microlith's `package` gate step failed on exactly
  that. **What a consumer does:** delete any `.sherd-state` left in a
  worktree, because it is no longer read. The store is a cache, so the only
  cost is one cold start. Nothing needs adding to `.gitignore`, since the
  git dir is never tracked or packaged. `SHERD_STATE` still overrides, and
  every worktree of a repository shares one store. Library:
  `state::resolve`, `state::git_common_dir`.

- **Federation proposal is its own node, `src/split`** (the task was
  `src/plan:T14`, done in 31b6fcd and removed per `src/fed:V9`).
  `structure`, `row_weight`, `rank`, `uniform_evidence`, `Evidence`,
  `Proposed`, `Ranked`, `propose` and `Proposal` moved from `sherd::plan`
  to `sherd::split`. `sherd::plan` re-exports all nine, so code written
  against `0.5.1` still compiles; new code should name `sherd::split`. The
  `split` and `plan --triage` commands are unchanged. `src/plan`
  now owns one subject, what to attempt next.

- **Contributor-facing: every node's tests live in its own `tests/` tree**
  (V124, `.:T108`), and `src/cli` is split by verb family (`src/cli:V18`).
  No public item moved; the crate's own V50 findings changed with the
  layout, which is what the `tests/` fix under Fixed is about.

### Fixed

- **A body-less `#[cfg(test)] mod x;` no longer hides the code after it**
  (V50, `src/code:B2`). The code/tests split waited for the attributed
  item's closing `}`, and a declaration has none, so the next production
  item was counted as tests. It now closes at the declaration's own `;`.

- **`sherd check` measures a file in a `tests/` tree as tests** (V50).
  A suite kept in its own file, included through `#[cfg(test)] mod
  tests;`, carries no `#[cfg(test)]` of its own, so it was measured as
  code, against a ceiling twice the test one. Test-ceiling findings
  disappeared when a suite moved out of its implementation file. Any
  file with a `tests` path component is now test code in full. In this
  repository that surfaced `dev/tests/cli.rs`, over its ceiling all
  along.

- **`sherd check` no longer reports `mth archive` stubs as done rows**
  (`src/fed:V9`, `src/fed:B14`, #60). `mth archive` moves a finished row's
  text to `SPEC-ARCHIVE.md` and leaves a stub (`T88|x|ARCHIVED to
  SPEC-ARCHIVE.md|V42`), so a milestone still finds the row and every
  citation still resolves (`microlith/V48`). Each stub was flagged as
  history, and the only way to clear the finding was to delete the row,
  which breaks V48. In xenolith that was 115 advisory findings. A done row
  that still carries its text is flagged as before.

## [0.5.1] - 2026-09-21

Still the `0.5` rung: what it promises -- every verb that never calls a model,
correct and reusable as a library -- is unchanged, and so is the parity. What
changed is how much of that promise holds. Most of this release is defects
found by running the deterministic verbs on repositories that are neither
`itok` nor this one, which is the rung's own test, plus two report-only verbs
(`seam`, `wave`) that write nothing and call no model.

`cargo semver-checks check-release --baseline-rev v0.5.0` requires no semver
update: every public item `0.5.0` shipped is still there with the signature
it shipped with. `plan()` and `Plan` in particular keep their shape; the
milestone filter arrives beside them rather than inside them.

### Fixed

- **`wave` no longer counts a type-only import as a wait** (`src/wave:B1`,
  `V4`, `src/code:V5`). Every `use crate::` edge was blocking, so `wave`
  reported the pessimistic shape precisely when a seam commit had made the
  edges non-blocking: `sherd seam` proposes the vocabulary that frees them
  and `wave` measured the same rounds anyway, so the two verbs told a reader
  contradicting stories. `.:R57` has the measurement — a ten-node repository
  `wave` called five rounds deep, which seven workers in fact built in one.

  An edge is type-only when every item the line names is a public type the
  sibling declares: `use crate::lint::Level` is a reference to a type a seam
  commit has already created, not a wait for `lint`'s logic. An import of the
  module itself names no item and is never type-only, and one behavioural
  reach makes the whole edge blocking.

  The report now carries **both** numbers — `N sibling edge(s) · M blocking ·
  K type-only` — because only one of them decided the rounds, and a reader
  cannot see which from `depth` alone.

- **`sherd adopt` names a milestone RANGE it cannot split, instead of failing
  three steps downstream** (`src/adopt:B5`, `V9`, `src/spec:V10`). A monolith
  whose milestone row lists `T1-T3` -- the form `microlith/V15` documents as
  the cheap way to maintain that column, and so the form a brownfield spec
  most often carries -- was refused with `microlith/V15: T3 is in no
  milestone`, a symptom naming a row that never moved. The citation rewrite
  walks token by token, so the range became `` `child:T1` ``-`T3`, which
  expands to no tasks at all.

  The range is now detected before anything is written and quoted as the
  reader wrote it: *``M1`` lists its tasks as a RANGE (`T1-T3`) and this map
  moves T1, T2 -- a range cannot survive a split. Expand it into ids before
  adopting.* It is not expanded automatically, because the ids of one
  milestone may land in two nodes and which of them keeps the row is a
  judgement. A range no row of the map touches is left alone, so a rerun over
  an already-migrated tree still exits clean (`V6`).

- **`sherd adopt` refuses a source it cannot read instead of reporting it
  empty** (`src/adopt:B4`, `V8`, `src/spec:V9`). A `SPEC.md` whose `§T` is
  written as a bracketed markdown table -- `| T1 | . | first task | - |` --
  parsed as having no rows at all, so `adopt --check` printed
  `0 rows read · 0 placed · 0 unplaced` and exited 0. In this verb's exit
  scheme that means *nothing to move*, which on a repository being weighed
  for adoption reads as *already federated* -- the opposite of the truth, in
  the same words and at the same exit code as the honest answer.

  Rows carrying an id in a form the reader declines are now detected before
  any count is printed, named with the line each sits on, and the command
  exits 2. Exit 0 keeps meaning "I read this and there is nothing to move".
  Milestone tables are untouched: `microlith::milestones` reads that dialect,
  and `M` is not an id this grammar owns.

- **`V50` weighs every region of a `.rs` file, not everything above the
  first test module** (`.:B29`, `T107`). A file was cut into code and tests
  at the FIRST `#[cfg(test)]`, so production code written below a test
  module was weighed against the test ceiling and never counted as code at
  all. `code::split_regions` now sums every non-test region and every test
  region; `split_module` keeps its single cut for the callers that need one
  position in the file (`src/tdd`, `src/review`).

- **`sync` writes a `§N` lens back as the escaped cell it read**
  (`src/fed:B13`, `V16`). `split_row` unescaped `\|` when reading a `§F`
  row, but `§N` was written raw, so any `owns` cell holding a pipe came back
  with an extra column in every `§N` naming that node -- 17 files in this
  repository alone.

- **A `§C`/`§I` id written as a bullet can be cited** (`src/spec:B4`, `V8`).
  `FORMAT.md` writes those sections as `- C1: …` bullets, but `declares()`
  matched only a line opening with the id, so `` `node:C1` `` pointing at a
  real bullet reported `resolves to no row` -- constraints became uncitable
  the moment a monolith's `§C` was split across nodes.

- **`adopt` inserts received rows in id order** (`src/adopt:B3`, `V7`). Rows
  moved into a node that already held rows were appended after them, so a
  second adoption moving `T3` beside a resident `T88` wrote an order
  `microlith/V14` rejects, and the delta check refused the whole migration
  -- an order the verb had written itself.

- **The code DAG reads `use crate::{a, b}` groups** (`src/code`). A brace
  group collected an empty module name and the line was dropped, so
  `src/cli`, which names eight siblings on one such line, read as depending
  on nothing and landed in the first ready set.

- **A relative `[dir]` is resolved before the root walk** (`src/cli`). The
  argument itself was taken for the repository root and then joined onto
  itself, so `sherd seam code` from `src/` answered `code/code matched no
  node`. Every `[dir]` verb was affected -- `budget`, `seam`, `lens`,
  `check`, `fed`.

- **Pushing a release tag no longer crashes the pre-push hook.** git hands
  `pre-push` the annotated tag object, which hk fed to a merge-base without
  peeling, so `cargo release push` failed at its last step on both `0.5.0`
  releases. The hook now skips only when every pushed ref is a tag whose
  commit `origin/main` already contains; a branch, or a tag on an unmerged
  commit, still pays the gate.

### Changed

- **`microlith` 0.6 -> 0.7.1.** 0.7.1 exports the milestone partition that
  `plan --milestone` reads, rather than sherd re-parsing the grammar, and
  0.7.0 brought `microlith/V42`: a row whose literal `|` is unescaped splits
  into more fields than its table declares. `sherd check` now reports that,
  so a spec that passed under `0.5.0` may not -- eight such rows in this
  repository were escaped before the bump.

- **The gate audits the CI workflow with `zizmor` at `pedantic`** alongside
  `actionlint` (`.:V122`, `B28`). Contributor-facing only; the crate's
  behaviour is unchanged.

- **`wave` is its own node, `src/wave`** (`src/plan:T14`). The scheduler --
  the code DAG, the ready set per round, depth and width -- moved out of
  `src/plan`, whose subject is what to attempt *next* and why it might not
  survive contact. It takes its rule with it: the code DAG is not the
  federation DAG is now `src/wave:V1`, and the three citations of it were
  repointed. `src/wave` is also where the executor half, frozen until rung
  `0.7`, will live, which is somewhere that is not the planner.

  Its public API is `wave::wave`, `wave::schedule`, `wave::code_deps`,
  `wave::Schedule` and `wave::CodeDep`. They briefly lived under `plan::` on
  `main`, but no release carried them there, so nothing a `0.5.0` consumer
  could have imported moved.

  One thing the split MEASURED rather than assumed: `src/plan/mod.rs`
  reports the same code weight before and after (4,468 tok), while its tests
  fall 20,093 → 16,879. `V50` splits a file at the *first* `#[cfg(test)]`,
  so the 337 lines of scheduler below `plan`'s first test module had been
  counted as tests all along. Recorded as `.:B29`; fixed above.

- **`src/fed` reads pipe rows with microlith's exported codec** rather than
  its own (`src/fed:V4`, `V17`). `split_row` is `microlith::cells` +
  `unescape`, `escape_cell` is `microlith::escape`; only the trim and the
  owned cell stay local, because those are `fed`'s shape and not the grammar.
  Three defects came out of the local reading -- `B11`, `B12` and `B13` -- and
  the last of them was the reader and the writer drifting apart, which one
  codec used in both directions cannot do.

  One encoded form changes: upstream doubles every backslash, where the local
  writer doubled only the ones the splitter would have re-read. Both decode
  to the same cell, which is what `V16` asserts and all a reader sees; a `§N`
  row holding a Windows path is rewritten once by `sync`. No such row exists
  in this repository -- `sherd sync --check` reports 20 nodes, 0 stale.

- **A `[dir]` that matches no node now names the spelling that would have
  worked** (`src/cli:V17`, `src/fed:V18`). `[dir]` is resolved against the
  repository root, so `sherd seam code` typed in `src/` looks for
  `<root>/code` and misses while `src/code` sits right there. The contract
  stays -- an argument whose meaning depends on where the caller stands is
  the ambiguity `src/cli:B5` and `B7` were about -- and the miss teaches
  instead of ending the conversation:

  ```
  sherd: /repo/code matched no node
    [dir] is resolved against the repo ROOT, not the directory you are standing in (V15).
    did you mean `src/code`?
  ```

  The suggestion is read from the spelling alone and never from the working
  directory. A name no node carries gets no invented suggestion, and a name
  several nodes carry is listed rather than guessed.

### Added

- **`sherd seam [dir]`** prints, per node, the public types it declares --
  the vocabulary a sibling can spell before either node is written, which is
  what lets nodes be built in parallel (`.:R57`). Report-only: which of those
  names are shared is a judgement, so there is no `--apply`. Library:
  `code::public_types`.

- **`sherd wave [dir]`** prints the rounds a parallel build would run -- the
  ready set per round, with depth and width -- from the CODE DAG (`use
  crate::` between sibling nodes), which is not the federation DAG
  (`src/wave:V1`). It creates no worktree and calls no model; the executor
  half stays frozen until rung `0.7`. A cycle is named, and the verb still
  exits 0.

- **`sherd plan --milestone <M>`** narrows the horizon to the rows milestone
  `M` claims, read from each node's own `§T` milestone table through
  `microlith::milestones`. Open rows in nodes that declare no milestones are
  counted and printed rather than dropped, and a milestone no node declares
  exits 2 -- a typo must not look like a finished milestone. Library:
  `plan::plan_in`, `in_milestone`, `milestone_declared`.

- **`sherd --version` and `sherd -V` print `sherd <semver>` on stdout and exit
  0** (`src/cli:B10`). Both spellings previously fell through to the unknown
  command arm, so the binary answered the question with its usage banner on
  stderr and exit 2 -- the code reserved for a malformed invocation. A CI gate
  that records the version of every tool it ran could record every sibling in
  the toolchain and not this one. The number comes from `CARGO_PKG_VERSION`,
  so it cannot drift from the manifest.

## [0.5.0] - 2026-08-31

The `0.5` rung as a release. Identical in surface to `0.5.0-rc.1` -- the
`semver` gate diffs the two and finds no public API change -- and different
in one thing the candidate existed to find.

### Fixed

- **The featureless build warned, and every gate step read green** (`.:B26`).
  `cargo build --no-default-features` emitted two warnings -- an unused `mut`
  in `run_args` and a dead `preflight` -- and the default feature set is
  empty, so that is the build a consumer gets. They rode through eleven gate
  steps, through `cargo release hook`, and into the published `0.5.0-rc.1`
  `.crate` and its own verify. Every one of those exited 0.

  `preflight`'s only non-test caller is `apply`, which is `ollama`-gated, and
  the `mut` exists solely for the gated `-v` removal; both are now gated with
  the code that uses them.

  The fix that matters is the gate, not the two lines. `clippy` denies
  warnings but runs `--all-features`, so it never compiles this
  configuration; `no-default-features` compiled it and accepted warnings. The
  configuration that SHIPS was the one no step held to a standard.
  `no-default-features` now carries `RUSTFLAGS=-D warnings`, verified against
  a planted violation.

### Changed

- **The release ladder's `0.5` rung is marked reached**, and `0.6` becomes
  next. `0.1` through `0.4` remain rungs reached without an artifact.

## [0.5.0-rc.1] - 2026-08-30

The first artifact published to crates.io, and a release candidate rather
than a release: pre-`1.0` SemVer lets a minor break, crates.io is immutable
— yanking hides a version, it does not delete it — and a pre-release version
is one cargo does not select by default. What the rung claims is the
deterministic core: every verb that never calls a model, reusable as a
library as well as a binary.

`0.1` through `0.4` were rungs reached and left unpublished, so this entry
carries their verbs as well. Four rungs of work reaching a reader as a
version number with no content is the thing a changelog exists to prevent.

### Added

- **`sherd init [dir]`** scaffolds a `SPEC.md`, deriving its `§F` rows from
  the child directories already present. The `0.2` rung.
- **`sherd route <query>`** answers which node owns a question, in its exit
  code and not only in prose: `0` hit, `2` miss, `3` ambiguous. A query no
  node owns and a query two nodes own are different failures, and a caller
  that cannot tell them apart cannot act on either.
- **`sherd validate`** gives one verdict over the whole federation — DAG
  shape, id uniqueness, token ceilings, slice drift — so a repository has a
  single question to ask before it trusts its own spec. With `route`, the
  `0.3` rung.
- **`sherd split [dir]`** proposes a federation split for a node that has
  outgrown its ceiling, and **writes nothing**: the proposal is output, the
  edit stays a human's.
- **`sherd sync [dir]`** regenerates `§N` from `§F` and exits `1` when it
  wrote, which is what makes it usable as a gate op rather than only as a
  fixer.
- **`sherd adopt <dir>`** migrates a foreign single-file `SPEC.md` onto a
  federation, with `--map` for an explicit section-to-node mapping and
  `--check` for the dry run. Every repository but this one now has a path
  in. With `split` and `sync`, the `0.4` rung.

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
  covered. The `nix build` job named above joined it once
  `src/cli:B1` was fixed; before that, a sandboxed build would have failed
  for a reason that had nothing to do with the package.

### Changed

- **Default features are empty.** `cargo install sherd` now builds the
  deterministic core and nothing else -- no HTTP client, no TLS stack, no
  network code in the binary. `ollama` adds `ask`, `tdd` and `oneshot`, and
  the binary names the missing feature rather than reporting an unknown
  command. The default was `["ollama"]` on the reasoning that the loop is the
  point of installing; the ladder says otherwise now, and a default that
  carried an endpoint client into every install made the featureless build
  the corner case for a tool whose §C says the core may never call a model.
- **The project is `sherd`.** One name for the package, the binary and the
  repository — `cargo install sherd` installs `sherd`. The crate was
  `bbx-cli` with a `bbx` binary because both `bbx` and `blackbox` are taken
  on crates.io, so no spelling of the old name could be shared by the package
  and the command. Environment variables are `SHERD_*`, state files are
  `.sherd-*`, and the dev crate is `sherd-dev`.

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

- **`result` is no longer tracked by git.** `nix build` leaves it as a
  symlink into `/nix/store`, so the committed entry pointed at a path that
  exists on one machine. `.gitignore` had listed it since before it was
  committed, and an ignore rule does not apply to an already-tracked path.
  The crate's `exclude` list already kept it out of the tarball; this is the
  repository half of the same fix.
- **`*.profraw` moved from `.git/info/exclude` into `.gitignore`.** The
  coverage op drops them in the working tree, and `.git/info/exclude` is not
  cloned, so every contributor but the one who wrote it saw the droppings as
  untracked files.
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
