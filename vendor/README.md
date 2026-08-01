Vendored from https://github.com/pr0d1r2/set-and-setting
(`set/skills/principles`), MIT. Copied 2026-08-01.

Selected by WHICH FAILURE THEY ADDRESS, not by topic. The first pass took only
the code-structure principles (kiss, dry, nih, pola, solid) and left Dalio's
out as "judgement-only" -- wrong, and wrong in the same way `plan::classify`
was wrong: sorted by surface form rather than by function.

Read against this repo's own §B log, three of the five measured failures are
named by the ones that were excluded:

  truth       a stub documenting itself as "sufficient for the test" is
              spinning a failure in writing
  reality     filtering `not_owns` as a path, inventing a `dir == "."` row
              shape -- acting on a wished-for data model
  rootcause   `sanitize_first_cell` stripping `/` so the violation vanishes
              is symptom-patching

KISS/DRY/SOLID address none of those. They shape correct code; our failures
were not code for the problem.

Still not vendored, because they need reasoning a one-line slice cannot carry
and belong with the supervisor (slice V6): believability, meritocracy,
openness, ownership, transparency, sync, process, progress, evolve, machine.

Sliced into `src/tdd/principles.txt` by `bbx slice`; `bbx slice --check` gates
the drift.
