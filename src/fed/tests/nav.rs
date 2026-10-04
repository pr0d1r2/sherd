use super::*;

#[test]
fn the_root_has_no_up_and_no_siblings() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rows = nav(root, root);
    assert_eq!(rows.len(), 2, "{rows:?}");
    let shape: Vec<(&str, &str)> = rows
        .iter()
        .map(|r| (r.rel.as_str(), r.path.as_str()))
        .collect();
    assert_eq!(shape, vec![("up", "-"), ("self", ".")]);
}

/// V34: one `self`, an `up` per ancestor, a `sib` per co-child. The lens
/// is the parent's own words about that child (V38), never re-described.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one nav table, four properties -- one self, an up per \
                  ancestor, no self among the siblings, and a sibling \
                  carrying its parent's lens. Split, each half would rebuild \
                  the same table to assert one of them"
)]
fn a_leaf_names_its_ancestors_itself_and_its_co_children() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rows = nav(root, &root.join("src").join("fed"));

    let self_rows: Vec<&Nav> =
        rows.iter().filter(|r| r.rel == "self").collect();
    assert_eq!(self_rows.len(), 1, "exactly one self: {rows:?}");
    assert_eq!(self_rows.first().map(|r| r.path.as_str()), Some("src/fed"));

    let ups = rows.iter().filter(|r| r.rel == "up").count();
    assert_eq!(ups, 2, "root and src: {rows:?}");

    let sibs: Vec<&Nav> = rows.iter().filter(|r| r.rel == "sib").collect();
    assert!(!sibs.is_empty(), "src has other children");
    assert!(sibs.iter().all(|s| s.path != "src/fed"), "no self as sib");
    assert!(
        sibs.iter()
            .any(|s| s.path == "src/lens" && !s.lens.is_empty()),
        "a sibling carries its parent's lens: {sibs:?}"
    );
}

/// V16. A lens is a `§F` CELL, so it is written back as one: a `|` the
/// parser unescaped is re-escaped, or the `§N` row gains a column (`B13`).
/// Round-trips through `split_row`, the parser every reader uses.
#[test]
fn a_lens_holding_a_pipe_stays_one_cell() {
    for lens in ["depth `rule`|`why`", r"C:\path", r"tail\", r"a\|b"] {
        let rows = vec![Nav {
            rel: "sib".into(),
            path: "src/lens".into(),
            lens: lens.into(),
        }];
        let section = nav_section(&rows);
        let row = section.lines().last().unwrap_or_default();
        assert_eq!(
            split_row(row),
            vec!["sib".to_string(), "src/lens".into(), lens.into()],
            "{row}"
        );
    }
}

/// V16 / B17 (#98): a cell authored in the shortest form is written back
/// BYTE FOR BYTE, so `§N` spells a lens exactly as its `§F` does (V21).
/// This replaces a test that pinned the opposite -- microlith before
/// 0.7.4 doubled every backslash, `x \& y` became `x \\& y`, and the pin
/// called that a decision. A backslash `split_row` would spend (before
/// `\`, before `|`, at the end) is still doubled, and every case
/// decodes back to itself.
#[test]
fn a_shortest_form_cell_is_copied_byte_for_byte() {
    for (cell, written) in [
        (r"x \& y", r"x \& y"),
        (r"C:\path", r"C:\path"),
        (r"tail\", r"tail\\"),
        (r"a|b", r"a\|b"),
    ] {
        assert_eq!(escape_cell(cell), written, "{cell}");
        assert_eq!(split_row(&escape_cell(cell)), vec![cell.to_string()]);
    }
}

#[test]
fn a_section_renders_with_its_header() {
    let rows = vec![Nav {
        rel: "self".into(),
        path: ".".into(),
        lens: "-".into(),
    }];
    assert_eq!(
        nav_section(&rows),
        "## \u{a7}N NAV\n\nrel|path|lens\nself|.|-\n"
    );
}

/// The walk finds nested files and skips what is not source: a build
/// product is not code, and a ceiling over `target/` measures the
/// compiler. Sorted, so a report is stable between runs.
#[test]
fn the_walk_finds_nested_source_and_skips_build_output() {
    let dir =
        std::env::temp_dir().join(format!("sherd-walk-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["src/deep", "target/debug", ".git"] {
        let _ = std::fs::create_dir_all(dir.join(sub));
    }
    for f in [
        "src/a.rs",
        "src/deep/b.rs",
        "target/debug/c.rs",
        ".git/d.rs",
        "src/notes.md",
    ] {
        let _ = std::fs::write(dir.join(f), "fn f() {}\n");
    }
    let found: Vec<String> = rust_files(&dir)
        .iter()
        .filter_map(|p| Some(p.strip_prefix(&dir).ok()?.display().to_string()))
        .collect();
    assert_eq!(found, vec!["src/a.rs", "src/deep/b.rs"], "{found:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two nodes, one NESTED inside the other, plus a subdirectory that is no
/// node at all. Handed to the test rather than discovered (`src/cli:V6`).
fn nested_nodes(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("sherd-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for sub in ["src/fed", "src/loose"] {
        let _ = std::fs::create_dir_all(dir.join(sub));
    }
    for f in ["src/lib.rs", "src/fed/mod.rs", "src/loose/x.rs"] {
        let _ = std::fs::write(dir.join(f), "fn f() {}\n");
    }
    dir
}

/// What a node owns, relative to the tree, so the assertion reads the way
/// the directories do.
fn rel_owned(dir: &Path, node: &Path, nodes: &[PathBuf]) -> Vec<String> {
    owned_rust_files(node, nodes)
        .iter()
        .filter_map(|p| Some(p.strip_prefix(dir).ok()?.display().to_string()))
        .collect()
}

/// V15. The nodes NEST, so an unattributed walk hands `src` every
/// sibling's file and the root the whole crate -- and a report built on
/// that claims the root declares every type and depends on everything.
#[test]
fn a_file_belongs_to_its_nearest_node_and_to_no_ancestor() {
    let dir = nested_nodes("owned");
    let nodes = vec![dir.join("src"), dir.join("src/fed")];
    assert_eq!(
        rel_owned(&dir, &dir.join("src"), &nodes),
        vec!["src/lib.rs", "src/loose/x.rs"],
        "a deeper node's file is not its parent's, and a subdirectory \
             that is no node still reaches the node above it"
    );
    assert_eq!(
        rel_owned(&dir, &dir.join("src/fed"), &nodes),
        vec!["src/fed/mod.rs"]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// The root is `.`, not the empty string it strips to, and a path from
/// another tree is left alone rather than silently relabelled.
#[test]
fn a_node_label_is_relative_and_the_root_is_a_dot() {
    let root = Path::new("/tmp/sherd-label");
    assert_eq!(node_label(root, root), ".");
    assert_eq!(node_label(root, &root.join("src/fed")), "src/fed");
    assert_eq!(node_label(root, Path::new("/elsewhere")), "/elsewhere");
}
