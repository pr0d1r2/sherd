//! Fixtures shared by more than one verb family's tests. Test-only.

use super::*;

pub(super) fn argv(s: &[&str]) -> Vec<String> {
    s.iter().map(|a| (*a).to_string()).collect()
}

/// One child node whose `§G` body is `goal` -- a query matches its words,
/// and anything after them is whatever the test needs next.
pub(super) fn write_spec(root: &Path, dir: &str, goal: &str) {
    let node = root.join(dir);
    let spec = format!("# SPEC\n\n## \u{a7}G GOAL\n\n{goal}\n");
    let Ok(()) = std::fs::create_dir_all(&node) else {
        unreachable!("a node dir is creatable")
    };
    let Ok(()) = std::fs::write(node.join("SPEC.md"), spec) else {
        unreachable!("a node spec is writable")
    };
}

/// A fixture with two child nodes, each carrying a `§G` a query can hit.
pub(super) fn routing_fixture(tag: &str) -> crate::testrepo::TestRepo {
    let Ok(repo) = crate::testrepo::TestRepo::new(tag) else {
        unreachable!("a fixture repository is buildable")
    };
    write_spec(repo.path(), "alpha", "widgets and sprockets");
    write_spec(repo.path(), "beta", "gizmos");
    repo
}
