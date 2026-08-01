# DRY

Do not repeat yourself. Every piece of knowledge -- logic, constant,
structure, validation rule -- must have a single, authoritative source.
When the same idea appears in two places, one will inevitably drift.

Apply uniformly across implementation and test code:

- Extract shared logic into a named unit (function, module, helper)
  when two or more call sites duplicate the same behavior.
- Centralize constants, configuration values, and magic numbers at
  their single source of truth.
- In tests, use shared setup (fixtures, factories, helpers) for
  repeated arrangements; do not copy-paste setup blocks across cases.
- Prefer parameterized or table-driven tests over duplicated test
  bodies that differ only in input values.

Duplication is acceptable only when removing it would introduce
coupling between unrelated concerns or would obscure intent by
forcing a premature abstraction. The cost of wrong abstraction
exceeds the cost of local repetition.
