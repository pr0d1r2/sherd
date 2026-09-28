use super::*;

fn doc(badges: &str, graph: &str) -> String {
    format!(
        "# t\n\n<!-- BEGIN badges -->\n{badges}<!-- END badges -->\n\ntext\n\n<!-- BEGIN graph -->\n{graph}<!-- END graph -->\n"
    )
}

fn blocks(badges: &str, graph: &str) -> Blocks {
    vec![
        ("badges".to_string(), badges.to_string()),
        ("graph".to_string(), graph.to_string()),
    ]
}

#[test]
fn a_document_where_every_block_matches_is_fresh() {
    let d = doc("A\n", "B\n");
    assert_eq!(apply(&d, &blocks("A\n", "B\n"), true), Outcome::Fresh);
    assert_eq!(apply(&d, &blocks("A\n", "B\n"), false), Outcome::Fresh);
}

/// The failure `.:B16` records: one block current, another four nodes
/// behind. A per-block checker that stopped at the first fresh one would
/// have reported this document clean for weeks.
#[test]
fn a_stale_second_block_is_reported_even_when_the_first_is_fresh() {
    let d = doc("A\n", "OLD\n");
    let Outcome::Stale(diff) = apply(&d, &blocks("A\n", "NEW\n"), true) else {
        unreachable!("a stale block must be reported")
    };
    assert!(diff.iter().any(|l| l == "stale: graph"));
    assert!(!diff.iter().any(|l| l == "stale: badges"));
}

#[test]
fn writing_replaces_every_stale_block_in_one_pass() {
    let d = doc("OLD\n", "OLD\n");
    let Outcome::Wrote(next) = apply(&d, &blocks("A\n", "B\n"), false) else {
        unreachable!("two stale blocks must be rewritten")
    };
    assert_eq!(next, doc("A\n", "B\n"));
    assert_eq!(apply(&next, &blocks("A\n", "B\n"), false), Outcome::Fresh);
}

#[test]
fn a_block_whose_markers_are_absent_is_named_as_such() {
    let d = "# t\n\n<!-- BEGIN badges -->\nA\n<!-- END badges -->\n";
    assert_eq!(apply(d, &blocks("A\n", "B\n"), true), Outcome::NoMarkers);
}
