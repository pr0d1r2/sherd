//! A RATCHET: measure, compare to a recorded floor, refuse the wrong way.
//!
//! Promoted because the rule was stated three times -- twice in `hk.pkl` as
//! awk and once in `src/land` as Rust -- and the Rust copy diverged from the
//! gate on its flags, its file set AND its denominator at once, reporting
//! 10.8 where the gate computed 14.6 (`.:B25`). Three readings of one rule is
//! `.:B13`, pointed at the gate itself.
//!
//! What the gate reads is stated ONCE here. `src/land` calls it; `hk.pkl`
//! will (`T1`).

use std::path::Path;
use std::process::Command;

/// The gate's own clippy invocation. A different build is a different number
/// (`V4`).
pub const GATE_ARGS: [&str; 5] = [
    "clippy",
    "--workspace",
    "--all-targets",
    "--all-features",
    "--message-format=short",
];

/// The directories the gate measures. ONE list, so the numerator and the
/// denominator cannot drift apart (`§C`).
pub const MEASURED: [&str; 2] = ["src", "dev"];

/// Ceiling for the CODE half of one `.rs` file (`V8`).
pub const CEILING_FILE: u64 = 4_000;

/// Ceiling for the TEST half of one `.rs` file (`V8`).
pub const CEILING_TEST: u64 = 2_000;

/// Does `hk` count this line? `^(src|dev)/.*: warning`.
#[must_use]
pub fn gate_counts(line: &str) -> bool {
    MEASURED.iter().any(|d| line.starts_with(&format!("{d}/")))
        && line.contains(": warning")
}

/// Did the build fail? Then there is nothing to count (`V3`).
#[must_use]
pub fn build_failed(line: &str) -> bool {
    line.starts_with("error[") || line.starts_with("error:")
}

/// Tenths as the number a reader sees: `170` is `17.0`.
#[must_use]
pub fn per_kloc(tenths: usize) -> String {
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// Lines of Rust the gate divides by.
///
/// One walker (`§C`): `src/fed` owns the traversal and this asks it, rather
/// than becoming the fourth reading of "the `.rs` files in this tree".
#[must_use]
pub fn measured_lines(root: &Path) -> usize {
    MEASURED
        .iter()
        .flat_map(|d| crate::fed::rust_files(&root.join(d)))
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .map(|t| t.lines().count())
        .sum()
}

/// Warnings per thousand lines, in TENTHS, measured exactly as the gate does.
///
/// Integer tenths because every comparison in the loop is `usize` and a float
/// has no business next to a gate.
///
/// `None` when clippy could not run, the build failed, or there is no Rust to
/// divide by: there is simply nothing to compare, and refusing would block
/// every candidate on a bench problem (`src/tdd:V26`).
#[must_use]
pub fn density(root: &Path, cargo: &str) -> Option<usize> {
    let o = Command::new(cargo)
        .args(GATE_ARGS)
        .current_dir(root)
        .output()
        .ok()?;
    let out = String::from_utf8_lossy(&o.stderr);
    if out.lines().any(build_failed) {
        return None;
    }
    let n = out.lines().filter(|l| gate_counts(l)).count();
    let loc = measured_lines(root);
    (loc > 0).then(|| n.saturating_mul(10_000) / loc)
}

/// The `density` ceiling from `.lint-debt`, in TENTHS, if the file is there.
#[must_use]
pub fn recorded(root: &Path) -> Option<usize> {
    let text = std::fs::read_to_string(root.join(".lint-debt")).ok()?;
    text.lines()
        .find_map(|l| tenths(l.strip_prefix("density ")?))
}

/// `"14.6"` as `146`. A missing decimal reads as `.0`.
fn tenths(v: &str) -> Option<usize> {
    let v = v.trim();
    let (whole, frac) = v.split_once('.').unwrap_or((v, "0"));
    let tenth = frac.chars().next()?.to_digit(10)? as usize;
    whole
        .parse::<usize>()
        .ok()?
        .checked_mul(10)?
        .checked_add(tenth)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `V5`: the loop's inputs are the gate's, asserted against `hk.pkl`'s
    /// OWN TEXT. A test hardcoding today's ratio passes while the two drift.
    #[test]
    fn every_input_is_the_gate_s_own() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pkl =
            std::fs::read_to_string(root.join("hk.pkl")).unwrap_or_default();
        assert!(!pkl.is_empty(), "this repository has a gate");
        for flag in GATE_ARGS.iter().skip(1) {
            assert!(pkl.contains(flag), "the gate stopped passing {flag}");
        }
        assert!(
            pkl.contains("'^(src|dev)/.*: warning'"),
            "the gate's warning filter changed; `gate_counts` mirrors it"
        );
        assert!(
            pkl.contains(r#"find src dev -name "*.rs""#),
            "the gate's denominator changed; `MEASURED` mirrors it"
        );
    }

    /// The mirror is honest in both directions.
    #[test]
    fn the_filter_counts_src_and_dev_warnings_only() {
        assert!(gate_counts("src/a.rs:1:1: warning: x"));
        assert!(gate_counts("dev/b.rs:1:1: warning: x"));
        assert!(!gate_counts("tests/c.rs:1:1: warning: x"));
        assert!(!gate_counts("src/a.rs:1:1: error: x"));
        assert!(build_failed("error[E0425]: x"));
        assert!(!build_failed("warning: x"));
    }

    /// `V3`/`V6`: the recorded ceiling is read BEFORE anything is measured,
    /// and a missing file means "no ratchet here", never "zero allowed".
    #[test]
    fn a_recorded_ceiling_is_tenths_and_absence_is_not_zero() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(
            recorded(root).is_some_and(|t| t > 0),
            "this repo records one"
        );
        assert_eq!(recorded(Path::new("/definitely-not-a-repo")), None);
        assert_eq!(tenths("14.6"), Some(146));
        assert_eq!(tenths("9"), Some(90));
        assert_eq!(per_kloc(146), "14.6");
    }

    /// The denominator is the gate's, and it is not zero on this tree.
    #[test]
    fn the_denominator_counts_both_measured_directories() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let all = measured_lines(root);
        let src_only = crate::fed::rust_files(&root.join("src"))
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .map(|t| t.lines().count())
            .sum::<usize>();
        assert!(all > src_only, "`dev` is measured too: {all} vs {src_only}");
    }
}
