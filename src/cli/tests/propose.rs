use super::super::fixtures::*;
use super::*;

/// `split` PROPOSES and never writes, which is the property worth
/// pinning: `--apply` refuses, and a federated node has nothing left to
/// promote so the table is empty rather than noise.
#[test]
fn split_proposes_and_refuses_to_apply() {
    let repo = routing_fixture("cli-split");
    let root = repo.path();
    // `alpha` and `beta` are nodes already; a flat module is not.
    let Ok(()) = std::fs::write(root.join("gamma.rs"), "pub fn gamma() {}\n")
    else {
        unreachable!("a module file is writable")
    };
    write_spec(root, "alpha", "widgets and sprockets");
    let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
        unreachable!("the fixture spec is readable")
    };
    let Ok(()) = std::fs::write(
        root.join("SPEC.md"),
        format!("{spec}\nV1: gamma holds the gamma rule\n"),
    ) else {
        unreachable!("the fixture spec is writable")
    };

    assert_eq!(split_cmd(root, root, true), ExitCode::from(2), "--apply");
    assert_eq!(split_cmd(root, root, false), ExitCode::SUCCESS);
    // Proposing must not have written anything.
    assert!(!root.join("gamma").exists(), "split created a directory");
}

/// The structure-first proposal on a fixture whose modules the spec
/// never names: `.:src/split:B1` is that a row ranking sees nothing here,
/// while the code plainly declares two nodes.
/// Every grade, including the bottom rung that always fires: a plain
/// `mod` and a `pub(crate) mod` are both DECLARED, which is what
/// `microlith` is made of (`.:src/split:B2`).
#[test]
fn a_private_or_crate_visible_module_is_still_a_node() {
    let repo = routing_fixture("cli-split-grades");
    let root = repo.path();
    let Ok(()) = std::fs::create_dir_all(root.join("src")) else {
        unreachable!("a src dir is creatable")
    };
    let Ok(()) = std::fs::write(
        root.join("src").join("lib.rs"),
        "pub mod api;\npub(crate) mod inner;\nmod hidden;\n",
    ) else {
        unreachable!("a lib.rs is writable")
    };
    let found = split::structure(root);
    let grade =
        |n: &str| found.iter().find(|p| p.name == n).map(|p| p.evidence);
    assert_eq!(grade("api"), Some(split::Evidence::Published));
    assert_eq!(
        grade("inner"),
        Some(split::Evidence::Declared),
        "pub(crate) is not published"
    );
    assert_eq!(grade("hidden"), Some(split::Evidence::Declared));
}

#[test]
fn split_proposes_nodes_the_spec_never_mentions() {
    let repo = routing_fixture("cli-split-structure");
    let root = repo.path();
    let Ok(()) = std::fs::create_dir_all(root.join("src")) else {
        unreachable!("a src dir is creatable")
    };
    let Ok(()) = std::fs::write(
        root.join("src").join("lib.rs"),
        "pub mod widget;\nmod helper;\n#[cfg(test)]\nmod testonly;\n",
    ) else {
        unreachable!("a lib.rs is writable")
    };
    let found = split::structure(root);
    let names: Vec<&str> = found.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        vec!["widget", "helper"],
        "published first, then declared: {found:?}"
    );
    assert_eq!(split_cmd(root, root, false), ExitCode::SUCCESS);
}

/// `src/split:V7` through the verb: a shell tree is proposed, rather than
/// answered with `nothing to propose`, including the ambiguous-name line.
#[test]
fn split_proposes_a_shell_tree() -> Result<(), String> {
    let repo = crate::testrepo::TestRepo::new("cli-split-shell")?;
    repo.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}V INVARIANTS\n\nV1: `run.sh` and `a/x/one.sh`\n",
    )?;
    for s in [
        "a/x/one.sh",
        "a/run.sh",
        "b/run.sh",
        "pr-a.sh",
        "pr-b.sh",
        "pr-c.sh",
    ] {
        repo.write(s, "#!/bin/sh\n")?;
    }
    repo.commit("shell")?;
    let root = repo.path();
    assert_eq!(split_cmd(root, root, false), ExitCode::SUCCESS);
    let names: Vec<String> =
        split::structure(root).into_iter().map(|p| p.name).collect();
    assert_eq!(names, vec!["a", "b", "pr"]);
    Ok(())
}

/// Where a candidate's scripts sit beneath it, busiest first and at most
/// five; nothing when they all sit in the candidate itself.
#[test]
fn script_dirs_names_the_busiest_dirs_beneath_a_node() {
    let node = Path::new("/r/scripts");
    let at = |p: &str| node.join(p);
    let deep: Vec<std::path::PathBuf> =
        ["just/t/1.sh", "just/t/2.sh", "lib/3.sh", "4.sh"]
            .iter()
            .map(|p| at(p))
            .collect();
    assert_eq!(script_dirs(&deep, node), " · in just/t 2, lib 1");
    assert_eq!(script_dirs(&[at("4.sh")], node), "");
    let many: Vec<std::path::PathBuf> =
        (0..7).map(|i| at(&format!("d{i}/x.sh"))).collect();
    assert_eq!(
        script_dirs(&many, node).matches(',').count(),
        4,
        "five shown"
    );
}

/// A node with no `SPEC.md` is a usage error, not an empty proposal.
#[test]
fn split_on_a_directory_with_no_spec_is_usage() {
    let repo = routing_fixture("cli-split-nospec");
    let bare = repo.path().join("bare");
    let Ok(()) = std::fs::create_dir_all(&bare) else {
        unreachable!("a dir is creatable")
    };
    assert_eq!(split_cmd(repo.path(), &bare, false), ExitCode::from(2));
}

/// A node declaring one public type and one private one, beside a
/// sibling node that declares none.
fn seam_fixture() -> crate::testrepo::TestRepo {
    let repo = routing_fixture("cli-seam");
    let Ok(()) = std::fs::write(
        repo.path().join("alpha").join("lib.rs"),
        "pub struct Shared {}\nstruct Hidden;\n",
    ) else {
        unreachable!("a module file is writable")
    };
    repo
}

/// The vocabulary a sibling can NAME. A private type is not one: a
/// parallel worker cannot write against a name it cannot spell.
#[test]
fn seam_names_a_public_type_and_not_a_private_one() {
    let repo = seam_fixture();
    let nodes = fed::discover(repo.path());
    let t = node_types(&repo.path().join("alpha"), &nodes);
    let named: Vec<&str> = t.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(named, vec!["Shared"], "a private type is not vocabulary");
}

/// Absence is an answer, not a failure (`V12`): a node with no types
/// adds no name a sibling has to know, and the verb still exits 0.
#[test]
fn a_node_declaring_no_types_reports_rather_than_erroring() {
    let repo = seam_fixture();
    let root = repo.path();
    let nodes = fed::discover(root);
    assert!(node_types(&root.join("beta"), &nodes).is_empty());
    assert_eq!(seam_cmd(root, root), ExitCode::SUCCESS, "absence is legal");
}

/// A file belongs to the NEAREST node. `fed::rust_files` recurses and
/// the nodes nest, so without that the root row would be the whole
/// crate and every type would be counted once per ancestor.
#[test]
fn a_type_belongs_to_the_nearest_node_not_to_every_ancestor() {
    let repo = seam_fixture();
    let nodes = fed::discover(repo.path());
    let at_root = node_types(repo.path(), &nodes);
    assert!(at_root.is_empty(), "alpha owns Shared: {at_root:?}");
}

/// A dir naming no node is a usage error rather than an empty report,
/// the shape `B5` records one verb over.
#[test]
fn seam_on_a_dir_matching_no_node_is_usage() {
    assert_eq!(run_args(argv(&["seam", "no-such-dir"])), ExitCode::from(2));
}

/// `wave` over a dir naming no node is usage, the same shape as `seam`.
#[test]
fn wave_on_a_dir_matching_no_node_is_usage() {
    assert_eq!(run_args(argv(&["wave", "no-such-dir"])), ExitCode::from(2));
}
