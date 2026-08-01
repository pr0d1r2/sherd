//! Federation: the `§F` table, the parent/child edges, the chain to a node.
//!
//! This is what blackbox adds on top of cavespec. `§F` rows are
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

fn split_row(line: &str) -> Vec<String> {
    let mut cells = vec![String::new()];
    let mut esc = false;
    for ch in line.trim().chars() {
        match ch {
            '\\' if !esc => esc = true,
            '|' if !esc => cells.push(String::new()),
            c => {
                if let Some(last) = cells.last_mut() {
                    last.push(c);
                }
                esc = false;
            }
        }
    }
    cells.iter().map(|c| c.trim().to_string()).collect()
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
    const IGNORE: [&str; 4] = ["target", ".git", "node_modules", ".direnv"];
    if dir.join("SPEC.md").is_file() {
        out.push(dir.to_path_buf());
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let name = p.file_name().unwrap_or_default().to_string_lossy().to_string();
        if IGNORE.contains(&name.as_str()) {
            continue;
        }
        walk(&p, out);
    }
}
/// Find all federation edges that violate the depth invariant (V2).
///
/// An edge is considered a violation if its `dir` field contains more than one
/// path component, i.e. it points to a directory that is not exactly one level
/// deeper than the node declaring it.
///
/// The function walks the entire repository tree starting at `root`, looks for
/// any Markdown files (`*.md`) and parses federation tables from them using
/// the existing `edges` helper.  All offending edges are returned in a vector.
#[must_use]
pub fn find_depth_violations(root: &Path) -> Vec<Edge> {
    use std::fs::{read_to_string, read_dir};
    use std::path::Component;

    let mut violations = Vec::new();

    // Recursive helper to walk the directory tree.
    fn walk(dir: &Path, violations: &mut Vec<Edge>) {
        // Read all entries in the current directory.
        if let Ok(entries) = read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Recurse into subdirectories.
                    walk(&path, violations);
                } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    // Read the markdown file and parse federation edges.
                    if let Ok(content) = read_to_string(&path) {
                        for edge in edges(&content) {
                            // Count components of the relative dir.
                            let comp_count = Path::new(&edge.dir)
                                .components()
                                .filter(|c| matches!(c, Component::Normal(_)))
                                .count();
                            if comp_count != 1 {
                                violations.push(edge);
                            }
                        }
                    }
                }
            }
        }
    }

    walk(root, &mut violations);
    violations
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

    #[test]
    fn escaped_pipe_stays_in_the_cell() {
        let t = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\na|rule\\|why|-|10\n";
        assert_eq!(edges(t)[0].owns, "rule|why");
    }

    #[test]
    fn header_row_is_not_an_edge() {
        assert!(edges("## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\n").is_empty());
    }

#[test]
fn detects_depth_invariant_violation() {
    use std::fs::{create_dir_all, write};
    use std::path::{Path, PathBuf};

    // Create a temporary directory for the test.
    let root = Path::new("temp_depth_test");
    if root.exists() {
        std::fs::remove_dir_all(root).unwrap();
    }
    create_dir_all(root).unwrap();

    // Write a federation file that contains an edge pointing two levels deeper
    // than its parent (the root directory), which violates V2.
    let content = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\nsubdir/subsubdir|foo|bar|-";
    write(root.join("README.md"), content).unwrap();

    // Call the new public function that checks depth invariants.
    let violations = find_depth_violations(root);

    // The test should fail if no violations are reported, i.e. the current
    // implementation does not enforce V2.
    assert!(
        !violations.is_empty(),
        "Expected at least one depth violation but found none"
    );

    // Verify that the offending edge is the one we inserted.
    let offending_edge = &violations[0];
    assert_eq!(
        offending_edge.dir,
        "subdir/subsubdir",
        "The violating edge should point to 'subdir/subsubdir'"
    );

    // Clean up after ourselves.
    std::fs::remove_dir_all(root).unwrap();
}
}
