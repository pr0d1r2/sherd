use super::*;
use std::collections::BTreeMap;

const DOC: &str = "# SPEC\n\n## \u{a7}V INVARIANTS\n\nV1: a ! b\nV2: see V1\n\n## \u{a7}T TASKS\n\nid|status|task|cites\nT3|.|do it|V1\n";

#[test]
fn a_row_carries_its_section_and_its_line() {
    let r = rows(DOC);
    assert_eq!(r.len(), 3, "V1, V2, T3 -- and not the header row: {r:?}");
    let [v1, _, t3] = r.as_slice() else {
        unreachable!("three rows, and the pattern says so")
    };
    assert_eq!((v1.section, v1.id.as_str(), v1.line), ('V', "V1", 5));
    assert_eq!((t3.section, t3.id.as_str(), t3.line), ('T', "T3", 11));
    assert_eq!(t3.text, "T3|.|do it|V1", "the row is carried verbatim");
}

/// The header row of a `§T` table opens with `id`, and prose opens with
/// anything. Neither is a row, and reading one as a row would move it.
#[test]
fn only_a_line_an_id_opens_is_a_row() {
    for line in ["id|status|task|cites", "V without a number: x", "Vx|.|y"] {
        assert!(rows(line).is_empty(), "not a row: {line}");
    }
}

/// V10. The pair only earns its place where the two readings DIFFER, so
/// that is what this asserts: same ids, and a cell that still says
/// `T1-T3` where `milestones` has already expanded it to three numbers.
#[test]
fn a_milestone_cell_is_carried_as_written_while_its_claim_is_expanded() {
    let doc = "## \u{a7}T TASKS\n\n\
                   | id | scope | tasks | done-when |\n\
                   |----|-------|-------|-----------|\n\
                   | M1 | first | T1-T3 | all three |\n\
                   | M2 | second | T7, T9 | both |\n";
    let cells = milestone_cells(doc);
    assert_eq!(
        cells,
        vec![
            ("M1".to_string(), "T1-T3".to_string()),
            ("M2".to_string(), "T7, T9".to_string()),
        ]
    );

    let claimed = milestones(doc);
    let ids: Vec<&String> = claimed.iter().map(|(m, _)| m).collect();
    assert_eq!(ids, vec!["M1", "M2"], "the two readings pair up");
    assert_eq!(
        claimed.first().map(|(_, t)| t.clone()),
        Some(vec![1, 2, 3]),
        "expanded there, and not here"
    );
    assert!(milestone_cells("V1: not a milestone row").is_empty());
}

/// V9. A DETECTOR needs a positive case or a function that finds nothing
/// passes it (`src/fed:V10`), and it needs the negatives too, because
/// every false positive here REFUSES a migration.
#[test]
fn a_bracketed_table_row_is_seen_as_unreadable_and_a_real_one_is_not() {
    let doc = "# SPEC\n\n\
                   ## \u{a7}T TASKS\n\n\
                   | id | status | task | cites |\n\
                   | --- | --- | --- | --- |\n\
                   | T1 | . | first task | - |\n\
                   | T2 | x | second | - |\n";
    let found = unreadable_rows(doc);
    assert_eq!(found.len(), 2, "{found:?}");
    let [first, _] = found.as_slice() else {
        unreachable!("two, and the pattern says so")
    };
    assert_eq!(first.line, 7);
    assert_eq!(first.text, "| T1 | . | first task | - |");

    // Rows this grammar DOES read, and lines that only look like rows.
    assert!(unreadable_rows(DOC).is_empty(), "the readable dialect");
    for line in [
        // `M` belongs to `microlith::milestones`, which reads exactly
        // this table. Reporting it would refuse every spec that keeps a
        // milestone table -- the form the format documents.
        "| M1 | first milestone | T1-T3 | all three are done |",
        "| id | status | task | cites |",
        "| --- | --- | --- | --- |",
        // Two cells is a table of prose, not a row shape.
        "| T1 | a note |",
        // Not a leading-pipe table at all.
        "see T1 | elsewhere | in prose",
    ] {
        assert!(unreadable_rows(line).is_empty(), "not a row: {line}");
    }
}

fn homes() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("V1".to_string(), "src/fed".to_string()),
        ("V2".to_string(), "src/spec".to_string()),
    ])
}

/// The number survives the move; the owner is what gets added.
#[test]
fn a_citation_whose_target_moved_gains_that_node() {
    assert_eq!(
        requalify("V9: see V1 and V2", "src/spec", &homes()),
        "V9: see `src/fed:V1` and V2",
        "V2 stayed here, so its bare form still resolves"
    );
}

/// Backticks already around the id are the canonical form, so the owner
/// goes INSIDE them. Writing a second pair produces ``src/fed:V1``, which
/// microlith reads as prose and `check` then calls a dead link.
#[test]
fn an_already_quoted_id_keeps_one_pair_of_backticks() {
    assert_eq!(
        requalify("T1|.|do it|`V1`", "src/spec", &homes()),
        "T1|.|do it|`src/fed:V1`"
    );
}

/// Three non-citations, and each is a distinct way to corrupt a spec: the
/// row's own id is a declaration, a namespaced id already has an owner,
/// and a slash form names another repository.
#[test]
fn a_declaration_a_namespaced_id_and_a_foreign_rule_are_untouched() {
    for line in ["V1: a ! b", "x `src/tdd:V1` y", "z/V1"] {
        assert_eq!(
            requalify(line, "src/spec", &homes()),
            line,
            "must not rewrite: {line}"
        );
    }
}

/// A row that does not move needs no rewrite at all, and running the
/// rewrite twice must not stack owners (`src/adopt:V6`).
#[test]
fn requalifying_twice_changes_nothing_the_second_time() {
    let once = requalify("V9: see V1", "src/spec", &homes());
    assert_eq!(requalify(&once, "src/spec", &homes()), once);
}
