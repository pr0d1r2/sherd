//! V22: inside a git work tree, a walk skips what git ignores. Every test
//! pairs the ignored case with the same tree WITHOUT the rule, so a walker
//! that finds nothing anywhere fails here too (V10).

use super::*;
use crate::testrepo::TestRepo;

fn labels(root: &Path, nodes: &[PathBuf]) -> Vec<String> {
    nodes.iter().map(|n| node_label(root, n)).collect()
}

/// A committed fixture: `ignore` as `.gitignore`, then `files` written after
/// the commit, so an ignored file is never staged.
fn tree(tag: &str, ignore: &str, files: &[&str]) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    r.write(".gitignore", ignore)?;
    r.write("docs/README.md", "x\n")?;
    r.commit("fixture")?;
    for f in files {
        r.write(f, "# SPEC\n")?;
    }
    Ok(r)
}

/// `.:B15`, the shape #80 measured: a gitignored scratch dir holding a
/// checkout of another repository, SPEC.md and all.
#[test]
fn a_gitignored_scratch_checkout_is_not_a_node() -> Result<(), String> {
    let r = tree("fed-ignored", "scratch/\n", &["scratch/other/SPEC.md"])?;
    let root = r.path();
    assert_eq!(labels(root, &discover(root)), vec!["."]);
    r.write(".gitignore", "")?;
    assert_eq!(labels(root, &discover(root)), vec![".", "scratch/other"]);
    Ok(())
}

/// A SPEC.md ignored by its own name, in a directory git tracks.
#[test]
fn an_ignored_spec_file_does_not_make_its_dir_a_node() -> Result<(), String> {
    let r = tree("fed-ignored-file", "docs/SPEC.md\n", &["docs/SPEC.md"])?;
    let root = r.path();
    assert_eq!(labels(root, &discover(root)), vec!["."]);
    r.write(".gitignore", "")?;
    assert_eq!(labels(root, &discover(root)), vec![".", "docs"]);
    Ok(())
}

/// The fixed list is a floor, not a fallback: a TRACKED `vendor/` is still
/// not a node, and a tracked ordinary dir still is.
#[test]
fn the_fixed_list_holds_for_tracked_dirs() -> Result<(), String> {
    let r = tree("fed-floor", "", &["vendor/dep/SPEC.md", "src/SPEC.md"])?;
    r.commit("tracked")?;
    let root = r.path();
    assert_eq!(labels(root, &discover(root)), vec![".", "src"]);
    Ok(())
}

/// Outside a work tree git answers nothing, and discovery is what it was.
#[test]
fn outside_a_work_tree_the_walk_is_unchanged() -> Result<(), String> {
    let r = tree(
        "fed-nogit",
        "scratch/\n",
        &["scratch/SPEC.md", "target/SPEC.md"],
    )?;
    std::fs::remove_dir_all(r.path().join(".git"))
        .map_err(|e| e.to_string())?;
    let root = r.path();
    assert_eq!(labels(root, &discover(root)), vec![".", "scratch"]);
    Ok(())
}

/// V11's child scan: an ignored dir is not a child `§F` must list.
#[test]
fn an_ignored_dir_is_not_a_missing_federation_row() -> Result<(), String> {
    let r = tree("fed-exhaustive", "scratch/\n", &["scratch/x"])?;
    let root = r.path();
    let (_, missing) = find_exhaustive_violations(&[], root);
    assert_eq!(missing, vec![root.join("docs")]);
    r.write(".gitignore", "")?;
    let (_, mut missing) = find_exhaustive_violations(&[], root);
    missing.sort();
    assert_eq!(missing, vec![root.join("docs"), root.join("scratch")]);
    Ok(())
}

/// V19: an ignored `.rs` file is not measured against a ceiling.
#[test]
fn an_ignored_rust_file_is_not_measured() -> Result<(), String> {
    let r = tree("fed-rust", "scratch/\n", &["scratch/big.rs", "src/lib.rs"])?;
    let root = r.path();
    assert_eq!(rust_files(root), vec![root.join("src/lib.rs")]);
    r.write(".gitignore", "")?;
    let all = vec![root.join("scratch/big.rs"), root.join("src/lib.rs")];
    assert_eq!(rust_files(root), all);
    Ok(())
}
