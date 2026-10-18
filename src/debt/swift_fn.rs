//! A SwiftPM exclusion of ONE FUNCTION, and the CAP on every excluded line
//! (`V11`, T5, #138).
//!
//! llvm-cov's `files[].summary.lines` is not a count of distinct lines: it
//! is the sum, over the file's function GROUPS (records sharing a first
//! region), of each group's own line count -- a closure's lines count once
//! for the closure and again for the function around it. So a function is
//! excluded by subtracting exactly what llvm-cov added for it, recomputed
//! from its `regions`, and the recomputation must give back the file's own
//! summary before anything is subtracted (`V3`).

use super::llvm_json::{Function, function_lines, functions, source_files};
use super::swift::{hundredths, swift_run};
use std::path::Path;
use std::process::{Command, Stdio};

/// One `exclude` row that carries a reason: a whole `Sources/` file, or one
/// function in it (`exclude <path>:<function> <reason>`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exclusion {
    /// Root-relative path, `./` trimmed.
    pub path: String,
    /// The function's simplified demangled name, `None` for the whole file.
    pub function: Option<String>,
}

/// The exclusions `.coverage` names. The target splits at its FIRST `:`, so
/// a Swift name keeps its own (`KitCrypto.make(_:)`). A row with no reason
/// excludes nothing, as `V10` has it.
#[must_use]
pub fn exclusions(root: &Path) -> Vec<Exclusion> {
    let text =
        std::fs::read_to_string(root.join(".coverage")).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let mut w = l.strip_prefix("exclude ")?.split_whitespace();
            let target = w.next()?;
            w.next()?;
            let (path, function) = match target.split_once(':') {
                Some((p, f)) if !f.is_empty() => (p, Some(f.to_string())),
                _ => (target, None),
            };
            Some(Exclusion {
                path: path.trim_start_matches("./").to_string(),
                function,
            })
        })
        .collect()
}

/// `.coverage`'s `excluded <n>` row: the most `Sources/` lines the
/// exclusions may remove. `None` when the row is absent, which caps at 0.
#[must_use]
pub fn excluded_cap(root: &Path) -> Option<usize> {
    let text = std::fs::read_to_string(root.join(".coverage")).ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("excluded ")?.trim().parse().ok())
}

/// A SwiftPM measurement: the figure, and the lines exclusions removed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwiftMeasure {
    /// Line coverage of what is left, in hundredths.
    pub lines: usize,
    /// `Sources/` lines removed from the denominator, file and function rows.
    pub excluded: usize,
}

/// Measure an llvm-cov export with `rows` applied (`V10`, `V11`).
/// `demangle` maps mangled names to `swift demangle --simplified` lines,
/// one per name, and is asked only when a function row exists.
///
/// # Errors
/// A row that matches nothing (a stale exclusion), a file whose functions
/// do not sum to llvm-cov's own count, a demangler that did not run, an
/// unreadable entry, or no line left under `Sources/`.
pub fn measure_export(
    json: &str,
    root: &Path,
    rows: &[Exclusion],
    demangle: &dyn Fn(&[String]) -> Option<Vec<String>>,
) -> Result<SwiftMeasure, String> {
    let roots = [Some(root.to_path_buf()), root.canonicalize().ok()];
    let unreadable = || "sherd/debt:V10: an llvm-cov entry is unreadable";
    let files = source_files(json, &roots).ok_or_else(unreadable)?;
    let named =
        |p: &str| rows.iter().any(|r| r.function.is_some() && r.path == p);
    let functions = if rows.iter().any(|r| r.function.is_some()) {
        functions(json, &roots, &named).ok_or_else(unreadable)?
    } else {
        Vec::new()
    };
    let names = if functions.is_empty() {
        Vec::new()
    } else {
        let mangled: Vec<String> =
            functions.iter().map(|f| f.name.clone()).collect();
        demangle(&mangled)
            .filter(|d| d.len() == mangled.len())
            .ok_or(
                "sherd/debt:V11: `swift demangle --simplified` did not run",
            )?
    };
    let (mut count, mut covered, mut excluded) = (0_usize, 0_usize, 0_usize);
    for row in rows {
        if !files.iter().any(|(p, ..)| *p == row.path) {
            return Err(stale(row, "no `Sources/` file llvm-cov measured"));
        }
    }
    for (path, n, c) in &files {
        let whole =
            rows.iter().any(|r| r.path == *path && r.function.is_none());
        let fns: Vec<&String> = rows
            .iter()
            .filter(|r| r.path == *path)
            .filter_map(|r| r.function.as_ref())
            .collect();
        let (dn, dc) = if whole {
            check_rows(&fns, path, &functions, &names)?;
            (*n, *c)
        } else if fns.is_empty() {
            (0, 0)
        } else {
            dropped(path, (*n, *c), &fns, &functions, &names)?
        };
        excluded = excluded.checked_add(dn).ok_or_else(unreadable)?;
        count = count
            .checked_add(n.checked_sub(dn).ok_or_else(unreadable)?)
            .ok_or_else(unreadable)?;
        covered = covered
            .checked_add(c.checked_sub(dc).ok_or_else(unreadable)?)
            .ok_or_else(unreadable)?;
    }
    let lines = hundredths(covered, count)
        .ok_or("sherd/debt:V10: no line left under `Sources/`")?;
    Ok(SwiftMeasure { lines, excluded })
}

/// The refusal for an exclusion that matches nothing.
fn stale(row: &Exclusion, what: &str) -> String {
    let target = row
        .function
        .as_ref()
        .map_or_else(|| row.path.clone(), |f| format!("{}:{f}", row.path));
    format!(
        "sherd/debt:V11: `exclude {target}` matches {what} -- a stale \
         exclusion is deleted, never kept"
    )
}

/// The name a row matches: the LAST word of the simplified demangling, so
/// `static Keys.load(tag:)` is `Keys.load(tag:)` and `closure #1 in
/// helper()` goes with `helper()`.
fn short(demangled: &str) -> &str {
    demangled.split_whitespace().last().unwrap_or(demangled)
}

/// Every function row on a wholly excluded file must still name one.
fn check_rows(
    fns: &[&String],
    path: &str,
    functions: &[Function],
    names: &[String],
) -> Result<(), String> {
    for f in fns {
        let hit = functions
            .iter()
            .zip(names)
            .any(|(func, n)| func.file == path && short(n) == f.as_str());
        if !hit {
            return Err(stale(
                &row_of(path, f),
                "no function llvm-cov measured",
            ));
        }
    }
    Ok(())
}

fn row_of(path: &str, f: &str) -> Exclusion {
    Exclusion {
        path: path.to_string(),
        function: Some(f.to_string()),
    }
}

/// The lines, and covered lines, the `fns` rows remove from `path`: each
/// matched GROUP's own count, once. `Err` when a row matches nothing, or
/// the groups do not sum to `summary` -- a count sherd cannot reproduce is
/// one it may not subtract from.
fn dropped(
    path: &str,
    summary: (usize, usize),
    fns: &[&String],
    functions: &[Function],
    names: &[String],
) -> Result<(usize, usize), String> {
    // (first region line, col) -> (lines, covered, matched)
    let mut groups: std::collections::BTreeMap<
        (usize, usize),
        (usize, usize, bool),
    > = std::collections::BTreeMap::new();
    let mut hits = vec![false; fns.len()];
    for (f, n) in functions.iter().zip(names).filter(|(f, _)| f.file == path) {
        let Some(first) = f.regions.first() else {
            continue;
        };
        let (lines, cov) = function_lines(&f.regions);
        let g = groups.entry((first.ls, first.cs)).or_insert((0, 0, false));
        g.0 = g.0.max(lines);
        g.1 = g.1.max(cov);
        for (hit, row) in hits.iter_mut().zip(fns) {
            if short(n) == row.as_str() {
                *hit = true;
                g.2 = true;
            }
        }
    }
    if let Some((_, row)) = hits.iter().zip(fns).find(|(h, _)| !**h) {
        return Err(stale(&row_of(path, row), "no function llvm-cov measured"));
    }
    let sum = groups.values().try_fold((0_usize, 0_usize), |(a, b), g| {
        Some((a.checked_add(g.0)?, b.checked_add(g.1)?))
    });
    if sum != Some(summary) {
        return Err(format!(
            "sherd/debt:V11: {path}: its functions' lines do not sum to \
             llvm-cov's {}/{} -- refusing to subtract from a count sherd \
             cannot reproduce",
            summary.1, summary.0
        ));
    }
    Ok(groups.values().filter(|g| g.2).fold((0, 0), |(a, b), g| {
        (a.saturating_add(g.0), b.saturating_add(g.1))
    }))
}

/// `swift demangle --simplified` over `names`, one line each, or `None`
/// when it did not run (`V3`).
fn swift_demangle(
    root: &Path,
    swift: &str,
    names: &[String],
) -> Option<Vec<String>> {
    use std::io::Write as _;
    let mut child = Command::new(swift)
        .args(["demangle", "--simplified"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let input = format!("{}\n", names.join("\n"));
    // Written from its own thread: a demangler that answers as it reads
    // would fill stdout while this side still writes, and both would wait.
    let o = std::thread::scope(|s| {
        let writer = s.spawn(move || stdin.write_all(input.as_bytes()));
        let o = child.wait_with_output().ok()?;
        writer.join().ok()?.ok()?;
        Some(o)
    })?;
    o.status.success().then(|| {
        String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(str::to_string)
            .collect()
    })
}

/// `root`'s SwiftPM figure with `.coverage`'s exclusions (`V10`, `V11`).
///
/// # Errors
/// Either `swift test` run failed, or [`measure_export`] refused.
pub fn swift_measure(root: &Path, swift: &str) -> Result<SwiftMeasure, String> {
    let json = swift_run(root, swift).ok_or_else(|| {
        format!(
            "could not read a coverage total from `{swift} test` -- that is \
             an ERROR, not a floor breach (sherd/debt:V10)"
        )
    })?;
    let demangle = |n: &[String]| swift_demangle(root, swift, n);
    measure_export(&json, root, &exclusions(root), &demangle)
}

/// Did the excluded lines exceed the cap? The report, and whether it passes.
#[must_use]
pub fn excluded_verdict(now: usize, cap: usize) -> (bool, String) {
    if now > cap {
        return (
            false,
            format!(
                "sherd/debt:V11: exclusions remove {now} lines, over the \
                 cap of {cap}. Cover them, or raise `excluded` by hand in a \
                 reviewed commit that says why."
            ),
        );
    }
    (true, format!("  {now} lines (cap {cap})"))
}

/// Rewrite `.coverage`'s `excluded` row to `now`, refusing a RAISE -- the
/// cap may only fall, as the floor may only rise (`V6`, `V11`).
///
/// # Errors
/// `now` is over the cap, or the file could not be read or written.
pub fn record_excluded(root: &Path, now: usize) -> Result<String, String> {
    let cap = excluded_cap(root);
    if now > cap.unwrap_or(0) {
        return Err(format!(
            "refusing to RAISE `excluded`, {} -> {now}. Cover the lines, or \
             edit .coverage by hand with the reason.",
            cap.unwrap_or(0)
        ));
    }
    if cap.is_none() {
        return Ok("no exclusions, no `excluded` row".into());
    }
    let path = root.join(".coverage");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{e}"))?;
    let out: String = text.lines().fold(String::new(), |mut acc, l| {
        if l.starts_with("excluded ") {
            acc.push_str(&format!("excluded {now}"));
        } else {
            acc.push_str(l);
        }
        acc.push('\n');
        acc
    });
    std::fs::write(&path, &out).map_err(|e| format!("{e}"))?;
    Ok(format!("recorded `excluded` {now} lines"))
}

#[cfg(test)]
#[path = "tests/swift_fn.rs"]
pub(crate) mod swift_fn_tests;
