#!/usr/bin/env bash
# The body of the `zizmor` step in hk.pkl: `scripts/zizmor-gate.sh <workflow>...`.
#
# It exists to keep two outcomes apart that one `|| echo` could not. zizmor
# exits 1 when it could not run (a tool failure) and 10-14 when it audited and
# found something (by severity), so the verdict is read from the code:
#   0      clean         -> silence
#   >= 10  a finding     -> the finding message
#   other  never audited -> "zizmor could not run: <reason>"
# Both failures exit 1. Only the SECOND must never be reported as the first:
# a gate does not claim a finding from an audit it never made (`.:T110`).
#
# In a Claude Code cloud session (`CLAUDE_CODE_REMOTE=true`) `GH_TOKEN` and
# `GITHUB_TOKEN` hold the placeholder `proxy-injected`. zizmor sends it with
# its own git reads and GitHub answers 401, while the same read succeeds
# anonymously. So there, and only there, zizmor runs with both unset; if the
# online audits still cannot reach GitHub, it falls back to `--offline`. Local
# runs and GitHub CI keep the tokens and the full online audit.
set -u

readonly ZIZMOR=(zizmor --no-progress --persona=pedantic -c .github/zizmor.yml)
err=$(mktemp) || exit 1
trap 'rm -f "$err"' EXIT

# One run; stderr is kept for the reason, then shown. Written to a file and
# replayed rather than teed through a process substitution, which can still be
# writing when the next line reads it.
run() {
  "$@" 2> "$err"
  local rc=$?
  cat "$err" >&2
  return "$rc"
}

if [ "${CLAUDE_CODE_REMOTE:-}" = true ]; then
  run env -u GH_TOKEN -u GITHUB_TOKEN "${ZIZMOR[@]}" "$@"
  rc=$?
  if [ "$rc" -ne 0 ] && [ "$rc" -lt 10 ]; then
    echo 'hk: zizmor could not reach GitHub from this cloud session; auditing offline (the online audits are skipped here, and still run in CI).' >&2
    run env -u GH_TOKEN -u GITHUB_TOKEN "${ZIZMOR[@]}" --offline "$@"
    rc=$?
  fi
else
  run "${ZIZMOR[@]}" "$@"
  rc=$?
fi

if [ "$rc" -eq 0 ]; then
  exit 0
elif [ "$rc" -ge 10 ]; then
  echo 'hk: zizmor found a workflow security finding. Fix it, or -- if it is understood and deliberately accepted -- record it in .github/zizmor.yml with the reason AND the condition that would retire it. An ignore with no exit is a rule nobody revisits.' >&2
else
  # The cause is the last message before zizmor's backtrace, minus the
  # "N: " numbering of its cause chain; the frames below it name code, not
  # what failed.
  reason=$(sed '/^Stack backtrace:/,$d' "$err" | sed -E 's/^[[:space:]]*[0-9]+:[[:space:]]*//' | grep -v '^[[:space:]]*$' | tail -n 1)
  echo "hk: zizmor could not run: ${reason:-exit $rc, no message}. This is a TOOL failure, not a finding -- no audit was performed." >&2
fi
exit 1
