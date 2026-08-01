# Consequences

Weigh second- and third-order consequences, not just first-order. The
immediately pleasant first-order choice is often the wrong one once
downstream effects are counted.

## Applying consequences

- Before propagating a breaking change, probe its blast radius: trace
  every consumer, every downstream build, every dependent workflow. The
  first-order "green" that reddens ten downstream repos is a net red.
- Canary-then-widen. Roll a change to a single consumer (or a small
  subset) and observe second-order effects before fleet-wide
  propagation. A canary that fails is cheap; a fleet that fails is not.
- Storm-brake on fleet reddening. If multiple downstream signals go red
  after a change, halt propagation immediately -- do not push through
  hoping the next repo will be green. The second-order cost of continued
  rollout compounds faster than the first-order benefit of finishing.
- Separate first-order appeal from net outcome. A shortcut that saves
  ten minutes now but creates an hour of debugging later is not a gain.
  Evaluate the total cost across all orders, not the immediate reward.
- When two options look equivalent in the first order, let the second
  order break the tie. The choice with fewer downstream risks, fewer
  implicit dependencies, and fewer surprise consumers is the better
  choice -- even if it takes slightly longer up front.

## Signals of violation

- A change is shipped because it passes local checks, without tracing
  its effect on downstream consumers or dependent builds.
- A fleet-wide rollout proceeds after early canary failures because "most
  repos are fine."
- A shortcut is taken for first-order speed and the resulting second-
  order cleanup is left for someone else or deferred indefinitely.
- The blast radius of a breaking change is discovered after the fact,
  not before.
- A decision is defended by its first-order outcome while ignoring
  documented downstream regressions.

## When consequences conflicts with speed

Not every decision needs a full downstream trace. Low-blast-radius,
easily reversible changes (a typo fix, a local refactor with no public
API surface) can move fast without a multi-order analysis. The test: if
this change breaks something downstream, how hard is it to detect and
how expensive is it to revert? If detection is fast and reversion is
cheap, first-order reasoning is sufficient. If detection is slow or
reversion is costly, invest in the second-order analysis before
committing.
