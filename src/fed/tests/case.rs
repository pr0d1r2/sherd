//! V23: a node's spec is named exactly `SPEC.md`. On a case-insensitive
//! filesystem `is_file()` says yes for `spec.md` too, so these tests are red
//! there before the fix and guards on a case-sensitive one.

use super::*;
use crate::testrepo::TestRepo;

/// `.:B16`, the shape found on the #80 consumer: a document that happens to
/// be called `spec.md`, deeper than the root.
#[test]
fn a_lowercase_spec_md_is_not_a_node() -> Result<(), String> {
    let r = TestRepo::new("fed-case")?;
    r.write("skills/pm/spec.md", "a skill doc\n")?;
    r.write("src/SPEC.md", "# SPEC\n")?;
    let root = r.path();
    assert!(!is_node(&root.join("skills/pm")));
    assert!(is_node(&root.join("src")), "the exact name is a node (V10)");
    let found: Vec<String> =
        discover(root).iter().map(|n| node_label(root, n)).collect();
    assert_eq!(found, vec![".", "src"]);
    Ok(())
}

/// The chain to a node passes through its ancestors; one carrying only a
/// lowercase `spec.md` is not one of them.
#[test]
fn a_lowercase_spec_md_is_not_in_a_chain() -> Result<(), String> {
    let r = TestRepo::new("fed-case-chain")?;
    r.write("a/spec.md", "not a spec\n")?;
    r.write("a/b/SPEC.md", "# SPEC\n")?;
    let root = r.path();
    let want = vec![root.join("SPEC.md"), root.join("a/b/SPEC.md")];
    assert_eq!(chain(root, &root.join("a/b")), want);
    Ok(())
}
