use super::*;

#[test]
fn a_node_ceiling_comes_from_the_file_not_a_constant() {
    // Reads a file this repo has and the published crate excludes.
    crate::testrepo::dogfood(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        // .context-limits names src/tdd; the value is read, not assumed.
        let tdd = ceiling_for(root, &root.join("src/tdd")).unwrap();
        assert!(
            tdd > tokens::DEFAULT_NODE,
            "src/tdd is listed and should not fall back to the default: {tdd}"
        );
        // A new node under src inherits src's ceiling -- prefix matching, so
        // adding a node does not silently drop it to the global default.
        assert_eq!(
            ceiling_for(root, &root.join("src/nope")).unwrap(),
            ceiling_for(root, &root.join("src")).unwrap()
        );
        // A path sharing no listed prefix falls back, which is NOT "no limit".
        assert_eq!(
            ceiling_for(root, &root.join("docs")).unwrap(),
            tokens::DEFAULT_NODE
        );
    });
}

#[test]
fn verdict_reports_direction_and_distance() {
    assert_eq!(verdict(100, 500), Verdict::Fits { slack: 400 });
    assert_eq!(verdict(900, 500), Verdict::Over { by: 400 });
}

#[test]
fn the_root_ceiling_comes_from_its_spec_row_not_the_default() {
    // Reads a file this repo has and the published crate excludes.
    crate::testrepo::dogfood(|| {
        // `.context-limits` names the root `SPEC.md`, while a node is
        // addressed as `.` -- so the lookup has to bridge those two spellings
        // or the root silently falls to DEFAULT_NODE and reads as 5x over
        // (`.:V104`: absence must never read as a verdict).
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let c = ceiling_for(root, root).unwrap();
        assert!(
            c > tokens::DEFAULT_NODE,
            "root must resolve to its SPEC.md row, got the default: {c}"
        );
    });
}

#[test]
fn depth_selects_something_or_it_is_a_flag_that_lies() {
    // V105, and the test that would have caught B8 the day `Depth` was
    // introduced: same input, two settings, DIFFERENT output. `Rule` was
    // consulted nowhere, so the two branches agreed for the project's
    // whole life while §I advertised a choice.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rule = pack(root, root, Depth::Rule).unwrap();
    let all = pack(root, root, Depth::All).unwrap();
    assert!(
        rule.cost.tokens < all.cost.tokens,
        "rule {} must be cheaper than all {}",
        rule.cost.tokens,
        all.cost.tokens
    );
    assert!(all.text.contains("## \u{a7}B"), "all keeps the archive");
    assert!(
        !rule.text.contains("## \u{a7}B"),
        "rule drops §B -- history is not a rule"
    );
    assert!(
        !rule.text.contains("## \u{a7}R"),
        "rule drops §R -- a measurement is not a rule"
    );
    assert!(
        rule.text.contains("## \u{a7}V"),
        "rule keeps §V -- that is the point of it"
    );
}

#[test]
fn every_node_resolves_to_a_real_ceiling() {
    // Reads a file this repo has and the published crate excludes.
    crate::testrepo::dogfood(|| {
        // V104's second half, made mechanical: absence must never read as
        // permission. DEFAULT_NODE is 2,000 -- a NODE budget, impossible as a
        // CHAIN ceiling -- so a node landing on it means no row covers it,
        // directly or by prefix, and it would be gated against a number
        // nobody chose.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        for node in fed::discover(root) {
            let c = ceiling_for(root, &node).unwrap();
            assert_ne!(
                c,
                tokens::DEFAULT_NODE,
                "{} fell back to the node default -- give it a row in \
                 .context-limits, or a prefix that covers it",
                node.display()
            );
        }
    });
}

#[test]
fn a_ceiling_is_compared_at_the_boundary_not_near_it() {
    // T10 turns this comparison into an exit code, so off-by-one here is
    // a gate that fires on compliant nodes or misses drifting ones.
    assert_eq!(verdict(500, 500), Verdict::Fits { slack: 0 });
    assert_eq!(verdict(501, 500), Verdict::Over { by: 1 });
}

#[test]
fn chain_of_repo_root_is_at_least_the_root_spec() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(
        !fed::chain(root, root).is_empty(),
        "root SPEC.md must exist (V5)"
    );
}

/// `V5`: a node that is not there is an error, never its ancestors' pack
/// under its name. `pack(root, "nowhere")` returned the ROOT chain, so
/// `sherd lens nowhere` printed the root's rules as that node's (`B2`).
#[test]
fn a_dir_with_no_spec_is_an_error_not_its_ancestors_pack() -> Result<(), String>
{
    let r = crate::testrepo::TestRepo::new("lens-nowhere")?;
    let err = pack(r.path(), &r.path().join("nowhere"), Depth::Rule)
        .err()
        .ok_or("a missing node was packed")?;
    assert!(err.to_string().contains("nowhere"), "{err}");
    assert!(
        pack(r.path(), r.path(), Depth::Rule).is_ok(),
        "the root packs"
    );
    Ok(())
}
