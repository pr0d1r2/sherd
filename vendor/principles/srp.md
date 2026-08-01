# Single Responsibility Principle (SRP)

A module, class, or function should have one and only one reason to
change. Each unit of code is accountable to exactly one actor or
stakeholder concern.

## Applying SRP

1. Identify the actors -- who requests changes to this code?
2. If more than one actor can independently demand a change, split the
    module along those actor boundaries.
3. Name each resulting unit after its responsibility, not its
    implementation mechanism.

## Signals of violation

- A file that changes in unrelated PRs (coupling unrelated concerns).
- A class with methods that serve different stakeholders (e.g., report
  formatting mixed with business-rule validation).
- A function that both computes a value and produces a side effect.
- The word "and" in a module description ("parses config and sends
  alerts").

## Decomposition strategies

- **Extract class/module** -- move the secondary responsibility into its
  own unit; the original delegates to it.
- **Facade** -- when multiple small units need a unified entry point,
  compose them behind a facade that itself holds no logic.
- **Event/callback** -- decouple the trigger from the effect so each can
  change independently.

## SRP in practice

- Prefer many small files over few large ones.
- A commit that touches unrelated concerns in one file is a hint that
  the file violates SRP.
- SRP applies at every scale: function, class, module, service.
