use super::*;

#[test]
fn propose_moves_a_single_node_row() {
    assert_eq!(
        propose("orphan check: SPEC w/o parent §F row"),
        Proposal::Move("fed")
    );
    assert_eq!(
        propose("tier select from sherd.toml"),
        Proposal::Move("tokens")
    );
}

#[test]
fn propose_decomposes_a_multi_node_row() {
    match propose("`§F`.tokens staleness, tier-tagged") {
        Proposal::Decompose(ns) => assert!(ns.len() > 1, "{ns:?}"),
        other => panic!("expected Decompose, got {other:?}"),
    }
}

#[test]
fn propose_keeps_a_row_with_no_node_vocabulary() {
    assert_eq!(
        propose("report the caveman finding upstream"),
        Proposal::Keep
    );
}
