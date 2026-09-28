use super::*;

const DOC: &str = "# SPEC\n\n## \u{a7}G GOAL\n\ng\n\n## \u{a7}F FEDERATION\n\nf\n\n## \u{a7}V INVARIANTS\n\nv\n";

#[test]
fn an_absent_section_is_inserted_after_its_anchor() {
    let out = upsert_section(DOC, "N NAV", "## \u{a7}N NAV\n\nn\n", "F");
    assert!(out.contains("## \u{a7}N NAV"));
    let n = out.find("\u{a7}N").unwrap_or(0);
    let f = out.find("\u{a7}F").unwrap_or(0);
    let v = out.find("\u{a7}V").unwrap_or(0);
    assert!(f < n && n < v, "§N sits between §F and §V:\n{out}");
}

#[test]
fn an_existing_section_is_replaced_and_nothing_else_moves() {
    let once = upsert_section(DOC, "N NAV", "## \u{a7}N NAV\n\nold\n", "F");
    let twice = upsert_section(&once, "N NAV", "## \u{a7}N NAV\n\nnew\n", "F");
    assert!(twice.contains("new"));
    assert!(!twice.contains("old"));
    assert_eq!(twice.matches("\u{a7}N NAV").count(), 1, "one section only");
    assert!(twice.contains("## \u{a7}G GOAL\n\ng\n"), "§G untouched");
    assert!(
        twice.contains("## \u{a7}V INVARIANTS\n\nv\n"),
        "§V untouched"
    );
}

/// Writing twice changes nothing the second time, which is what lets
/// `sync` report "wrote" honestly.
#[test]
fn upserting_the_same_body_is_idempotent() {
    let once = upsert_section(DOC, "N NAV", "## \u{a7}N NAV\n\nn\n", "F");
    let twice = upsert_section(&once, "N NAV", "## \u{a7}N NAV\n\nn\n", "F");
    assert_eq!(once, twice);
}

#[test]
fn a_document_without_the_anchor_is_returned_unchanged() {
    let doc = "# SPEC\n\n## \u{a7}G GOAL\n\ng\n";
    assert_eq!(upsert_section(doc, "N NAV", "x", "F"), doc);
}
