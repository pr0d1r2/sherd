//! V22: inside a git work tree, a walk skips what git ignores. Every test
//! pairs the ignored case with the same tree WITHOUT the rule, so a walker
//! that finds nothing anywhere fails here too (V10).

use super::*;
use crate::testrepo::TestRepo;

fn labels(root: &Path, nodes: &[PathBuf]) -> Vec<String> {
    nodes.iter().map(|n| node_label(root, n)).collect()
}

/// `.:B15`, the shape #80 measured: a gitignored scratch dir holding a
/// checkout of another repository, SPEC.md and all.
#[test]
fn a_gitignored_scratch_checkout_is_not_a_node() {
    let r = TestRepo::new("fed-ignored").expect("fixture repo");
    r.write(".gitignore", "scratch/\n").expect("write");
    r.write("scratch/other/SPEC.md", "# SPEC\n").expect("write");
    r.commit("ignore scratch").expect("commit");
    let root = r.path();
    assert_eq!(labels(root, &discover(root)), vec!["."]);

    r.write(".gitignore", "").expect("write");
    assert_eq!(
        labels(root, &discover(root)),
        vec![".", "scratch/other"],
        "without the rule the same dir is a node"
    );
}

/// A SPEC.md ignored by its own name, in a directory git tracks.
#[test]
fn an_ignored_spec_file_does_not_make_its_dir_a_node() {
    let r = TestRepo::new("fed-ignored-file").expect("fixture repo");
    r.write("docs/README.md", "x\n").expect("write");
    r.write(".gitignore", "docs/SPEC.md\n").expect("write");
    r.commit("docs").expect("commit");
    r.write("docs/SPEC.md", "# SPEC\n").expect("write");
    let root = r.path();
    assert_eq!(labels(root, &discover(root)), vec!["."]);

    r.write(".gitignore", "").expect("write");
    assert_eq!(labels(root, &discover(root)), vec![".", "docs"]);
}

/// The fixed list is a floor, not a fallback: a TRACKED `vendor/` is still
/// not a node, and a tracked ordinary dir still is.
#[test]
fn the_fixed_list_holds_for_tracked_dirs() {
    let r = TestRepo::new("fed-floor").expect("fixture repo");
    r.write("vendor/dep/SPEC.md", "# SPEC\n").expect("write");
    r.write("src/SPEC.md", "# SPEC\n").expect("write");
    r.commit("tracked").expect("commit");
    let root = r.path();
    assert_eq!(labels(root, &discover(root)), vec![".", "src"]);
}

/// Outside a work tree git answers nothing, and discovery is what it was.
#[test]
fn outside_a_work_tree_the_walk_is_unchanged() {
    let root = std::env::temp_dir().join(format!(
        "sherd-fed-nogit-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for d in ["", "scratch", "target"] {
        std::fs::create_dir_all(root.join(d)).expect("mkdir");
        std::fs::write(root.join(d).join("SPEC.md"), "# SPEC\n")
            .expect("write");
    }
    std::fs::write(root.join(".gitignore"), "scratch/\n").expect("write");
    let found = labels(&root, &discover(&root));
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(found, vec![".", "scratch"]);
}

/// V11's child scan: an ignored dir is not a child `§F` must list.
#[test]
fn an_ignored_dir_is_not_a_missing_federation_row() {
    let r = TestRepo::new("fed-exhaustive").expect("fixture repo");
    r.write(".gitignore", "scratch/\n").expect("write");
    r.write("scratch/x", "x\n").expect("write");
    r.commit("ignore scratch").expect("commit");
    let root = r.path();
    let (_, missing) = find_exhaustive_violations(&[], root);
    assert!(missing.is_empty(), "{missing:?}");

    r.write(".gitignore", "").expect("write");
    let (_, missing) = find_exhaustive_violations(&[], root);
    assert_eq!(missing, vec![root.join("scratch")]);
}

/// V19: an ignored `.rs` file is not measured against a ceiling.
#[test]
fn an_ignored_rust_file_is_not_measured() {
    let r = TestRepo::new("fed-rust").expect("fixture repo");
    r.write(".gitignore", "scratch/\n").expect("write");
    r.write("scratch/big.rs", "fn x() {}\n").expect("write");
    r.write("src/lib.rs", "fn y() {}\n").expect("write");
    r.commit("ignore scratch").expect("commit");
    let root = r.path();
    assert_eq!(rust_files(root), vec![root.join("src/lib.rs")]);

    r.write(".gitignore", "").expect("write");
    assert_eq!(
        rust_files(root),
        vec![root.join("scratch/big.rs"), root.join("src/lib.rs")]
    );
}
