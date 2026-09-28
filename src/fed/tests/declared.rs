use super::*;

/// The declared set is read from the `§F` TABLES, and this repository is
/// its own fixture: `src` is declared by root, `src/fed` by `src`, and
/// the root itself is always present as the home for an unclaimed row.
#[test]
fn the_declared_set_is_what_the_tables_name() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let homes = declared(root);
    let names: Vec<&str> = homes.iter().map(|h| h.node.as_str()).collect();
    assert!(names.contains(&"."), "root is always a home: {names:?}");
    assert!(names.contains(&"src"), "{names:?}");
    assert!(names.contains(&"src/fed"), "{names:?}");
    assert!(
        homes
            .iter()
            .any(|h| h.node == "src/fed" && !h.owns.is_empty()),
        "a home carries the lens that declared it"
    );
}

/// V12 says two rows must not name one dir, and `discover` walks a tree
/// where a node can be reached twice. The set is deduped so a row cannot
/// be offered two identical homes and called ambiguous.
#[test]
fn each_node_appears_once() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut names: Vec<String> =
        declared(root).into_iter().map(|h| h.node).collect();
    let before = names.len();
    names.dedup();
    assert_eq!(before, names.len(), "duplicate home: {names:?}");
}
