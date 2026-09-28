use super::*;

/// The defect `src/split:B3` names: this repository's root spec talks about `tdd`
/// on dozens of rows, and the promotable-candidate path reported zero
/// because `src/tdd` is already a node.
#[test]
fn an_existing_node_still_has_a_weight_in_its_parent() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
        unreachable!("this repository has a root spec")
    };
    let (rows, tokens) = row_weight(&spec, "tdd");
    assert!(rows > 0, "root names `tdd` on real rows");
    assert!(tokens > 0);

    // And a name nothing mentions weighs nothing.
    assert_eq!(row_weight(&spec, "wombat"), (0, 0));
}

/// `src/split:B4`, first cause, on the example that row names: lowercasing split
/// the filename `SPEC.md` into `spec`, so every row naming the FILE
/// counted as a row about the NODE. Measured on this repository's own
/// root spec the column read 54 where a case-sensitive count reads 27.
#[test]
fn a_filename_is_not_a_row_about_the_node_it_spells() {
    assert!(!names_word("the node carries `SPEC.md`", "spec"));
    assert!(names_word("`spec` owns the section split", "spec"));

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Ok(text) = std::fs::read_to_string(root.join("SPEC.md")) else {
        unreachable!("this repository has a root spec")
    };
    let counted = naming_rows(&text, "spec").len();
    let lowercased = text
        .lines()
        .filter(|l| !l.starts_with("## \u{a7}"))
        .filter(|l| {
            l.to_lowercase()
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|w| w == "spec")
        })
        .count();
    assert!(
        lowercased > counted,
        "the lowercasing counted more than the case-sensitive read: \
             {lowercased} against {counted}"
    );
    for row in naming_rows(&text, "spec") {
        assert!(
            names_word(row.as_str(), "spec"),
            "a row was counted only for spelling the FILE: {row}"
        );
    }
}

/// `src/split:B4`, second cause: `sync` GENERATES `§N` into every node and `§N`
/// names every sibling, so counting it makes this function read its own
/// generator's output and the loop never converges (`src/split:V3`). `§F` is
/// authored rather than generated and is excluded for the same reason --
/// it is structure, not law.
#[test]
fn generated_navigation_is_not_law_about_a_sibling() {
    let spec = "## \u{a7}N NAV\n\nrel|path|lens\nsib|src/tdd|the loop\n\n\
                    ## \u{a7}F FEDERATION\n\ndir|lens|grade\ntdd|the loop|HT\n\n\
                    ## \u{a7}V INVARIANTS\n\nV1: `tdd` refuses a green test\n";
    assert_eq!(
        naming_rows(spec, "tdd"),
        vec!["V1: `tdd` refuses a green test".to_string()],
        "only the §V row is law about `tdd`"
    );
}

/// The same, on the tree that measured it: `src/tdd/SPEC.md` names five
/// siblings and every one of those mentions is a generated `§N` row.
#[test]
fn a_leaf_spec_weighs_nothing_for_its_siblings() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Ok(text) = std::fs::read_to_string(root.join("src/tdd/SPEC.md")) else {
        unreachable!("this repository has a spec for `src/tdd`")
    };
    assert!(
        text.contains("sib|src/lens"),
        "the nav does list `lens` as a sibling"
    );
    for row in naming_rows(&text, "lens") {
        assert!(
            !row.starts_with("sib|")
                && !row.starts_with("up|")
                && !row.starts_with("rel|"),
            "a nav row was weighed as law about a sibling: {row}"
        );
    }
}
