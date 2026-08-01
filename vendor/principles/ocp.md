# Open/Closed Principle (OCP)

Software entities should be open for extension but closed for
modification. Add new behavior by writing new code, not by changing
existing, tested code.

## Applying OCP

1. Identify the axis of change -- what varies between use cases?
2. Abstract that axis behind a stable interface or extension point.
3. New variants implement the interface; existing code remains
    untouched.

## Extension mechanisms

- **Polymorphism** -- define a base type or interface; each variant is a
  subtype that implements it. Callers depend on the abstraction.
- **Strategy/plugin** -- accept a function or configuration that
  customizes behavior without modifying the host.
- **Decoration/wrapping** -- layer additional behavior around an
  existing unit without altering its internals.
- **Configuration** -- externalize decisions (feature flags, dependency
  injection) so the runtime path varies without source changes.

## Signals of violation

- A switch/case or if-else chain that grows with every new variant.
- A change request that requires editing multiple existing files that
  were previously stable.
- Shotgun surgery -- a single logical change scattered across many
  modules.

## Boundaries

OCP does not mean "never edit code." It means design for anticipated
change axes so that the common case of adding a variant does not require
modifying existing, proven logic. When requirements shift in
unanticipated ways, refactor first to create the extension point, then
extend.
