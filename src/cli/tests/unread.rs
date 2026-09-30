use super::*;
use crate::testrepo::TestRepo;

/// A root declaring node `a`, whose `SPEC.md` is not UTF-8 and so cannot be
/// read as text -- discovered, and unreadable.
pub(crate) fn unreadable_node(tag: &str) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nroot\n\n## \u{a7}F FEDERATION\n\n\
         dir|owns|\u{22a5}owns|tokens\na|alpha|-|-\n",
    )?;
    std::fs::create_dir_all(r.path().join("a")).map_err(|e| e.to_string())?;
    std::fs::write(r.path().join("a/SPEC.md"), b"# SPEC\n\n\xff\xfe\n")
        .map_err(|e| e.to_string())?;
    Ok(r)
}

/// `.:V48`: a node discovered and not read is a FAIL, never a skip. `check`
/// and `validate` both skipped it and reported `2 nodes examined · 0
/// violations`, exit 0 -- a clean bill for a tree they could not read
/// (`.:B31`).
#[test]
fn a_node_that_cannot_be_read_fails_check_and_validate() -> Result<(), String> {
    let r = unreadable_node("cli-unread")?;
    assert_eq!(check(r.path()), ExitCode::from(1));
    assert_eq!(validate(r.path()), ExitCode::from(1));

    // The same tree with the node readable passes both, so the failure
    // above is the unreadable file and nothing else in the fixture.
    r.write("a/SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nalpha\n")?;
    assert_eq!(check(r.path()), ExitCode::SUCCESS);
    assert_eq!(validate(r.path()), ExitCode::SUCCESS);
    Ok(())
}
