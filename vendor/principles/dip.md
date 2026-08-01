# Dependency Inversion Principle (DIP)

High-level modules should not depend on low-level modules. Both should
depend on abstractions. Abstractions should not depend on details;
details should depend on abstractions.

## Applying DIP

1. Identify the high-level policy (business logic, orchestration).
2. Identify the low-level detail (I/O, database, network, framework
    glue).
3. Define an abstraction (interface, protocol, type class) owned by the
    high-level layer.
4. The low-level layer implements that abstraction; the high-level layer
    depends only on it.

## Ownership rule

The abstraction belongs to the consumer, not the provider. The
high-level module declares what it needs; the low-level module conforms.
This inverts the traditional dependency direction -- details point
inward toward policy, not the other way around.

## Signals of violation

- Importing a concrete implementation (database driver, HTTP client)
  directly in business logic.
- High-level modules that break when a low-level library is swapped.
- Test setup that requires spinning up infrastructure (database, queue,
  network) to test business rules.
- Circular dependencies between layers.

## Inversion mechanisms

- **Constructor/parameter injection** -- pass dependencies in rather
  than creating them internally.
- **Interface/trait/protocol** -- define the contract in the consumer's
  layer; the provider implements it.
- **Module boundaries** -- in languages without interfaces, invert via
  module structure: core module defines types, adapter modules
  implement.
- **Configuration** -- wire dependencies at the composition root
  (main/entry point), not at the usage site.

## Practical scope

Apply DIP at architectural boundaries (business logic vs. persistence,
domain vs. transport). For leaf utilities or pure functions with no
external dependencies, indirection adds complexity with no benefit.
