# Principle of Least Astonishment (POLA / POLS)

Also called the Principle of Least Surprise (POLS). Every interface --
function signature, API endpoint, CLI flag, config option, class method,
return value -- should behave the way its user
would reasonably expect given its name, context, and the conventions of
the surrounding system. When two interpretations are possible, choose
the one that surprises fewer people.

## Applying POLA

- Name things by what they do, not how they do it. A reader who knows
  only the name should predict the behavior correctly.
- Follow platform and language idioms. A Go function returning
  `(value, error)` or a shell command exiting non-zero on failure meets
  expectations; deviating from these patterns forces every caller to
  learn a special case.
- Make defaults safe and unsurprising. Dangerous operations require
  explicit opt-in; the zero-value or omitted-flag path does the least
  harmful thing.
- Preserve symmetry. If `open` has a `close`, if `add` has a `remove`,
  if `start` has a `stop`, their contracts should mirror each other in
  scope, side effects, and error behavior.
- Localize side effects. A function called `validate` that silently
  mutates state violates POLA. A function called `normalize` may mutate,
  because the name signals it.

## Signals of violation

- Users repeatedly misuse an interface the same wrong way -- the fault
  is the interface, not the users.
- Documentation must explain why something does not do what its name
  suggests.
- Code reviewers ask "why does this return X here?" without reading the
  implementation.
- A boolean parameter inverts its meaning depending on context or
  caller.
- Changing a default would break nothing but would fix confusion.

## When a surprise is necessary

Sometimes the correct behavior genuinely violates expectations (security
constraints, backward compatibility, performance trade-offs). In those
cases, make the surprise visible: document it at the call site, name it
explicitly, or surface a warning. A justified surprise is still a
surprise -- compensate with clarity.
