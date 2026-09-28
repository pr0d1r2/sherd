use super::*;

/// The output is the first thing `check` must accept, because a
/// generator whose output its own checker rejects has shipped a second
/// dialect (`.:V27` dogfooding, one verb over).
#[test]
fn a_scaffold_passes_the_checker_that_gates_every_other_spec() {
    let out = scaffold("src/lens", &["deep".to_string()]);
    assert!(
        check(&out).is_empty(),
        "our own checker rejects our own scaffold: {:?}",
        check(&out)
    );
}

#[test]
fn a_childless_directory_gets_no_federation_table() {
    let out = scaffold("src/leaf", &[]);
    assert!(!out.contains("\u{a7}F"));
    assert!(check(&out).is_empty());
}

/// Ids are monotonic and never reused, so a placeholder id is permanent.
/// The scaffold emits section headers and a table header, and not one
/// `V1`, `T1` or `B1`.
#[test]
fn a_scaffold_carries_no_ids_at_all() {
    let out = scaffold("src/x", &["a".to_string(), "b".to_string()]);
    for line in out.lines() {
        assert!(
            !line.starts_with("V1")
                && !line.starts_with("T1")
                && !line.starts_with("B1"),
            "a seeded id is permanent: {line}"
        );
    }
}

#[test]
fn every_child_gets_a_row_naming_what_it_does_not_own() {
    let out = scaffold("src", &["fed".to_string(), "lens".to_string()]);
    assert!(out.contains("fed|WHAT IT OWNS|WHAT IT DOES \u{22a5} OWN"));
    assert!(out.contains("lens|WHAT IT OWNS|WHAT IT DOES \u{22a5} OWN"));
}
