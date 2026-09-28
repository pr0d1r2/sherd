use super::*;

const SPEC: &str = "\
## §I INTERFACES

- cmd: `sherd budget [dir]` → node/chain token table. exit 1 over
- cmd: `sherd route \"<query>\"` → dir + reason (0.3)
- cmd: `sherd validate` → DAG + ids + budget. exit 1 fail
- file: `SPEC.md` ∀ dir any depth
";

const DISPATCH: &str = "\
    match args.first().map(String::as_str) {
        Some(\"budget\") => budget(),
        Some(\"oneshot\") => oneshot(),
    }
";

/// Both halves of `.:B17`, in one fixture: `oneshot` ships unnamed, and
/// `validate` is promised with no rung and no implementation.
#[test]
fn drift_is_reported_in_both_directions() {
    let found = interface_drift(SPEC, DISPATCH);
    assert_eq!(
        found,
        vec![
            "`oneshot` dispatches and §I does not name it".to_string(),
            "`validate` is in §I, does not dispatch, and names no rung"
                .to_string(),
        ]
    );
}

/// A rung is the promise that makes an unbuilt verb legal in §I. Without
/// this, the only way to satisfy the rule would be to delete the plan.
#[test]
fn a_rung_marked_verb_may_be_unbuilt() {
    let rows = specced(SPEC);
    assert!(rows.contains(&("route".to_string(), Some("0.3".to_string()))));
    assert!(
        !interface_drift(SPEC, DISPATCH)
            .iter()
            .any(|d| d.contains("route"))
    );
}

#[test]
fn a_matched_interface_reports_nothing() {
    let spec = "- cmd: `sherd budget [dir]` → table\n";
    // The reader is scoped to the dispatch match, so a fixture without
    // one dispatches NOTHING -- which is a true answer to a different
    // question, and reported the verb as unbuilt until this line existed.
    let dispatch = "    match args.first().map(String::as_str) {\n        Some(\"budget\") => budget(),\n    }\n";
    assert!(interface_drift(spec, dispatch).is_empty());
}
