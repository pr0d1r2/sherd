//! The finding paths of `check` and `validate`: each one fails the verb it
//! lives in, asserted through that verb's exit code and not only through
//! the helper that counts it.

use super::*;
use crate::testrepo::TestRepo;

/// A citation naming a row that does not exist fails `check`
/// (`src/spec:V7`): a citation is a link, and a dead link is a finding.
#[test]
fn a_dangling_citation_fails_check() -> Result<(), String> {
    let r = TestRepo::new("cli-dangling")?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nx\n\n## \u{a7}V INVARIANTS\n\n\
         V1: see `src/nowhere:V9`\n",
    )?;
    assert_eq!(check(r.path()), ExitCode::from(1));
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nx\n\n## \u{a7}V INVARIANTS\n\nV1: holds\n",
    )?;
    assert_eq!(check(r.path()), ExitCode::SUCCESS, "the same tree, linked");
    Ok(())
}

/// `validate`'s drift half: a drifted slice is counted and named, and a
/// registry that exists and cannot be parsed counts one (`src/cli:V12`).
#[test]
fn validate_counts_drift_and_an_unparsable_registry() -> Result<(), String> {
    let r = TestRepo::new("cli-validate-drift")?;
    r.write("doc.md", "First paragraph.\n\nSecond.\n")?;
    r.write(".sherd-slices", "out.txt doc.md lead:1\n")?;
    r.write("out.txt", "not what the source produces\n")?;
    assert_eq!(validate_drift(r.path()), 1, "one drifted slice");
    assert_eq!(validate(r.path()), ExitCode::from(1));

    r.write(".sherd-slices", "out.txt doc.md\n")?;
    assert_eq!(validate_drift(r.path()), 1, "unparsable registry");
    Ok(())
}
