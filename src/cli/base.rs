//! `budget --base <rev> [--check]` -- judge a ceiling change by CHAIN.
//!
//! A row-diff guard refuses a split that moves rows out of a fat root: the
//! split adds a node and raises sibling file rows, yet every context an agent
//! loads got smaller. What matters is the max single chain, not the row count.

use super::*;
use std::collections::BTreeMap;

/// One side of the comparison: each node's measured chain and ceiling, plus
/// the per-file rows (`<dir>/SPEC.md`) keyed by their node.
#[derive(Default)]
pub(super) struct Side {
    /// node -> (measured chain tokens, chain ceiling)
    pub chains: BTreeMap<String, (u64, u64)>,
    /// node -> file ceiling
    pub files: BTreeMap<String, u64>,
}

fn node_name(root: &Path, node: &Path) -> String {
    let rel = node.strip_prefix(root).unwrap_or(node);
    if rel.as_os_str().is_empty() {
        ".".to_string()
    } else {
        rel.to_string_lossy().into_owned()
    }
}

/// Measure the `scope` subtree of a tree. `Err` names what could not be read:
/// a side that measured nothing is not a side that passes.
pub(super) fn measure_side(root: &Path, scope: &Path) -> Result<Side, String> {
    let b = measure_budget(root, scope);
    if let Some(e) = b.stderr {
        return Err(e);
    }
    let mut side = Side::default();
    for r in &b.rows {
        side.chains
            .insert(node_name(root, &r.node), (r.chain_tokens, r.ceiling));
    }
    for (key, limit) in tokens::Ceilings::load(root)?.rows() {
        if let Some(dir) = key.strip_suffix("/SPEC.md") {
            side.files.insert(dir.to_string(), *limit);
        }
    }
    Ok(side)
}

/// The four rules. Returns one line per breach; empty means the change passes.
pub(super) fn judge(base: &Side, head: &Side) -> Vec<String> {
    let mut out = Vec::new();
    let base_max = base.chains.values().map(|c| c.1).max().unwrap_or(0);
    for (node, &(cost, ceiling)) in &head.chains {
        match base.chains.get(node) {
            // 1. an existing chain's ceiling rises
            Some(&(_, was)) if ceiling > was => out.push(format!(
                "{node}: chain ceiling raised {was} -> {ceiling}"
            )),
            Some(_) => {}
            // 3. a new node is no bigger than the biggest chain at base
            None if ceiling > base_max => out.push(format!(
                "{node}: new node's chain ceiling {ceiling} exceeds the \
                 largest chain ceiling at base ({base_max})"
            )),
            None => {}
        }
        // 4. any measured chain over its ceiling
        if cost > ceiling {
            out.push(format!(
                "{node}: chain {cost} tok is OVER its ceiling {ceiling} by {}",
                cost - ceiling
            ));
        }
    }
    // 2. a file ceiling rises while its node's chain ceiling does not fall.
    // A file row ADDED to an existing node counts as a raise from nothing.
    for (node, &limit) in &head.files {
        let Some(&(_, was_chain)) = base.chains.get(node) else {
            continue; // a new node: rule 3 governs it
        };
        let was = base.files.get(node).copied().unwrap_or(0);
        let now_chain = head.chains.get(node).map_or(was_chain, |c| c.1);
        if limit > was && now_chain >= was_chain {
            out.push(format!(
                "{node}/SPEC.md: file ceiling raised {was} -> {limit}, but \
                 row `{node}` chain ceiling does not fall ({was_chain} -> \
                 {now_chain})"
            ));
        }
    }
    out
}

fn print_side(label: &str, s: &Side) {
    println!("{label}:");
    for (node, (cost, ceiling)) in &s.chains {
        println!("  {node:<24} chain {cost:>6} tok  ceiling {ceiling:>6}");
    }
}

/// Export `rev`'s tree into a fresh directory and measure it there.
fn measure_rev(root: &Path, rev: &str, scope: &Path) -> Result<Side, String> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let dir = std::env::temp_dir()
        .join(format!("sherd-base-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    // The same subtree, re-rooted under the export: `[dir]` is one target set
    // on both sides, or the check certifies nodes the change never touched.
    let rel = scope.strip_prefix(root).unwrap_or(Path::new(""));
    let result = export(root, rev, &dir)
        .and_then(|()| measure_side(&dir, &dir.join(rel)))
        .and_then(|side| {
            // A base with no node measured nothing: judging against it would
            // report every head node as "exceeds the largest ceiling (0)".
            if side.chains.is_empty() {
                Err(format!(
                    "`{rev}` has no SPEC.md node under {}: nothing to compare against",
                    root.display()
                ))
            } else {
                Ok(side)
            }
        });
    let _ = std::fs::remove_dir_all(&dir);
    result
}

fn export(root: &Path, rev: &str, dir: &Path) -> Result<(), String> {
    let archive = crate::git::at(root, &["archive", "--format=tar", rev])
        .output()
        .map_err(|e| format!("git archive: {e}"))?;
    if !archive.status.success() {
        return Err(format!(
            "`{rev}` is not a revision git can read: {}",
            String::from_utf8_lossy(&archive.stderr).trim()
        ));
    }
    let mut tar = std::process::Command::new("tar")
        .args(["-x", "-C"])
        .arg(dir)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("tar: {e}"))?;
    {
        use std::io::Write;
        let stdin = tar.stdin.as_mut().ok_or("tar: no stdin")?;
        stdin
            .write_all(&archive.stdout)
            .map_err(|e| format!("tar: {e}"))?;
    }
    let status = tar.wait().map_err(|e| format!("tar: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err("tar could not extract the base tree".to_string())
    }
}

pub(super) fn budget_base(
    root: &Path,
    dir: &Path,
    rev: &str,
    check: bool,
) -> ExitCode {
    let (base, head) =
        match (measure_rev(root, rev, dir), measure_side(root, dir)) {
            (Ok(b), Ok(h)) => (b, h),
            (Err(e), _) | (_, Err(e)) => {
                eprintln!("sherd: {e}");
                return ExitCode::from(2);
            }
        };
    print_side(&format!("base {rev}"), &base);
    print_side("head", &head);
    println!(
        "\n  {} chains at base · {} at head · max chain ceiling {} -> {}",
        base.chains.len(),
        head.chains.len(),
        base.chains.values().map(|c| c.1).max().unwrap_or(0),
        head.chains.values().map(|c| c.1).max().unwrap_or(0),
    );
    if !check {
        return ExitCode::SUCCESS;
    }
    let breaches = judge(&base, &head);
    if breaches.is_empty() {
        println!("  ceiling change passes: no chain ceiling rose");
        return ExitCode::SUCCESS;
    }
    for b in &breaches {
        eprintln!("{b}");
    }
    ExitCode::from(1)
}

#[cfg(test)]
#[path = "tests/base.rs"]
mod tests;
