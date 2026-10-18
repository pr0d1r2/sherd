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

/// SwiftPM's TOTAL line percentage for `root`, in hundredths.
///
/// `None` when either `swift test` run FAILED or the JSON has no line
/// total. The status is checked before the file is read: a previous run's
/// JSON is still on disk, and reading it would report an old tree's number
/// for this one (`V3`, `.:V118`).
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
    llvm_cov_lines(&std::fs::read_to_string(root.join(path)).ok()?)
}

/// `data[0].totals.lines` of an llvm-cov JSON export, in hundredths and
/// rounded the way llvm-cov prints a percentage.
#[must_use]
pub fn llvm_cov_lines(json: &str) -> Option<usize> {
    let totals = after_key(json, "totals")?;
    let lines = after_key(totals, "lines")?;
    let lines = lines.get(..lines.find('}')?)?;
    let count = number_of(lines, "count")?;
    let covered = number_of(lines, "covered")?;
    // Round half up: `covered / count` in hundredths, as `%.2f` prints it.
    let scale = COVERAGE_SCALE.checked_mul(COVERAGE_SCALE)?;
    let twice = covered.checked_mul(scale)?.checked_mul(2)?;
    twice.checked_add(count)?.checked_div(count.checked_mul(2)?)
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
