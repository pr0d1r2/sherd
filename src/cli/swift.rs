//! `sherd coverage` for a SwiftPM package (`src/debt:V10`, #138). Apart
//! from `measure` so the cargo path there stays as it was.

use super::*;

/// `sherd coverage` in whichever toolchain measures `root` (`src/debt:V10`).
/// A cargo tree goes to [`coverage_cmd`] unchanged.
pub(super) fn coverage_in(
    root: &Path,
    mode: Option<&str>,
    cargo: &str,
    swift: &str,
) -> ExitCode {
    match crate::debt::toolchain_of(root) {
        crate::debt::Toolchain::SwiftPm => {
            swift_coverage_cmd(root, mode, swift)
        }
        crate::debt::Toolchain::Cargo => coverage_cmd(root, mode, cargo),
    }
}

/// A SwiftPM package has ONE build, so only `lines` is measured; a
/// `lines-default` row is a cargo feature split and is left as it is. The
/// `excluded` cap is checked beside the floor, and a breach of it records
/// NOTHING: a floor raised by lines the cap refuses was not earned
/// (`src/debt:V11`).
fn swift_coverage_cmd(
    root: &Path,
    mode: Option<&str>,
    swift: &str,
) -> ExitCode {
    use crate::debt::Build;
    let Some(was) = crate::debt::recorded_floor(root) else {
        let path = root.join(".coverage");
        eprintln!("sherd: {}: no `lines` row to read", path.display());
        return ExitCode::from(2);
    };
    let m = match crate::debt::swift_measure(root, swift) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("sherd: {e}");
            return ExitCode::from(2);
        }
    };
    let cap = crate::debt::excluded_cap(root).unwrap_or(0);
    let (fits, report) = crate::debt::excluded_verdict(m.excluded, cap);
    if !fits {
        eprintln!("excluded: {report}");
        return ExitCode::from(1);
    }
    println!("  excluded: {}", report.trim_start());
    ExitCode::from(if mode == Some("--record") {
        record_coverage(root, Build::All, m.lines)
            .max(record_excluded(root, m.excluded))
    } else {
        check_coverage(Build::All.row(), m.lines, was)
    })
}

/// `--record` for the cap: lower it, refusing a raise.
fn record_excluded(root: &Path, now: usize) -> u8 {
    match crate::debt::record_excluded(root, now) {
        Ok(msg) => {
            println!("{msg}");
            0
        }
        Err(e) => {
            eprintln!("sherd: {e}");
            1
        }
    }
}

#[cfg(test)]
#[path = "tests/swift.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/swift_fn.rs"]
mod fn_tests;
