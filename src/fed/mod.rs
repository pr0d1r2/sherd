//! Federation: the `§F` table, the parent/child edges, the chain to a node.
//!
//! This is what sherd adds on top of microlith. `§F` rows are
//! `dir|owns|⊥owns|tokens` and an edge is **exactly one dir deeper** (V2).

use std::path::{Path, PathBuf};

/// One `§F` row: an edge to a child node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// Child dir, relative to the node declaring it. Depth +1 exactly (V2).
    pub dir: String,
    /// What the child owns -- decides DESCEND.
    pub owns: String,
    /// What it does NOT own, and where that lives -- decides STOP (V66).
    /// This is the byte that prevents loading.
    pub not_owns: String,
    /// Recorded size of the child subtree pack, or `None` for `-`.
    pub tokens: Option<u64>,
}

/// Parse the `§F FEDERATION` table out of one spec. Rows are pipe-delimited;
/// a literal `|` inside a cell is escaped `\|`.
#[must_use]
pub fn edges(text: &str) -> Vec<Edge> {
    let mut out = Vec::new();
    let mut in_f = false;
    for line in text.lines() {
        if line.starts_with("## \u{a7}") {
            in_f = line.starts_with("## \u{a7}F");
            continue;
        }
        if !in_f {
            continue;
        }
        let cells = split_row(line);
        // Skip the header row and anything that is not a 4-cell row.
        if cells.len() != 4 || cells[0] == "dir" {
            continue;
        }
        out.push(Edge {
            dir: cells[0].clone(),
            owns: cells[1].clone(),
            not_owns: cells[2].clone(),
            tokens: cells[3].parse().ok(),
        });
    }
    out
}

/// Split a `§F` row on unescaped pipes.
///
/// `\` escapes the next character only when that character is `\` or `|`;
/// before anything else it is literal and kept (V4). Two defects shaped this
/// and both are in `§B`:
///
/// B11 -- the first version consumed `\` before ANY character, so the
/// backslash vanished out of `C:\path`, and a cell ending in `\\` produced
/// five cells for a four-column row, which `edges()` then dropped silently.
///
/// B12 -- the first fix escaped only before `|`, which left a cell ENDING in
/// a backslash unrepresentable: `tail\\|` swallowed the column break. An
/// escape scheme has to be able to express its own escape character.
fn split_row(line: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut chars = line.trim().chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' if matches!(chars.peek(), Some('\\' | '|')) => {
                if let Some(c) = chars.next() {
                    push(&mut cells, c);
                }
            }
            '|' => cells.push(String::new()),
            c => push(&mut cells, c),
        }
    }
    cells.iter().map(|c| c.trim().to_string()).collect()
}

/// Append to the cell being built. The slice is seeded with one `String` and
/// never shrinks, so the `None` arm is unreachable -- but `indexing_slicing`
/// is denied and an `expect` here would be a panic in a tool that runs
/// unattended.
fn push(cells: &mut [String], c: char) {
    if let Some(last) = cells.last_mut() {
        last.push(c);
    }
}

/// The chain of `SPEC.md` files from repo root down to `dir`, inclusive.
///
/// V19: a reader descends one edge at a time, so the chain is what any node
/// costs to reach -- root plus every ancestor plus itself.
#[must_use]
pub fn chain(root: &Path, dir: &Path) -> Vec<PathBuf> {
    let mut nodes = Vec::new();
    let rel = dir.strip_prefix(root).unwrap_or(dir);
    let mut cur = root.to_path_buf();
    let root_spec = cur.join("SPEC.md");
    if root_spec.is_file() {
        nodes.push(root_spec);
    }
    for part in rel.components() {
        cur = cur.join(part);
        let spec = cur.join("SPEC.md");
        if spec.is_file() {
            nodes.push(spec);
        }
    }
    nodes
}

/// Every dir under `root` that carries a `SPEC.md`, ignoring build output.
#[must_use]
pub fn discover(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if dir.join("SPEC.md").is_file() {
        out.push(dir.to_path_buf());
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if is_ignored_dir(&name) {
            continue;
        }
        walk(&p, out);
    }
}
/// Return all edges that violate the V2 depth invariant.
///
/// An edge violates V2 if its `dir` field contains more than one path component,
/// i.e., it is not a direct child of the node declaring it.
pub fn depth_violations(edges: &[Edge]) -> Vec<&Edge> {
    edges
        .iter()
        .filter(|e| e.dir.split('/').count() != 1)
        .collect()
}
/// Return all edges that violate the V3 invariant.
///
/// An edge violates V3 if its `not_owns` field is empty (or contains only whitespace).
pub fn missing_not_owns(edges: &[Edge]) -> Vec<&Edge> {
    edges
        .iter()
        .filter(|e| e.not_owns.trim().is_empty())
        .collect()
}

/// The federation as a mermaid graph, derived from `§F`.
///
/// GENERATED, never authored (`.:V83`).
///
/// Maximally conservative syntax, because two richer versions failed to render
/// (B6): `graph TD` not `flowchart` (older mermaid, which GitLab pins, does not
/// know `flowchart`), bare directory names as labels, no HTML, no colons, no
/// punctuation, ASCII only. The lens text lives in [`table`], which is plain
/// markdown and always renders.
#[must_use]
pub fn mermaid(root: &Path) -> String {
    let mut defs = String::new();
    let mut links = String::new();
    let mut seen: Vec<String> = Vec::new();
    for node in discover(root) {
        let rel = node.strip_prefix(root).unwrap_or(&node);
        let from = label(rel);
        if !seen.contains(&from) {
            seen.push(from.clone());
            defs.push_str(&format!("    {from}[{}]\n", ident(&disp(rel))));
        }
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
            continue;
        };
        for e in edges(&text) {
            let id = label(&rel.join(&e.dir));
            if !seen.contains(&id) {
                seen.push(id.clone());
                defs.push_str(&format!("    {id}[{}]\n", ident(&e.dir)));
            }
            links.push_str(&format!("    {from} --> {id}\n"));
        }
    }
    format!("graph TD\n{defs}{links}")
}

/// The `§F` lens text as a markdown table -- what each node owns and, more
/// usefully, what it does NOT (`.:V66`). Generated alongside [`mermaid`]:
/// plain markdown renders everywhere, and carries more than a node label can.
#[must_use]
pub fn table(root: &Path) -> String {
    let mut out =
        String::from("| node | owns | does not own |\n|---|---|---|\n");
    for node in discover(root) {
        let rel = node.strip_prefix(root).unwrap_or(&node);
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
            continue;
        };
        for e in edges(&text) {
            out.push_str(&format!(
                "| `{}` | {} | {} |\n",
                rel.join(&e.dir).display(),
                cell(&e.owns),
                cell(&e.not_owns)
            ));
        }
    }
    out
}

/// A markdown table cell: escape the delimiter, keep the text otherwise intact.
fn cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// A mermaid node label with no character that any mermaid version treats as
/// syntax: letters, digits, spaces, hyphens and underscores only.
fn ident(s: &str) -> String {
    let t: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.is_empty() { "root".into() } else { t }
}

/// The federation as a plain ASCII tree.
///
/// Renders in any markdown, any viewer, forever -- no renderer to fail (B15).
/// Same `§F` rows as [`mermaid`] and [`table`], so it cannot drift.
#[must_use]
pub fn tree(root: &Path) -> String {
    let mut out = String::from(".\n");
    fn walk_tree(root: &Path, rel: &Path, prefix: &str, out: &mut String) {
        let Ok(text) = std::fs::read_to_string(root.join(rel).join("SPEC.md"))
        else {
            return;
        };
        let es = edges(&text);
        for (i, e) in es.iter().enumerate() {
            let last = i + 1 == es.len();
            out.push_str(&format!(
                "{prefix}{}{}\n",
                if last { "`-- " } else { "|-- " },
                e.dir
            ));
            let deeper =
                format!("{prefix}{}", if last { "    " } else { "|   " });
            walk_tree(root, &rel.join(&e.dir), &deeper, out);
        }
    }
    walk_tree(root, Path::new(""), "", &mut out);
    out
}

/// The same graph in graphviz `dot`.
#[must_use]
pub fn dot(root: &Path) -> String {
    let mut out = String::from(
        "digraph federation {\n  rankdir=TB;\n  node [shape=box];\n",
    );
    for node in discover(root) {
        let rel = node.strip_prefix(root).unwrap_or(&node);
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
            continue;
        };
        for e in edges(&text) {
            out.push_str(&format!(
                "  \"{}\" -> \"{}\";\n",
                disp(rel),
                rel.join(&e.dir).display()
            ));
        }
    }
    out.push_str("}\n");
    out
}

fn label(p: &Path) -> String {
    let s = p.to_string_lossy().replace(['/', '.', '-'], "_");
    if s.is_empty() { "root".into() } else { s }
}

fn disp(p: &Path) -> String {
    let s = p.to_string_lossy().to_string();
    if s.is_empty() { ".".into() } else { s }
}
/// Return `true` if the directory name should be ignored by the walker.
///
/// The repository contains a handful of directories that are not part of the
/// source tree and should never be traversed: `target`, `.git`,
/// `node_modules`, and `.direnv`.  All other names are considered valid.
pub fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        "target" | ".git" | "node_modules" | ".direnv"
        // SUPERVISOR assets: instructions for the higher agent. They must
        // never become federation nodes, because a node's SPEC.md reaches the
        // local model's prompt and supervisor instructions are not for it
        // (V13). Excluded by discovery, not by convention.
        | ".claude" | ".github" | ".codex" | ".githooks"
    )
}
pub fn find_exhaustive_violations<'a>(
    edges: &'a [Edge],
    root: &Path,
) -> (Vec<&'a Edge>, Vec<PathBuf>) {
    // Count how many times each dir appears in the edge list.
    let mut counts = std::collections::HashMap::<&str, usize>::new();
    for e in edges {
        *counts.entry(e.dir.as_str()).or_insert(0) += 1;
    }

    // Collect all edges that have a duplicate entry.
    let duplicates: Vec<&Edge> = edges
        .iter()
        .filter(|e| counts.get(e.dir.as_str()).copied().unwrap_or(0) > 1)
        .collect();

    // Build a set of dir names present in the edge list for quick lookup.
    let edge_dirs: std::collections::HashMap<&str, ()> =
        edges.iter().map(|e| (e.dir.as_str(), ())).collect();

    // Scan the filesystem under `root` and report any directories that are
    // missing from the edge table.
    let mut missing = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|s| s.to_str())
                && !edge_dirs.contains_key(name)
                && !is_ignored_dir(name)
            {
                missing.push(path);
            }
        }
    }

    (duplicates, missing)
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: &str = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\nsrc|code nodes|scripts, docs|1200\nscripts|inference harness|rust code|-\n\n## \u{a7}V INVARIANTS\nV1: x\n";

    #[test]
    fn parses_rows_and_stops_at_next_section() {
        let e = edges(F);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].dir, "src");
        assert_eq!(e[0].not_owns, "scripts, docs");
        assert_eq!(e[0].tokens, Some(1200));
        assert_eq!(e[1].tokens, None, "`-` means unrecorded, not zero");
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
        assert_eq!(edges(t)[0].owns, "rule|why");
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
        assert_eq!(violations[0].dir, "src/subdir");
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
        assert_eq!(violations[0].dir, "src");
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
                    .filter(|p| p.file_name().and_then(|s| s.to_str())
                        == Some(*name))
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
        let (duplicates, missing) =
            find_exhaustive_violations(&edges_vec, &tmp);

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
                .any(|p| p.file_name().and_then(|s| s.to_str())
                    == Some("child2")),
            "Missing child directory 'child2' not found in the report: {:?}",
            missing
        );
    }
}

/// One `§N NAV` row: where a node sits, and the one-line lens of what it
/// finds there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nav {
    /// `up` an ancestor · `self` this node · `sib` a co-child.
    pub rel: String,
    pub path: String,
    pub lens: String,
}

/// The `§N` table a node should carry, DERIVED from its ancestors' `§F`.
///
/// `§F` is authoritative and `§N` is generated (`.:V36`), so this never reads
/// an existing `§N` -- it computes what one must say. The lens of each row is
/// a verbatim copy of that directory's `§F` row `owns` cell (`.:V38`): one
/// source, and a nav table that cannot describe a node differently from the
/// table that declares it.
///
/// Root gets `up = -` and `self = .` with no siblings (`.:V35`); every other
/// node gets one `up` per ancestor, exactly one `self`, and one `sib` per
/// co-child (`.:V34`).
#[must_use]
fn nav_root() -> Vec<Nav> {
    vec![
        Nav {
            rel: "up".into(),
            path: "-".into(),
            lens: "-".into(),
        },
        Nav {
            rel: "self".into(),
            path: ".".into(),
            lens: "-".into(),
        },
    ]
}

#[must_use]
pub fn nav(root: &Path, node: &Path) -> Vec<Nav> {
    let rel = node.strip_prefix(root).unwrap_or(node);
    if rel.as_os_str().is_empty() {
        return nav_root();
    }
    let mut rows = Vec::new();
    let mut cur = root.to_path_buf();
    rows.push(Nav {
        rel: "up".into(),
        path: ".".into(),
        lens: "-".into(),
    });
    for part in rel.components() {
        let parent = cur.clone();
        cur = cur.join(part);
        let name = part.as_os_str().to_string_lossy().to_string();
        let lens = lens_of(&parent, &name);
        let path = cur
            .strip_prefix(root)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| name.clone());
        if cur == node {
            rows.push(Nav {
                rel: "self".into(),
                path,
                lens,
            });
            rows.extend(siblings(&parent, root, &name));
        } else {
            rows.push(Nav {
                rel: "up".into(),
                path,
                lens,
            });
        }
    }
    rows
}

/// The `owns` cell a parent's `§F` gives one child, or `-` when the parent
/// declares no row for it. `-` is honest: the child exists and its parent
/// has not said what it owns.
fn lens_of(parent: &Path, child: &str) -> String {
    let Ok(text) = std::fs::read_to_string(parent.join("SPEC.md")) else {
        return "-".into();
    };
    edges(&text)
        .into_iter()
        .find(|e| e.dir == child)
        .map_or_else(|| "-".into(), |e| e.owns)
}

/// Every co-child of `name` under `parent`, as `sib` rows.
fn siblings(parent: &Path, root: &Path, name: &str) -> Vec<Nav> {
    let Ok(text) = std::fs::read_to_string(parent.join("SPEC.md")) else {
        return Vec::new();
    };
    let base = parent.strip_prefix(root).unwrap_or(parent);
    edges(&text)
        .into_iter()
        .filter(|e| e.dir != name)
        .map(|e| Nav {
            rel: "sib".into(),
            path: base.join(&e.dir).to_string_lossy().to_string(),
            lens: e.owns,
        })
        .collect()
}

/// The `§N` section as text, header row included.
#[must_use]
pub fn nav_section(rows: &[Nav]) -> String {
    let mut s = String::from("## \u{a7}N NAV\n\nrel|path|lens\n");
    for r in rows {
        s.push_str(&format!("{}|{}|{}\n", r.rel, r.path, r.lens));
    }
    s
}

#[cfg(test)]
mod nav_tests {
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
}
