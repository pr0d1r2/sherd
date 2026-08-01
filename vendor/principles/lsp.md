# Liskov Substitution Principle (LSP)

Subtypes must be substitutable for their base types without altering
the correctness of the program. If S is a subtype of T, then objects of
type T may be replaced with objects of type S without breaking any
property that callers depend on.

## Applying LSP

1. Define the contract of the base type -- preconditions,
    postconditions, invariants.
2. Every subtype must honor that contract: no stronger preconditions, no
    weaker postconditions, no broken invariants.
3. Callers code against the base contract and never need to check which
    subtype they received.

## Contract rules

- **Preconditions** -- a subtype may accept a wider range of inputs
  (weaker precondition) but must never require a narrower one.
- **Postconditions** -- a subtype may guarantee more (stronger
  postcondition) but must never promise less.
- **Invariants** -- properties that hold throughout the lifetime of the
  base type must also hold for the subtype.
- **History constraint** -- a subtype must not introduce state changes
  that the base type does not permit.

## Signals of violation

- Type-checking a subtype before calling a method (`instanceof`,
  `typeof` guards in generic code).
- A subtype that throws "not supported" for an inherited method.
- Overriding a method in a way that surprises callers (e.g., a
  `read-only` collection subclass that throws on `add`).
- Unit tests that pass for the base type but fail for a subtype on the
  same inputs.

## Remedies

- **Redesign the hierarchy** -- if a subtype cannot honor the contract,
  it is not truly a subtype; use composition or a separate interface.
- **Narrow the base contract** -- sometimes the base promises too much;
  tighten it and push optional behavior to a richer interface.
- **Favor composition over inheritance** -- wrap the existing type and
  expose only the operations that genuinely apply.
