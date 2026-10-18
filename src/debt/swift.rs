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

/// The line percentage of `root`'s `Sources/` under SwiftPM, in hundredths,
/// with `.coverage`'s exclusions applied (`V11`). `None` wherever
/// [`super::swift_measure`] is an error.
#[must_use]
pub fn swift_coverage(root: &Path, swift: &str) -> Option<usize> {
    super::swift_measure(root, swift).ok().map(|m| m.lines)
}

/// Both `swift test` runs, then the llvm-cov JSON the second names. `None`
/// when either run FAILED. The status is checked before the file is read:
/// a previous run's JSON is still on disk, and reading it would report an
/// old tree's number for this one (`V3`, `.:V118`).
pub(super) fn swift_run(root: &Path, swift: &str) -> Option<String> {
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
    std::fs::read_to_string(root.join(path)).ok()
}

/// The `Sources/` files `.coverage` EXCLUDES, as root-relative paths: one
/// `exclude <path> <reason>` row each (`V10`). A row with no reason is not an
/// exclusion -- an unexplained one is how a floor gets quietly loosened. A
/// `<path>:<function>` row is one function, not the file (`V11`).
#[must_use]
pub fn excluded_files(root: &Path) -> Vec<String> {
    super::exclusions(root)
        .into_iter()
        .filter(|e| e.function.is_none())
        .map(|e| e.path)
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
    let rows: Vec<super::Exclusion> = excluded
        .iter()
        .map(|p| super::Exclusion {
            path: p.clone(),
            function: None,
        })
        .collect();
    let never = |_: &[String]| None;
    super::measure_export(json, root, &rows, &never)
        .ok()
        .map(|m| m.lines)
}

/// `covered / count` in hundredths, rounded half up as `%.2f` prints it.
/// `None` for no lines: nothing was measured.
pub(super) fn hundredths(covered: usize, count: usize) -> Option<usize> {
    let scale = COVERAGE_SCALE.checked_mul(COVERAGE_SCALE)?;
    let twice = covered.checked_mul(scale)?.checked_mul(2)?;
    twice.checked_add(count)?.checked_div(count.checked_mul(2)?)
}

/// Is `file` under one of `roots`' `Sources/`? A relative `file` is read
/// against the root; an absolute one outside every root is not a source.
pub(super) fn is_source(
    file: &Path,
    roots: &[Option<std::path::PathBuf>],
) -> bool {
    let sources = Path::new("Sources");
    if file.is_relative() {
        return file.starts_with(sources);
    }
    roots.iter().flatten().any(|r| {
        file.strip_prefix(r)
            .is_ok_and(|rel| rel.starts_with(sources))
    })
}

/// A JSON string value and the text after it. `\"`, `\\` and `\/` are
/// unescaped; a path needs no other escape.
pub(super) fn string_of(v: &str) -> Option<(String, &str)> {
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
pub(super) fn after_key<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let quoted = format!("\"{key}\"");
    let at = json.find(&quoted)?.checked_add(quoted.len())?;
    json.get(at..)?.trim_start().strip_prefix(':')
}

/// The unsigned integer `"key"` holds, within one flat JSON object.
pub(super) fn number_of(object: &str, key: &str) -> Option<usize> {
    let v = after_key(object, key)?.trim_start();
    let end = v.find(|c: char| !c.is_ascii_digit()).unwrap_or(v.len());
    v.get(..end)?.parse().ok()
}

#[cfg(test)]
#[path = "tests/swift.rs"]
pub(crate) mod swift_tests;
