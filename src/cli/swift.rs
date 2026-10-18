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
/// `lines-default` row is a cargo feature split and is left as it is.
fn swift_coverage_cmd(
    root: &Path,
    mode: Option<&str>,
    swift: &str,
) -> ExitCode {
    use crate::debt::Build;
    let Some((was, now)) = swift_floor_and_now(root, swift) else {
        return ExitCode::from(2);
    };
    ExitCode::from(if mode == Some("--record") {
        record_coverage(root, Build::All, now)
    } else {
        check_coverage(Build::All.row(), now, was)
    })
}

/// The `lines` floor and the SwiftPM measurement, or `None` once the
/// missing one is named: either is an ERROR, never a breach (`.:V48`).
fn swift_floor_and_now(root: &Path, swift: &str) -> Option<(usize, usize)> {
    let Some(was) = crate::debt::recorded_floor(root) else {
        let path = root.join(".coverage");
        eprintln!("sherd: {}: no `lines` row to read", path.display());
        return None;
    };
    let now = crate::debt::swift_coverage(root, swift);
    if now.is_none() {
        eprintln!(
            "sherd: could not read a coverage total from `{swift} test` -- \
             that is an ERROR, not a floor breach (sherd/debt:V10)"
        );
    }
    Some((was, now?))
}

#[cfg(test)]
#[path = "tests/swift.rs"]
mod tests;
