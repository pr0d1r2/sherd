# Interface Segregation Principle (ISP)

No client should be forced to depend on methods it does not use. Split
large interfaces into smaller, role-specific ones so that implementers
and consumers see only what they need.

## Applying ISP

1. List all methods/capabilities of the interface.
2. Group them by which clients actually call them.
3. Split into cohesive, role-specific interfaces -- one per client
    group.
4. Implementers compose the interfaces they support; clients depend only
    on the slice they consume.

## Signals of violation

- An interface with methods that many implementers leave as no-ops or
  throw "not implemented."
- A change to one method forces recompilation or retesting of clients
  that never call it.
- A mock in tests that stubs out most methods with placeholders.
- The word "fat" or "god" in an interface name or description.

## Decomposition strategies

- **Role interfaces** -- each interface represents one role the
  implementer plays (e.g., `Readable`, `Writable`, `Closeable` instead
  of one `Stream`).
- **Client-specific facades** -- expose a tailored view of a larger
  service to each consumer.
- **Adapter pattern** -- when a third-party type has a fat interface,
  wrap it in a thin adapter exposing only the needed slice.

## Balance

ISP targets unnecessary coupling. Do not split interfaces so aggressively
that every method becomes its own type -- group by cohesion (methods
that change together and serve the same role belong together).
