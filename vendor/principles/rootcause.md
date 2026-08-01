# Root-cause

Diagnose problems to their root cause before designing a fix. A symptom
patched leaves the disease; find the deepest "why" first, then fix THAT.

## Applying root-cause

- On a red test or bug, trace back through the chain of causes until you
  reach the one whose removal would prevent the entire failure class, not
  just this instance. Fix that layer.
- Ask "why" repeatedly. The first answer is usually a symptom ("the test
  timed out"); the useful answer is several levels deeper ("the retry
  loop has no backoff, so under load it saturates the connection pool").
- Before retrying a failed operation, understand why it failed. A blind
  retry that happens to pass is a hidden bug, not a fix.
- When a fix is proposed, check whether it addresses the cause or merely
  suppresses the symptom. Catching and swallowing an exception, adding a
  sleep, or skipping a check may silence the signal while the underlying
  defect persists.
- After identifying the root cause, consider whether a new invariant,
  guardrail, or test should encode the lesson so the same class of
  failure cannot recur.

## Signals of violation

- A failing test is re-run until it passes without investigating why it
  failed.
- An error is caught and silenced rather than traced to its origin.
- A bug fix addresses the crash site but not the data flow that produced
  the invalid state.
- Multiple patches accumulate around the same subsystem because each one
  treated a different symptom of the same underlying defect.
- A workaround is shipped with no ticket, comment, or plan to address
  the actual cause.

## When root-cause conflicts with urgency

Sometimes a hotfix must ship before the full diagnosis is complete. That
is acceptable when the blast radius demands it -- but the hotfix is not
the resolution. File the root-cause investigation as a follow-up, mark
the hotfix as temporary, and schedule the deeper fix. A deferred
root-cause is honest debt; a forgotten one is silent rot.
