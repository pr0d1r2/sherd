use super::*;

#[test]
fn a_module_named_by_no_row_is_not_a_candidate() {
    let spec = "## \u{a7}V INVARIANTS\n\nV1: the ledger counts fires\n";
    assert!(naming_rows(spec, "ledger").len() == 1);
    assert!(naming_rows(spec, "corpus").is_empty());
}

/// `src/split:B5`: the evidence grade discriminates in ONE of six repositories
/// measured. Everywhere else every module carries the same grade, which
/// leaves the spec rows as the only signal -- and alphabetical order
/// threw it away.
#[test]
fn proposals_lead_with_the_heaviest_not_the_alphabetically_first() {
    let spec = "## \u{a7}V INVARIANTS\n\
                    V1: `heavy` does a thing\nV2: `heavy` does another\n\
                    V3: `heavy` again\nV4: `light` once\n";
    let p = |name: &str, e: Evidence| Proposed {
        name: name.to_string(),
        evidence: e,
        members: vec![name.to_string()],
        shared: Vec::new(),
        split_layout: false,
    };
    let nodes = [
        p("light", Evidence::Published),
        p("heavy", Evidence::Published),
    ];
    let order: Vec<&str> = rank(&nodes, spec)
        .iter()
        .map(|r| r.node.name.as_str())
        .collect();
    assert_eq!(order, vec!["heavy", "light"], "heaviest first");
    assert_eq!(rank(&nodes, spec).first().map(|r| r.rows), Some(3));
}

/// A grade every module shares ranks nothing, and a reader who takes the
/// order for a verdict is reading spec rows rather than structure.
///
/// MEASURED: 4 of 6 repositories are perfectly uniform -- `sherd` all
/// directory, `ashlar` and `metope` all `pub mod`, `microlith` all
/// declared -- and only `itok` carries a mixed profile.
#[test]
fn a_grade_every_module_shares_is_reported_as_ranking_nothing() {
    let p = |name: &str, e: Evidence| Proposed {
        name: name.to_string(),
        evidence: e,
        members: vec![name.to_string()],
        shared: Vec::new(),
        split_layout: false,
    };
    assert!(uniform_evidence(&[
        p("a", Evidence::Published),
        p("b", Evidence::Published)
    ]));
    assert!(!uniform_evidence(&[
        p("a", Evidence::Drawn),
        p("b", Evidence::Published)
    ]));
    // One node ranks nothing either way, and saying so would be noise.
    assert!(!uniform_evidence(&[p("a", Evidence::Drawn)]));
    assert!(!uniform_evidence(&[]));
}

/// Whole-word, or every module matches every row: `plan` inside
/// "planning" and `check` inside "checked" made the first version
/// propose the entire spec for every candidate.
#[test]
fn a_name_matches_as_a_word_and_not_as_a_substring() {
    assert!(names_word("the plan is fixed", "plan"));
    assert!(!names_word("planning is not planning", "plan"));
    assert!(names_word("src/ledger.rs holds it", "ledger"));
    assert!(!names_word("no mention here", "ledger"));
}

/// The layout `.:§C` forbids, detected where the node is proposed: both
/// `<name>.rs` and `<name>/` exist, so neither half can carry a spec
/// until they are merged. `rekall` has it for `cli`.
#[test]
fn a_module_that_is_both_a_file_and_a_directory_is_flagged() {
    let dir = std::env::temp_dir().join(format!(
        "sherd-layout-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let src = dir.join("src");
    let Ok(()) = std::fs::create_dir_all(src.join("both")) else {
        unreachable!("a src dir is creatable")
    };
    let Ok(()) = std::fs::write(src.join("lib.rs"), "mod both;\nmod solo;\n")
    else {
        unreachable!("a lib.rs is writable")
    };
    let Ok(()) = std::fs::write(src.join("both.rs"), "") else {
        unreachable!("a module file is writable")
    };
    let Ok(()) = std::fs::write(src.join("solo.rs"), "") else {
        unreachable!("a module file is writable")
    };

    let found = structure(&dir);
    let flagged = |n: &str| {
        found
            .iter()
            .find(|p| p.name == n)
            .is_some_and(|p| p.split_layout)
    };
    assert!(flagged("both"), "both.rs and both/ exist: {found:?}");
    assert!(!flagged("solo"));
    let _ = std::fs::remove_dir_all(&dir);
}

/// A FEDERATED tree proposes the nodes it already has, graded
/// `directory`, and each still carries whatever weight the parent spec
/// gives it. `src/split:B3` is what the opposite assumption cost: excluding
/// already-nodes left the weight column reading zero for every one of
/// them, in the only kind of repository where the question matters.
#[test]
fn a_federated_tree_proposes_and_weighs_its_existing_nodes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let found = structure(root);
    assert!(
        found.iter().all(|p| p.evidence == Evidence::Drawn),
        "every module here is a directory: {found:?}"
    );
    let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
        unreachable!("this repository has a root spec")
    };
    let weighed = found
        .iter()
        .filter(|p| row_weight(&spec, &p.name).0 > 0)
        .count();
    assert!(weighed > 1, "root names its nodes on real rows");
}
