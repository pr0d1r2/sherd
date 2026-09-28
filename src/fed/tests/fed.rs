use super::*;
use crate::testrepo::TestRepo;

const F: &str = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\nsrc|code nodes|scripts, docs|1200\nscripts|inference harness|rust code|-\n\n## \u{a7}V INVARIANTS\nV1: x\n";

/// V18. Three properties, one fixture, because they are three answers to
/// the same question and a tree per answer would say less: a name resolves
/// to the node carrying it, the spelling just TRIED is never offered back,
/// and a name two nodes carry stays ambiguous rather than being guessed.
#[test]
fn a_node_is_findable_by_the_name_a_reader_typed() {
    let r = TestRepo::new("fed-spelled").expect("fixture repo");
    for n in ["src", "src/code", "dev/tools/code"] {
        r.write(&format!("{n}/SPEC.md"), "# SPEC\n").expect("write");
    }
    let root = r.path();

    // `seam code` from `src/`: the miss `src/cli:B9` left honest and
    // unhelpful. Two nodes end in `code`, so there is no single answer.
    let both = spelled(root, Path::new("code"));
    let names: Vec<String> = both.iter().map(|n| node_label(root, n)).collect();
    assert_eq!(names, vec!["dev/tools/code", "src/code"], "{both:?}");

    // One more component and it is unambiguous.
    let one = spelled(root, Path::new("tools/code"));
    assert_eq!(
        one.iter().map(|n| node_label(root, n)).collect::<Vec<_>>(),
        vec!["dev/tools/code"]
    );

    // The whole relative path names a node exactly -- which is the
    // spelling that WORKS, so it is never reached as a miss, and handing
    // it back as a suggestion would be advice to retype what was typed.
    assert!(spelled(root, Path::new("src/code")).is_empty());
    assert!(spelled(root, Path::new("")).is_empty());
}

#[test]
fn parses_rows_and_stops_at_next_section() {
    let e = edges(F);
    let [first, second] = e.as_slice() else {
        unreachable!("two rows, and the pattern says so")
    };
    assert_eq!(first.dir, "src");
    assert_eq!(first.not_owns, "scripts, docs");
    assert_eq!(first.tokens, Some(1200));
    assert_eq!(second.tokens, None, "`-` means unrecorded, not zero");
}

/// V13: through `edges()`, not through the splitter alone -- the row that
/// vanished did so because `edges()` drops anything that is not four
/// cells, and a splitter test cannot see that.
#[test]
fn a_backslash_in_a_cell_survives_the_parse() {
    // B11. `split_row` consumed `\` before ANY character, so an `owns`
    // naming a Windows path or a regex lost it silently, and the §F table
    // read as though the author had written something else.
    let t = table_with("a", "C:\\path notes");
    let es = edges(&t);
    assert_eq!(es.len(), 1, "one row in, one edge out");
    assert_eq!(
        es.first().map(|e| e.owns.as_str()),
        Some("C:\\path notes"),
        "the backslash is literal here -- V4 escapes only before a pipe"
    );
}

#[test]
fn a_cell_ending_in_a_backslash_still_yields_its_edge() {
    // The severe half of B11: `tail\\` split the row into five cells,
    // V1 says four cells or not a row, and `edges()` dropped it -- so a
    // federation edge disappeared with no error at all.
    //
    // B12 is the other side of the same cell, and this assertion is
    // where it surfaced: the ROW carries `tail\\`, which DECODES to one
    // backslash. Asserting the encoded form here is what made the first
    // fix look correct while the column break was being swallowed.
    let t = table_with("c", "tail\\\\");
    let es = edges(&t);
    assert_eq!(es.len(), 1, "the edge must not VANISH");
    assert_eq!(es.first().map(|e| e.owns.as_str()), Some("tail\\"));
}
#[test]
fn an_escaped_pipe_is_still_one_cell() {
    // The behaviour V4 always documented, and the one the fix must not
    // break: `\|` is a literal pipe inside the cell, never a column.
    let t = table_with("b", "rule\\|why");
    let es = edges(&t);
    assert_eq!(es.first().map(|e| e.owns.as_str()), Some("rule|why"));
}

#[test]
fn every_row_of_a_mixed_table_survives() {
    // The POSITIVE case V10 asks for, over all three shapes at once: a
    // count is what caught B11 (two of three rows survived), and a
    // per-row assertion would have passed on the two that did.
    let t = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\n\
                 a|C:\\path|-|10\nb|rule\\|why|-|20\nc|tail\\\\|-|30\n";
    let dirs: Vec<String> = edges(t).into_iter().map(|e| e.dir).collect();
    assert_eq!(dirs, vec!["a", "b", "c"], "no row may vanish");
}

/// One §F table with a single row, so each test states only its own cell.
fn table_with(dir: &str, owns: &str) -> String {
    format!(
        "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\n\
             {dir}|{owns}|-|10\n"
    )
}
#[test]
fn escaped_pipe_stays_in_the_cell() {
    let t = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\na|rule\\|why|-|10\n";
    assert_eq!(
        edges(t).first().map(|e| e.owns.clone()),
        Some("rule|why".to_string())
    );
}

#[test]
fn supervisor_dirs_never_become_nodes() {
    // A SPEC.md under .claude/ would otherwise be discovered, chained, and
    // shipped into a worker prompt (V13).
    for d in [".claude", ".github", ".codex", ".githooks"] {
        assert!(is_ignored_dir(d), "{d} must never be walked");
    }
    let found = discover(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(
        !found
            .iter()
            .any(|p| p.to_string_lossy().contains("/.claude")),
        "supervisor assets leaked into discovery: {found:?}"
    );
}

#[test]
fn mermaid_is_derived_from_f_rows() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let m = mermaid(root);
    assert!(m.starts_with("graph TD"), "{m}");
    assert!(m.contains("root --> src\n"), "root must point at src: {m}");
    assert!(
        m.contains("src --> src_tdd"),
        "src must point at its children: {m}"
    );
}

#[test]
fn mermaid_uses_the_widely_supported_directive() {
    let m = mermaid(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(
        m.starts_with("graph TD"),
        "`flowchart` is not in older mermaid: {m}"
    );
    for l in m.lines().filter(|l| l.contains('[')) {
        let inner = l.split_once('[').unwrap().1.trim_end_matches(']');
        assert!(
            inner.chars().all(|c| c.is_ascii_alphanumeric()
                || c == ' '
                || c == '-'
                || c == '_'),
            "label has syntax-significant chars: {l}"
        );
    }
}

#[test]
fn tree_nests_and_needs_no_renderer() {
    let tr = tree(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(tr.starts_with(".\n"), "{tr}");
    assert!(tr.contains("-- src\n"), "root child: {tr}");
    assert!(
        tr.contains("    |-- tokens") || tr.contains("|   |-- tokens"),
        "grandchild must be indented: {tr}"
    );
    assert!(tr.is_ascii(), "must be ascii: {tr}");
}

#[test]
fn table_escapes_the_delimiter() {
    assert_eq!(cell("rule|why"), "rule\\|why");
}

#[test]
fn mermaid_is_plain_and_ascii() {
    let m = mermaid(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(!m.contains("<br"), "no HTML: GitLab disables htmlLabels");
    assert!(!m.contains("<i>"), "no HTML: GitLab disables htmlLabels");
    assert!(m.is_ascii(), "non-ascii in diagram: {m}");
    for l in m.lines().filter(|l| l.contains('[')) {
        let inner = l.split_once('[').unwrap().1;
        assert_eq!(inner.matches('[').count(), 0, "nested bracket: {l}");
    }
}

#[test]
fn mermaid_declares_each_node_once() {
    let m = mermaid(Path::new(env!("CARGO_MANIFEST_DIR")));
    let mut ids: Vec<&str> = m
        .lines()
        .filter(|l| l.contains('['))
        .filter_map(|l| l.trim().split_once('['))
        .map(|(i, _)| i)
        .collect();
    let n = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), n, "a node is declared more than once");
}

#[test]
fn header_row_is_not_an_edge() {
    assert!(
        edges("## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\n")
            .is_empty()
    );
}

#[test]
fn depth_invariant_violated() {
    // An edge that is two levels deep (`src/subdir`) violates V2.
    let t = "\
## \u{a7}F FEDERATION\
\ndir|owns|\u{22a5}owns|tokens\
\nsrc/subdir|code nodes|scripts, docs|1200\
";
    // Parse the edges from the federation table.
    let e = edges(t);
    assert_eq!(e.len(), 1, "Expected exactly one edge in the test data");

    // The new public function that checks V2 should return the offending rows.
    // It is expected to be implemented elsewhere in this module.
    let violations = depth_violations(&e);

    // The current implementation does not perform this check,
    // so `violations` will be empty and the assertion below will fail.
    assert!(
        !violations.is_empty(),
        "Expected a violation for edge with dir 'src/subdir', but none were reported"
    );
    assert_eq!(
        violations.first().map(|v| v.dir.clone()),
        Some("src/subdir".to_string())
    );
}

#[test]
fn missing_not_owns_detected() {
    // Federation table with an edge that has an empty ⊥owns cell.
    let t = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\nsrc|code nodes||1200\n";
    let e = edges(t);
    assert_eq!(e.len(), 1, "Expected exactly one edge in the test data");

    // The new public function that checks V3 should return the offending rows.
    let violations = missing_not_owns(&e);

    // Current implementation does not perform this check,
    // so `violations` will be empty and the assertion below will fail.
    assert!(
        !violations.is_empty(),
        "Expected a violation for edge with empty ⊥owns, but none were reported"
    );
    assert_eq!(
        violations.first().map(|v| v.dir.clone()),
        Some("src".to_string())
    );
}

#[test]
fn discover_ignores_globs() {
    // The root of the repository (where the tests run).
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    // Run the walker.
    let discovered = discover(root);

    // Directory names that must be skipped by the walker.
    let ignored_names = ["target", ".git", "node_modules", ".direnv"];

    for name in &ignored_names {
        // The new public helper should report these as ignored.
        assert!(
            is_ignored_dir(name),
            "is_ignored_dir should return true for '{}'",
            name
        );

        // Verify that the walker never returned a path ending with an ignored dir.
        let contains = discovered
            .iter()
            .any(|p| p.file_name().and_then(|s| s.to_str()) == Some(*name));
        assert!(
            !contains,
            "discovered paths contain ignored directory '{}': {:?}",
            name,
            discovered
                .iter()
                .filter(
                    |p| p.file_name().and_then(|s| s.to_str()) == Some(*name)
                )
                .collect::<Vec<_>>()
        );
    }

    // A normal directory should not be reported as ignored.
    assert!(
        !is_ignored_dir("src"),
        "normal directory 'src' incorrectly marked as ignored"
    );
}

#[test]
fn exhaustive_invariant_detects_duplicates_and_missing() {
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Create a unique temporary directory inside the OS temp dir.
    let mut tmp = std::env::temp_dir();
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    tmp.push(format!("exhaustive_test_{}", suffix));
    fs::create_dir_all(&tmp).expect("failed to create temp dir");

    // Ensure the directory is cleaned up even if an assertion panics.
    // (The generated code reached for `scopeguard`, which is not a dependency
    // here; four lines of Drop is cheaper than a crate.)
    struct Cleanup(PathBuf);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    let _guard = Cleanup(tmp.clone());

    // Create two child directories: one that will be duplicated in the table,
    // and another that will be missing from the table.
    let child1 = tmp.join("child1");
    let child2 = tmp.join("child2");
    fs::create_dir_all(&child1).expect("failed to create child1");
    fs::create_dir_all(&child2).expect("failed to create child2");

    // Federation table with two identical rows for `child1` and no row for `child2`.
    let f_text = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\n\
                  child1|code||-\n\
                  child1|code||-\n";

    // Parse the edges from the table.
    let edges_vec = edges(f_text);

    // Call the new public function that checks V11.
    // It is expected to return a tuple of (duplicate_edges, missing_dirs).
    let (duplicates, missing) = find_exhaustive_violations(&edges_vec, &tmp);

    // Verify that at least one duplicate was reported for `child1`.
    assert!(
        !duplicates.is_empty(),
        "Expected duplicate rows for 'child1', but none were reported"
    );
    assert!(
        duplicates.iter().any(|e| e.dir == "child1"),
        "Duplicate row for 'child1' not found in the report: {:?}",
        duplicates
    );

    // Verify that `child2` was reported as missing from the table.
    assert!(
        !missing.is_empty(),
        "Expected a missing entry for 'child2', but none were reported"
    );
    assert!(
        missing
            .iter()
            .any(|p| p.file_name().and_then(|s| s.to_str()) == Some("child2")),
        "Missing child directory 'child2' not found in the report: {:?}",
        missing
    );
}
