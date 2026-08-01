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
pub fn check_edge_depth(text: &str) -> Vec<String> {
    let mut violations = Vec::new();
    let mut in_f = false;

    for line in text.lines() {
        // Detect the start of a §F table.
        if line.starts_with("## \u{a7}") {
            in_f = line.starts_with("## \u{a7}F");
            continue;
        }

        if !in_f {
            continue;
        }

        let cells = split_row(line);
        // Skip the header row and any malformed rows.
        if cells.len() != 4 || cells[0] == "dir" {
            continue;
        }

        // The invariant: a child directory must be exactly one level deeper,
        // i.e. it should not contain a slash.
        if cells[0].contains('/') {
            violations.push(line.trim().to_string());
        }
    }

    violations
}
/// Detect rows in a §F table where the ⊥owns cell is missing (empty).
///
/// The function scans the supplied `text` for a section starting with
/// `## §F`.  For each row that has exactly four cells and is not the
/// header, it checks whether the third cell (`⊥owns`) contains any
/// non‑whitespace characters.  If the cell is empty, the original line
/// (trimmed of surrounding whitespace) is added to the returned vector.
///
/// The returned `Vec<String>` contains one entry per offending row,
/// suitable for reporting or further inspection.
#[must_use]
pub fn missing_not_owns(text: &str) -> Vec<String> {
    let mut violations = Vec::new();
    let mut in_f = false;

    for line in text.lines() {
        // Detect the start of a §F table.
        if line.starts_with("## \u{a7}") {
            in_f = line.starts_with("## \u{a7}F");
            continue;
        }

        if !in_f {
            continue;
        }

        let cells = split_row(line);
        // Skip the header row and any malformed rows.
        if cells.len() != 4 || cells[0] == "dir" {
            continue;
        }

        // The ⊥owns cell is the third column (index 2).
        if cells[2].trim().is_empty() {
            violations.push(line.trim().to_string());
        }
    }

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
fn edge_depth_invariant_violated() {
    // A federation table where the child directory is two levels deeper than its parent.
    let text = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\nsrc/subdir|code|-|-\n";
    
    // The new public function that validates edge depth. It should return the offending rows.
    let violations = check_edge_depth(text);
    
    // We expect at least one violation because `src/subdir` is not a single level deeper.
    assert!(!violations.is_empty(), "Expected a violation but found none");
    
    // The offending row should contain the problematic directory name.
    assert!(
        violations.iter().any(|row| row.contains("src/subdir")),
        "The reported violation does not mention `src/subdir`"
    );
}

#[test]
fn missing_not_owns_detected() {
    // Federation table with a row where the ⊥owns cell is empty.
    let text = "## \u{a7}F FEDERATION\ndir|owns|\u{22a5}owns|tokens\nsrc|code||-\n";
    
    // The new public function that reports rows missing a ⊥owns value.
    let violations = missing_not_owns(text);
    
    // We expect at least one violation because the row for `src` has an empty ⊥owns cell.
    assert!(
        !violations.is_empty(),
        "Expected a violation for a missing ⊥owns, but found none"
    );
    
    // The offending row should mention the problematic directory name.
    assert!(
        violations.iter().any(|row| row.contains("src")),
        "The reported violation does not mention `src`"
    );
}
}
