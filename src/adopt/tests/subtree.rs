use super::*;
use crate::testrepo::TestRepo;

/// `#104`'s fixture: `a` holds three rows and declares two empty children,
/// the root cites one of `a`'s rows by its namespaced form, and so does a
/// row already living in `a/x`.
fn subtree(tag: &str) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nroot\n\n## \u{a7}F FEDERATION\n\n\
         dir|owns|\u{22a5}owns|tokens\na|parser and renderer|the goal|-\n\
         b|release tagging|parsing|-\n\n\
         ## \u{a7}T TASKS\n\nid|status|task|cites\nT9|.|root work|`a:V1`\n",
    )?;
    r.write(
        "a/SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nparse and render\n\n\
         ## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n\
         x|parser, input, malformed rejection|rendering|-\n\
         y|rendering, output, colour|parsing|-\n\n\
         ## \u{a7}V INVARIANTS\n\n\
         V1: the parser rejects malformed input\n\
         V2: rendering output never mutates the tree, see V1\n\n\
         ## \u{a7}T TASKS\n\nid|status|task|cites\nT1|.|nested groups|V1\n",
    )?;
    r.write(
        "a/x/SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nx\n\n## \u{a7}T TASKS\n\n\
         id|status|task|cites\nT5|.|x work|`a:V2`\nT6|.|x more|`a:V1`\n",
    )?;
    r.write("a/y/SPEC.md", &spec::scaffold("a/y", &[]))?;
    r.write("b/SPEC.md", &spec::scaffold("b", &[]))?;
    Ok(r)
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| e.to_string())
}

/// V11: the source is `a/SPEC.md`, and every citation of a moved row
/// follows it -- in the source, in the receivers, and in a node the move
/// writes no row into.
#[test]
fn adopt_at_a_node_moves_its_rows_and_every_citation_follows()
-> Result<(), String> {
    let r = subtree("adopt-subtree")?;
    let report = apply_at(r.path(), "a", &read_map("V1 a/x\nV2 a/y")?)?;
    assert_eq!((report.rows_in, report.moved), (3, 2), "{report:?}");

    let a = read(r.path(), "a/SPEC.md")?;
    assert!(!a.contains("V1: the parser"), "V1 left a:\n{a}");
    assert!(a.contains("T1|.|nested groups|`a/x:V1`"), "{a}");

    let x = read(r.path(), "a/x/SPEC.md")?;
    assert!(x.contains("V1: the parser rejects"), "arrived:\n{x}");
    assert!(x.contains("T5|.|x work|`a/y:V2`"), "re-pointed:\n{x}");
    assert!(x.contains("T6|.|x more|V1\n"), "its own row, bare:\n{x}");

    let y = read(r.path(), "a/y/SPEC.md")?;
    assert!(y.contains("never mutates the tree, see `a/x:V1`"), "{y}");

    let root = read(r.path(), "SPEC.md")?;
    assert!(
        root.contains("T9|.|root work|`a/x:V1`"),
        "bystander:\n{root}"
    );
    Ok(())
}

/// V11, the proposal half: a home is a node BELOW the source, never the
/// root's other child -- and it places something (`src/fed:V10`).
#[test]
fn a_proposal_at_a_node_places_only_below_it() -> Result<(), String> {
    let r = subtree("adopt-subtree-propose")?;
    let p = propose_at(r.path(), "a")?;
    assert_eq!(p.rows_in, 3);
    assert!(!p.placements.is_empty(), "{p:?}");
    assert!(
        p.placements
            .iter()
            .all(|pl| pl.home == "a/x" || pl.home == "a/y"),
        "{p:?}"
    );
    Ok(())
}

/// The source is named when an id is missing from it, and a home outside
/// the source's subtree is refused even though the root declares it.
#[test]
fn a_map_naming_rows_or_homes_outside_the_node_is_refused() -> Result<(), String>
{
    let r = subtree("adopt-subtree-refuse")?;
    let refused = refusals_at(r.path(), "a", &read_map("V7 a/x\nV1 b")?);
    assert!(
        refused
            .iter()
            .any(|m| m.contains("the source, a/SPEC.md, declares no `V7`")),
        "{refused:?}"
    );
    assert!(
        refused.iter().any(|m| m.contains("`b` is not below `a`")),
        "{refused:?}"
    );
    Ok(())
}

/// `.` keeps today's reading: the root's spec is the source.
#[test]
fn adopt_at_the_root_reads_the_root_spec() -> Result<(), String> {
    let r = subtree("adopt-subtree-root")?;
    let refused = refusals_at(r.path(), ".", &read_map("V1 a/x")?);
    assert!(
        refused
            .iter()
            .any(|m| m.contains("the source, SPEC.md, declares no `V1`")),
        "the root holds no V1: {refused:?}"
    );
    Ok(())
}

/// The root's own namespaced form travels too: a row leaving `.` is cited
/// elsewhere as `.:V1`, and that citation follows it (V11).
#[test]
fn a_root_citation_follows_a_row_out_of_the_root() {
    let moved = BTreeMap::from([("V1".to_string(), "a/x".to_string())]);
    assert_eq!(
        spec::rehome("T2|.|y|`.:V1`,`.:V3`", "a/y", ".", &moved),
        "T2|.|y|`a/x:V1`,`.:V3`"
    );
    assert_eq!(
        spec::rehome("T2|.|y|`.:V1`", "a/x", ".", &moved),
        "T2|.|y|V1"
    );
}
