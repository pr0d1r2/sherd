# Reuse Before Building

Look for a well-maintained open-source solution before creating a custom
implementation. Prefer a proven fit over rebuilding an existing capability.

## Applying reuse before building

- Search the relevant ecosystem before designing. Compare established
  libraries, tools, standards, and platform capabilities that already solve
  the problem.
- Evaluate candidates on evidence: maintenance activity, release history,
  documentation, tests, security posture, license compatibility, community
  health, and adoption in comparable systems.
- Validate fit with a small spike or focused review. Confirm that the candidate
  meets the required behavior, integrates cleanly, and remains operable by the
  team.
- Account for total ownership cost. Include integration, upgrades, dependency
  risk, and operational support when comparing reuse with a custom
  implementation.
- Record the decision. Name the options considered and the concrete reason for
  adopting an existing solution, extending one, or building locally.
- Contribute generally useful improvements upstream when practical instead of
  maintaining a private fork or parallel implementation.

## Signals of violation

- Implementation begins without checking whether the ecosystem already
  provides the capability.
- A custom component duplicates a maintained library but offers no documented
  requirement that the library cannot meet.
- A candidate is rejected only because it originated outside the project.
- The comparison counts dependency cost but ignores the long-term maintenance
  cost of locally owned code.
- A new implementation claims to be simpler without testing integration with
  an existing solution.

## When a custom implementation is appropriate

Reuse is a preference, not an automatic choice. Build locally when available
options fail a concrete requirement, are unmaintained, introduce unacceptable
security or supply-chain risk, have incompatible licenses, impose
disproportionate complexity, or prevent necessary control of critical
behavior. Document the evidence and keep the custom scope as small as
possible. Reassess the decision when requirements or available solutions
change. See also [[kiss]], [[yagni]], and [[consequences]].
