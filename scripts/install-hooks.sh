#!/usr/bin/env bash
# Run by the dev shell's shellHook: point git at the tracked `.githooks/`.
#
# Without it a fresh clone or a new worktree commits UNGATED until someone
# remembers `git config core.hooksPath .githooks`. Guards, in order:
#   - only in sherd's own work tree (toplevel == $PWD, `hk.pkl` and
#     `.githooks/` present) -- entering from elsewhere must not touch a repo
#     that is not ours;
#   - never overwrite a `core.hooksPath` somebody already chose;
#   - LOUD when `hk` is missing, because the hooks refuse without it.
# Never fails the shell: a hook installer that locks you out of the shell you
# need to fix it is worse than the gap it closes.
set -u

top="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
[ "$top" = "$PWD" ] || exit 0
[ -f hk.pkl ] && [ -d .githooks ] || exit 0

if ! command -v hk >/dev/null 2>&1; then
  echo "sherd(shell): hk is not on PATH -- the git hooks will REFUSE to commit" >&2
  echo "  fix: re-enter via \`direnv allow\` or \`nix develop\`." >&2
fi

if [ -z "$(git config --local --get core.hooksPath 2>/dev/null)" ]; then
  git config --local core.hooksPath .githooks \
    && echo "sherd(shell): git hooks installed (core.hooksPath = .githooks)" >&2
fi
exit 0
