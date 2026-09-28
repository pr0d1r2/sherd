use super::*;

#[test]
fn a_literal_pattern_matches_only_itself() {
    assert!(matches("Cargo.toml", "Cargo.toml"));
    assert!(!matches("Cargo.toml", "dev/Cargo.toml"));
    assert!(!matches("Cargo.toml", "Cargo.lock"));
}

#[test]
fn a_prefix_pattern_matches_the_dir_and_below() {
    assert!(matches("dev/src/**", "dev/src/badge.rs"));
    assert!(matches("dev/src/**", "dev/src/nested/x.rs"));
    assert!(matches("dev/src/**", "dev/src"));
    assert!(!matches("dev/src/**", "dev/tests/cli.rs"));
    assert!(!matches("dev/src/**", "devsrc/x"));
}

#[test]
fn a_suffix_pattern_matches_the_name_at_any_depth() {
    assert!(matches("**/SPEC.md", "SPEC.md"));
    assert!(matches("**/SPEC.md", "src/fed/SPEC.md"));
    assert!(!matches("**/SPEC.md", "src/fed/SPEC.why.md"));
    assert!(!matches("**/SPEC.md", "NOTSPEC.md"));
}

/// The mapping that earned this file: touching a node's spec moves the
/// diagram and the node count, and touching a ratchet moves only the
/// badges.
#[test]
fn a_spec_change_selects_the_graph_and_a_ratchet_does_not() {
    let all = selected(&["src/fed/SPEC.md".to_string()]);
    assert_eq!(
        all,
        vec!["badges", "graph-tree", "graph-mermaid", "graph-table"]
    );

    let ratchet = selected(&[".coverage".to_string()]);
    assert_eq!(ratchet, vec!["badges"]);
}

#[test]
fn a_change_touching_no_input_selects_nothing() {
    assert!(selected(&["src/lens/mod.rs".to_string()]).is_empty());
    assert!(selected(&["docs/SECURITY.md".to_string()]).is_empty());
}

/// No scope means the wide run: `hk check --all`, CI, and the pre-push
/// layer all arrive here with nothing to narrow by, and must compare
/// every block rather than none.
#[test]
fn no_scope_selects_every_block() {
    assert_eq!(selected(&[]).len(), BLOCKS.len());
}
