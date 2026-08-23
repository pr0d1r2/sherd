# Security policy

## Reporting a vulnerability

Report privately, not in a public issue.

- Preferred: [GitHub private vulnerability
  reporting](https://github.com/pr0d1r2/sherd/security/advisories/new)
- Or email **pr0d1r2@gmail.com** with `sherd security` in the subject.

Include what you ran, what happened, and the input that triggered it. A
reproducing `SPEC.md` is worth more than a description of one.

Expect an acknowledgement within a week. If a report is valid, the fix and the
advisory go out together, and you are credited unless you ask otherwise.

## Supported versions

Pre-1.0, only the latest published version is supported. There are no
backports.

## What the attack surface actually is

Stated plainly, because this tool does more than read files and it would be
misleading to present it as though it did not.

**`sherd` runs commands and mutates your repository.** It shells out to
`git` — including `checkout -b` — and runs `cargo test`. It writes files,
including `SPEC.md` files and state under `.sherd-state/` and `.sherd-slices/`.
It does all of this in a loop driven by **the output of a language model**.

That combination is the thing worth attacking, and the thing worth reporting:

1. **Model output reaching a command.** Anything that lets generated text
   influence an argument vector, a branch name, a path, or a file that is
   later executed. This is the highest-value class here by a distance. A
   model is not a trusted input, and it is fed content from your repository —
   so a hostile `SPEC.md`, or a hostile source file, is an input to it.
2. **A write outside its lane.** `sherd` is supposed to touch specs, its
   own state, and branches it created. A path that lets it write elsewhere —
   traversal out of the repo root, a symlink followed, a spec section
   overwritten that the verb does not own — is a defect regardless of whether
   a model was involved.
3. **Prompt contents on the wire.** With `ollama` (a **default** feature)
   `sherd` POSTs slices of your spec and source to the endpoint named by
   `SHERD_ENDPOINT`. TLS is compiled in, so `https://` works and should be used
   when the endpoint is not on a machine you own. Over `http://` this is
   cleartext, and nothing constrains the endpoint to a LAN.
4. **Evidence that is not evidence.** `sherd land` asks for proof before a
   merge. A path that lets a check report success it did not observe — a
   fabricated test result, a skipped run reported as green — is a security
   defect here in the same sense a silently-passing gate is: it removes a
   control while appearing to apply it.

**What is structurally excluded** is shorter than for a pure reader:

- **No `unsafe`.** `unsafe_code = "forbid"`, so memory-safety bugs are not
  representable.
- **No cloud.** There is no API-key path and no hosted-model client. The
  endpoint is one you name.

Note this crate does **not** carry `itok`'s clippy deny-list — `unwrap`,
`panic` and indexing are not denied here. A panic on hostile input is a
defect worth reporting, but it is not currently prevented by lint.

## What is out of scope

- Choosing `http://` for an endpoint on your own machine and getting
  plaintext. That is the documented behaviour of the scheme you asked for.
- `sherd` modifying your repository when you asked it to. Creating
  branches and writing specs is the job; doing it *outside* the lane in
  point 2 is not.
- A model producing a wrong answer. That is a quality issue, and an ordinary
  issue is the right place for it.
- Anything requiring an attacker who already runs code as you.
