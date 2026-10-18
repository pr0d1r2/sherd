//! Line coverage for a SwiftPM package (`V10`, #138).
//!
//! SwiftPM writes the same LLVM data `cargo llvm-cov` reads: `swift test
//! --enable-code-coverage` runs the suite and `swift test
//! --show-codecov-path` names llvm-cov's JSON export. Only the MEASUREMENT
//! differs; the floor, its row and its verdict are the cargo ones.

use super::COVERAGE_SCALE;
use std::path::Path;
use std::process::Command;

/// Which toolchain measures a tree's coverage (`V10`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toolchain {
    /// `cargo llvm-cov`. Any tree with a `Cargo.toml` at its root.
    Cargo,
    /// `swift test`. `Package.swift` at the root and no `Cargo.toml`.
    SwiftPm,
}

/// The toolchain `root` is measured with. A `Cargo.toml` WINS, so a cargo
/// repository measures exactly as it did before SwiftPM was read (`V10`).
#[must_use]
pub fn toolchain_of(root: &Path) -> Toolchain {
    if root.join("Package.swift").is_file() && !root.join("Cargo.toml").exists()
    {
        return Toolchain::SwiftPm;
    }
    Toolchain::Cargo
}

/// The `swift` binary, from `SHERD_SWIFT` or the default. The edges read
/// the env; a test passes a scripted one instead.
#[must_use]
pub fn swift_bin() -> String {
    std::env::var("SHERD_SWIFT").unwrap_or_else(|_| "swift".into())
}

/// The line percentage of `root`'s `Sources/` under SwiftPM, in hundredths.
///
/// `None` when either `swift test` run FAILED or the JSON has no line
/// under `Sources/` ([`llvm_cov_lines`]). The status is checked before
/// the file is read: a previous run's JSON is still on disk, and reading it
/// would report an old tree's number for this one (`V3`, `.:V118`).
#[must_use]
pub fn swift_coverage(root: &Path, swift: &str) -> Option<usize> {
    let run = |args: &[&str]| {
        let o = Command::new(swift)
            .args(args)
            .current_dir(root)
            .output()
            .ok()?;
        o.status.success().then_some(o.stdout)
    };
    run(&["test", "--enable-code-coverage"])?;
    let shown = run(&["test", "--show-codecov-path"])?;
    let shown = String::from_utf8_lossy(&shown);
    let path = shown.lines().map(str::trim).rfind(|l| !l.is_empty())?;
    let json = std::fs::read_to_string(root.join(path)).ok()?;
    llvm_cov_lines_except(&json, root, &excluded_files(root))
}

/// The `Sources/` files `.coverage` EXCLUDES, as root-relative paths: one
/// `exclude <path> <reason>` row each (`V10`). A row with no reason is not an
/// exclusion -- an unexplained one is how a floor gets quietly loosened.
#[must_use]
pub fn excluded_files(root: &Path) -> Vec<String> {
    let text =
        std::fs::read_to_string(root.join(".coverage")).unwrap_or_default();
    text.lines()
        .filter_map(|l| {
            let mut w = l.strip_prefix("exclude ")?.split_whitespace();
            let path = w.next()?;
            w.next()?;
            Some(path.trim_start_matches("./").to_string())
        })
        .collect()
}

/// The line percentage of the package's OWN sources in an llvm-cov JSON
/// export, in hundredths and rounded the way llvm-cov prints a percentage.
///
/// Σ `covered` / Σ `count` of `summary.lines` over the `data[0].files[]`
/// entries under `root`'s `Sources/` (`V10`). Not `data[0].totals`, which
/// also counts `Tests/` and the `.build/` derived test runner. `None` when
/// no line is under `Sources/`, or an entry's `summary` cannot be read.
#[must_use]
pub fn llvm_cov_lines(json: &str, root: &Path) -> Option<usize> {
    llvm_cov_lines_except(json, root, &[])
}

/// [`llvm_cov_lines`] without the root-relative `excluded` files, which
/// leave the numerator AND the denominator. `None` when nothing is left.
#[must_use]
pub fn llvm_cov_lines_except(
    json: &str,
    root: &Path,
    excluded: &[String],
) -> Option<usize> {
    let roots = [Some(root.to_path_buf()), root.canonicalize().ok()];
    let (mut count, mut covered) = (0_usize, 0_usize);
    let mut rest = json;
    while let Some(entry) = after_key(rest, "filename") {
        let (name, lines, tail) = file_lines(entry)?;
        rest = tail;
        if is_source(Path::new(&name), &roots)
            && !is_excluded(Path::new(&name), &roots, excluded)
        {
            count = count.checked_add(number_of(lines, "count")?)?;
            covered = covered.checked_add(number_of(lines, "covered")?)?;
        }
    }
    hundredths(covered, count)
}

/// One `files[]` entry from its `filename` value on: the name, its
/// `summary.lines` object, and the text after the name. `None` when the
/// next `summary` belongs to a LATER file -- this one has none.
fn file_lines(entry: &str) -> Option<(String, &str, &str)> {
    let (name, rest) = string_of(entry)?;
    let summary = after_key(rest, "summary")?;
    let next = rest.find("\"filename\"").unwrap_or(rest.len());
    if rest.len().checked_sub(summary.len())? > next {
        return None;
    }
    let lines = after_key(summary, "lines")?;
    Some((name, lines.get(..lines.find('}')?)?, rest))
}

/// `covered / count` in hundredths, rounded half up as `%.2f` prints it.
/// `None` for no lines: nothing was measured.
fn hundredths(covered: usize, count: usize) -> Option<usize> {
    let scale = COVERAGE_SCALE.checked_mul(COVERAGE_SCALE)?;
    let twice = covered.checked_mul(scale)?.checked_mul(2)?;
    twice.checked_add(count)?.checked_div(count.checked_mul(2)?)
}

/// Is `file` under one of `roots`' `Sources/`? A relative `file` is read
/// against the root; an absolute one outside every root is not a source.
fn is_source(file: &Path, roots: &[Option<std::path::PathBuf>]) -> bool {
    let sources = Path::new("Sources");
    if file.is_relative() {
        return file.starts_with(sources);
    }
    roots.iter().flatten().any(|r| {
        file.strip_prefix(r)
            .is_ok_and(|rel| rel.starts_with(sources))
    })
}

/// Is `file` one of the `excluded` root-relative paths?
fn is_excluded(
    file: &Path,
    roots: &[Option<std::path::PathBuf>],
    excluded: &[String],
) -> bool {
    let rel = if file.is_relative() {
        Some(file)
    } else {
        roots
            .iter()
            .flatten()
            .find_map(|r| file.strip_prefix(r).ok())
    };
    rel.is_some_and(|rel| excluded.iter().any(|e| rel == Path::new(e)))
}

/// A JSON string value and the text after it. `\"`, `\\` and `\/` are
/// unescaped; a path needs no other escape.
fn string_of(v: &str) -> Option<(String, &str)> {
    let body = v.trim_start().strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = body.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((out, body.get(i.checked_add(1)?..)?)),
            '\\' => out.push(chars.next()?.1),
            _ => out.push(c),
        }
    }
    None
}

/// The text after `"key"` and its `:`, or `None` when the key is absent.
fn after_key<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let quoted = format!("\"{key}\"");
    let at = json.find(&quoted)?.checked_add(quoted.len())?;
    json.get(at..)?.trim_start().strip_prefix(':')
}

/// The unsigned integer `"key"` holds, within one flat JSON object.
fn number_of(object: &str, key: &str) -> Option<usize> {
    let v = after_key(object, key)?.trim_start();
    let end = v.find(|c: char| !c.is_ascii_digit()).unwrap_or(v.len());
    v.get(..end)?.parse().ok()
}

#[cfg(test)]
#[path = "tests/swift.rs"]
pub(crate) mod swift_tests;
