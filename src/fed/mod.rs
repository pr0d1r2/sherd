//! Federation: the `§F` table, the parent/child edges, the chain to a node.
//!
//! This is what sherd adds on top of microlith. `§F` rows are
//! `dir|owns|⊥owns|tokens` and an edge is **exactly one dir deeper** (V2).

use std::collections::HashSet;
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
        // The pattern IS the row shape: exactly four cells, and the header
        // row names its first column `dir`.
        let cells = split_row(line);
        let [dir, owns, not_owns, tokens] = cells.as_slice() else {
            continue;
        };
        if dir == "dir" {
            continue;
        }
        out.push(Edge {
            dir: dir.clone(),
            owns: owns.clone(),
            not_owns: not_owns.clone(),
            tokens: tokens.parse().ok(),
        });
    }
    out
}

/// Split a `§F` row on unescaped pipes (V4).
///
/// The grammar is UPSTREAM's, and so is the code that reads it:
/// `microlith::cells` decides which pipes are structural and
/// `microlith::unescape` decodes what is left. `src/spec:V1` -- what
/// microlith owns is adapted, not reimplemented -- and `src/spec:V5` -- what
/// the root re-exports is contract -- both point here, and microlith exports
/// this codec as a set precisely because a pipe row is the one construct a
/// consumer cannot avoid re-reading (its `B36`).
///
/// This node had its own reading for the project's life, and three defects
/// came out of it -- all in `§B`, all kept as the record of why:
///
/// B11 -- it consumed `\` before ANY character, so the backslash vanished out
/// of `C:\path`, and a cell ending in `\\` produced five cells for a
/// four-column row, which `edges()` then dropped silently.
///
/// B12 -- the first fix escaped only before `|`, which left a cell ENDING in
/// a backslash unrepresentable. An escape scheme has to be able to express
/// its own escape character.
///
/// B13 -- the reader and the writer drifted apart, because they were two
/// readings rather than one codec used in both directions.
///
/// What stays local is `fed`'s own shape, not the grammar: the trim, and the
/// owned `String` per cell that `edges()` and `nav()` hold onto.
fn split_row(line: &str) -> Vec<String> {
    microlith::cells(line.trim())
        .into_iter()
        .map(|c| microlith::unescape(c.trim()))
        .collect()
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
    if is_node(&cur) {
        nodes.push(cur.join("SPEC.md"));
    }
    for part in rel.components() {
        cur = cur.join(part);
        if is_node(&cur) {
            nodes.push(cur.join("SPEC.md"));
        }
    }
    nodes
}

/// Every dir under `root` that carries a `SPEC.md`, ignoring build output
/// and, inside a git work tree, whatever git ignores (V22).
#[must_use]
pub fn discover(root: &Path) -> Vec<PathBuf> {
    let ignored = git_ignored(root);
    let mut out = Vec::new();
    walk(root, &ignored, &mut out);
    out.sort();
    out
}

/// What git ignores under `dir`, as paths joined onto `dir` (V22).
///
/// One `ls-files` per walk. `--directory` names a wholly ignored directory
/// once instead of every file in it, so a scratch dir holding whole
/// checkouts costs one line. Outside a work tree, or with no git at all,
/// the answer is empty and the fixed list is the only rule -- the walk as it
/// was before `.:B15`.
fn git_ignored(dir: &Path) -> HashSet<PathBuf> {
    let args = [
        "ls-files",
        "-z",
        "--others",
        "--ignored",
        "--exclude-standard",
        "--directory",
    ];
    let Ok(out) = crate::git::at(dir, &args).output() else {
        return HashSet::new();
    };
    if !out.status.success() {
        return HashSet::new();
    }
    out.stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| {
            let rel = String::from_utf8_lossy(p);
            dir.join(rel.trim_end_matches('/'))
        })
        .collect()
}

/// `dir` carries a file named EXACTLY `SPEC.md` (V23).
///
/// `dir.join("SPEC.md").is_file()` is not that question on a
/// case-insensitive filesystem: APFS answers it for `spec.md` too, so the
/// same tree had a different federation on macOS than on Linux (`.:B16`).
/// The entry's NAME is what the filesystem stores, whatever it matches.
#[must_use]
pub fn is_node(dir: &Path) -> bool {
    dir.join("SPEC.md").is_file()
        && std::fs::read_dir(dir)
            .is_ok_and(|rd| rd.flatten().any(|e| e.file_name() == "SPEC.md"))
}

fn walk(dir: &Path, ignored: &HashSet<PathBuf>, out: &mut Vec<PathBuf>) {
    if is_node(dir) && !ignored.contains(&dir.join("SPEC.md")) {
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
        if is_ignored_dir(&name) || ignored.contains(&p) {
            continue;
        }
        walk(&p, ignored, out);
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
/// BUILD OUTPUT and TOOLING, never a federation node. The list was tuned to
/// this repository and `.:B19` is what that cost: run against `rekall`, the
/// checker advised `§F` rows for `result/` -- a nix build symlink -- and
/// `pkl/`, a vendored schema every repo in the fleet carries. Neither is
/// source, in any repository, and advising a row for them tells a stranger
/// to federate their build directory.
pub fn is_ignored_dir(name: &str) -> bool {
    matches!(
        name,
        "target" | ".git" | "node_modules" | ".direnv"
        // `nix build` leaves `result` (and `result-<n>` for multiple outputs)
        // as symlinks into the store. `pkl` is the vendored hk schema.
        | "result" | "pkl" | "vendor"
        // SUPERVISOR assets: instructions for the higher agent. They must
        // never become federation nodes, because a node's SPEC.md reaches the
        // local model's prompt and supervisor instructions are not for it
        // (V13). Excluded by discovery, not by convention.
        | ".claude" | ".github" | ".codex" | ".githooks"
    ) || name.starts_with("result-")
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
    let ignored = git_ignored(root);
    let mut missing = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir()
                && let Some(name) = path.file_name().and_then(|s| s.to_str())
                && !edge_dirs.contains_key(name)
                && !is_ignored_dir(name)
                && !ignored.contains(&path)
            {
                missing.push(path);
            }
        }
    }

    (duplicates, missing)
}

/// Every `.rs` file under a directory, sorted, so a report is stable.
///
/// `target` and hidden directories are skipped: a build product is not source
/// and a ceiling over it measures the compiler. So is whatever git ignores
/// (V19, V22): a scratch checkout is another repository's source.
#[must_use]
pub fn rust_files(dir: &Path) -> Vec<PathBuf> {
    files_by_extension(dir, "rs", |name| name == "target")
}

/// Every `*.sh` file under a directory, sorted -- what `split` reads a shell
/// codebase from (`src/split:V7`).
///
/// The same walk as [`rust_files`], one walker rather than two (`src/debt`'s
/// rule), and the same exclusions as [`discover`]: hidden dirs, the fixed
/// list and whatever git ignores (V22). A script under `vendor/` or a scratch
/// checkout is another repository's, exactly as a `SPEC.md` there is.
#[must_use]
pub fn script_files(dir: &Path) -> Vec<PathBuf> {
    files_by_extension(dir, "sh", is_ignored_dir)
}

/// The one file walk: every file ending in `.<ext>`, skipping hidden dirs,
/// dirs `skip` names and what git ignores. Sorted, so a report is stable.
fn files_by_extension(
    dir: &Path,
    ext: &str,
    skip: fn(&str) -> bool,
) -> Vec<PathBuf> {
    let ignored = git_ignored(dir);
    let mut out = Vec::new();
    collect(
        dir,
        &Walk {
            ext,
            skip,
            ignored: &ignored,
        },
        &mut out,
    );
    out.sort();
    out
}

/// What one file walk keeps and skips.
struct Walk<'a> {
    ext: &'a str,
    skip: fn(&str) -> bool,
    ignored: &'a HashSet<PathBuf>,
}

fn collect(dir: &Path, w: &Walk<'_>, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.filter_map(Result::ok) {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if w.ignored.contains(&p) {
            continue;
        }
        if p.is_dir() {
            if !(w.skip)(&name) && !name.starts_with('.') {
                collect(&p, w, out);
            }
        } else if p.extension().is_some_and(|x| x == w.ext) {
            out.push(p);
        }
    }
}

/// A node's NAME: its path relative to the repository root, `.` for the root
/// itself rather than the empty string it strips to.
///
/// The form a citation uses (`src/spec:V7`) and the form every report prints.
/// Here rather than in a caller because the rule was already written twice --
/// `disp` above, for the graph renderings, and a copy in `src/cli` for `seam`
/// and `split` -- and a third consumer would have made it three, which is the
/// two-readings defect `.:B13` records.
///
/// A path from ANOTHER tree is handed back as it is: `strip_prefix` fails,
/// and relabelling it silently would name a node this repository has not got.
#[must_use]
pub fn node_label(root: &Path, dir: &Path) -> String {
    disp(dir.strip_prefix(root).unwrap_or(dir))
}

/// Nodes whose path ENDS with the spelling `rel` -- what a reader typed,
/// read as a name rather than as a location (V18).
///
/// `[dir]` resolves against the repo ROOT by design (`src/cli:V15`,
/// `src/cli:B9`), so `seam code` typed in `src/` looks for `<root>/code` and
/// misses. The node meant is `src/code`, and it is recoverable from the
/// spelling ALONE: no CWD is consulted, so what an argument means does not
/// depend on where the caller stands -- the ambiguity `src/cli:B5` and `B7`
/// were about. This answers a MISS; it never chooses a node.
///
/// A match of the whole relative path is excluded: that spelling is the one
/// that was just tried, and offering it back is not a suggestion.
#[must_use]
pub fn spelled(root: &Path, rel: &Path) -> Vec<PathBuf> {
    let want: Vec<_> = rel.components().collect();
    if want.is_empty() {
        return Vec::new();
    }
    discover(root)
        .into_iter()
        .filter(|node| {
            let have: Vec<_> = node
                .strip_prefix(root)
                .unwrap_or(node)
                .components()
                .collect();
            have.len() > want.len() && have.ends_with(&want)
        })
        .collect()
}

/// The `.rs` files a node OWNS: everything under it that no DEEPER node
/// claims (V15).
///
/// [`rust_files`] recurses and the nodes NEST -- `src` contains every other
/// node here -- so without this every file is reported by each of its
/// ancestors and the root owns the whole crate. Attributing a file to the
/// NEAREST node reports it exactly once, and a file in an unfederated
/// subdirectory still reaches the node above it rather than vanishing
/// (`.:V16`).
///
/// In this node rather than in the caller that first needed it: which node a
/// path belongs to is a question about federation STRUCTURE, and `seam` and
/// `wave` both ask it.
#[must_use]
pub fn owned_rust_files(node: &Path, nodes: &[PathBuf]) -> Vec<PathBuf> {
    owned(rust_files(node), node, nodes)
}

/// The `*.sh` files a node OWNS, by the same nearest-node rule (V15):
/// what `wave` reads a node's script calls from (`src/wave:V5`).
#[must_use]
pub fn owned_script_files(node: &Path, nodes: &[PathBuf]) -> Vec<PathBuf> {
    owned(script_files(node), node, nodes)
}

/// The files no DEEPER node claims.
fn owned(files: Vec<PathBuf>, node: &Path, nodes: &[PathBuf]) -> Vec<PathBuf> {
    files
        .into_iter()
        .filter(|f| {
            !nodes
                .iter()
                .any(|n| n != node && n.starts_with(node) && f.starts_with(n))
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/fed.rs"]
mod tests;

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
/// a verbatim copy of that directory's `§F` row `owns` cell (`src/fed:V21`): one
/// source, and a nav table that cannot describe a node differently from the
/// table that declares it.
///
/// Root gets `up = -` and `self = .` with no siblings (`src/fed:V20`); every other
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
        s.push_str(&format!("{}|{}|{}\n", r.rel, r.path, escape_cell(&r.lens)));
    }
    s
}

/// A cell as `split_row` reads it back (V16): `microlith::escape`, the
/// inverse of the decode above and the same one microlith's own writers use.
///
/// The local version doubled a backslash only where V4 would otherwise read
/// it as an escape -- before `\`, before `|`, or ending the cell -- and left
/// `C:\path` single. Upstream doubles every backslash, so the ENCODED form of
/// such a cell changes; the DECODED form does not, which is the only thing a
/// reader sees and the only thing V16 asserts. One codec used in both
/// directions is what `B13` asks for: a reader and a writer that are two
/// readings of one sentence will drift again.
fn escape_cell(cell: &str) -> String {
    microlith::escape(cell)
}

#[cfg(test)]
#[path = "tests/nav.rs"]
mod nav_tests;

/// One node the federation DECLARES, with the lens that declares it.
///
/// The path is relative to the repository root -- `src/fed`, or `.` for the
/// root itself -- which is the form a citation uses (`src/spec:V7`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Home {
    /// Path from the root, `.` for the root node.
    pub node: String,
    /// What the declaring `§F` row says this node owns.
    pub owns: String,
    /// What it says the node does NOT own.
    pub not_owns: String,
}

/// Every node the `§F` tables declare, root first.
///
/// This is the set a row may be placed ONTO (`src/adopt:V3`). It is read from
/// the TABLES rather than from the directory tree, and the difference is the
/// point: a dir carrying a `SPEC.md` that no parent `§F` row names is an
/// orphan (T8), and placing a row there would hide it from every reader who
/// descends the federation. V11 and V12 make the declared set exhaustive and
/// disjoint, so it is also the only set worth proposing from.
///
/// The root is always a home, because a row nobody claims stays there and is
/// NAMED there rather than left quietly (`src/adopt:V2`).
#[must_use]
pub fn declared(root: &Path) -> Vec<Home> {
    let mut out = vec![Home {
        node: ".".to_string(),
        owns: String::new(),
        not_owns: String::new(),
    }];
    for node in discover(root) {
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
            continue;
        };
        let rel = node.strip_prefix(root).unwrap_or(&node);
        out.extend(edges(&text).into_iter().map(|e| Home {
            node: join_rel(rel, &e.dir),
            owns: e.owns,
            not_owns: e.not_owns,
        }));
    }
    out.sort_by(|a, b| a.node.cmp(&b.node));
    out.dedup_by(|a, b| a.node == b.node);
    out
}

/// A child's path from the root: the parent's own relative path, then the
/// `§F` cell. The root's relative path is empty, and joining onto it would
/// produce a leading separator that no citation uses.
fn join_rel(parent: &Path, child: &str) -> String {
    let p = parent.to_string_lossy();
    if p.is_empty() {
        child.to_string()
    } else {
        format!("{p}/{child}")
    }
}

#[cfg(test)]
#[path = "tests/declared.rs"]
mod declared_tests;

#[cfg(test)]
#[path = "tests/ignored.rs"]
mod ignored_tests;

#[cfg(test)]
#[path = "tests/case.rs"]
mod case_tests;
