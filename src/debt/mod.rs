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

/// Everything one clippy run tells the ratchet.
///
/// One struct because the two gated ratios share a run and a denominator:
/// computing them separately invites exactly the drift `B6` records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Measured {
    /// Warnings the gate counts.
    pub count: usize,
    /// Lines over the limit, summed across every function that exceeds it.
    pub excess: usize,
    /// Lines of Rust in the measured directories.
    pub loc: usize,
}

impl Measured {
    /// Warnings per thousand lines, in TENTHS.
    #[must_use]
    pub const fn density(self) -> usize {
        // `checked_div` rather than a guard plus `/`: a zero denominator is
        // "nothing measured", which is what `0` means here, and stating it
        // once beats stating it twice.
        match self.count.saturating_mul(10_000).checked_div(self.loc) {
            Some(d) => d,
            None => 0,
        }
    }

    /// Share of the tree inside an over-long function, in TENTHS of a
    /// percent -- `92` is 9.2%.
    #[must_use]
    pub const fn shape(self) -> usize {
        match self.excess.saturating_mul(1_000).checked_div(self.loc) {
            Some(s) => s,
            None => 0,
        }
    }
}

/// Lines over `too_many_lines`' limit on one warning line, if it is one.
///
/// `(269/15)` is 254. The LIMIT is read from the message rather than assumed,
/// so raising it in `clippy.toml` cannot leave this measuring the old one.
#[must_use]
pub fn excess_lines(line: &str) -> Option<usize> {
    let (_, tail) = line.split_once("too many lines (")?;
    let (nums, _) = tail.split_once(')')?;
    let (had, limit) = nums.split_once('/')?;
    Some(
        had.parse::<usize>()
            .ok()?
            .saturating_sub(limit.parse().ok()?),
    )
}

/// Measure the tree exactly as the gate does.
///
/// `None` when clippy could not run or the build failed: a count from a build
/// that did not compile is not a measurement (`V3`).
#[must_use]
pub fn measure(root: &Path, cargo: &str) -> Option<Measured> {
    let o = Command::new(cargo)
        .args(GATE_ARGS)
        .current_dir(root)
        .output()
        .ok()?;
    let out = String::from_utf8_lossy(&o.stderr);
    if out.lines().any(build_failed) {
        return None;
    }
    // No denominator is NOTHING MEASURED, never "zero debt": a ratio over
    // an empty tree would report PASS at 0.0 and claim a tree nobody built
    // is clean (`V3`). Caught by the test asserting silence on a tree with
    // no Rust, after `Measured` replaced a fn that returned `None` here.
    let loc = measured_lines(root);
    if loc == 0 {
        return None;
    }
    let hits: Vec<&str> = out.lines().filter(|l| gate_counts(l)).collect();
    Some(Measured {
        count: hits.len(),
        excess: hits.iter().filter_map(|l| excess_lines(l)).sum(),
        loc,
    })
}

/// The two gated ceilings from `.lint-debt`, in TENTHS.
#[must_use]
pub fn recorded_ceilings(root: &Path) -> Option<(usize, usize)> {
    let text = std::fs::read_to_string(root.join(".lint-debt")).ok()?;
    let read = |k: &str| text.lines().find_map(|l| tenths(l.strip_prefix(k)?));
    Some((read("density ")?, read("shape ")?))
}

/// The density breach message, if it rose.
fn breach_density(now: usize, was: usize) -> Option<String> {
    (now > was).then(|| {
        format!(
            "sherd/debt:V2: lint DENSITY rose, {} -> {} per KLoC. Fix the \
             new sites, or record WHY in .lint-debt with the raise.",
            per_kloc(was),
            per_kloc(now)
        )
    })
}

/// The shape breach message, if it rose.
fn breach_shape(now: usize, was: usize) -> Option<String> {
    (now > was).then(|| {
        format!(
            "sherd/debt:V1: SHAPE rose, {}% -> {}% of lines inside an \
             over-long function. Counting functions punishes splitting one, \
             so this counts lines OVER the limit.",
            per_kloc(was),
            per_kloc(now)
        )
    })
}

/// Did either ratio RISE? The report, and whether it refuses.
///
/// Both are checked and both are reported, because a run that stops at the
/// first breach hides the second and the next commit meets it alone.
#[must_use]
pub fn verdict(now: Measured, was: (usize, usize)) -> (bool, String) {
    let (dw, sw) = was;
    let (d, s) = (now.density(), now.shape());
    let mut lines: Vec<String> = [breach_density(d, dw), breach_shape(s, sw)]
        .into_iter()
        .flatten()
        .collect();
    let ok = lines.is_empty();
    // `V48`: state what was EXAMINED, not only what failed.
    lines.push(format!(
        "  lint {} per KLoC (ceiling {}) · shape {}% (ceiling {}%) · {} \
         warnings and {} excess lines over {} lines of Rust",
        per_kloc(d),
        per_kloc(dw),
        per_kloc(s),
        per_kloc(sw),
        now.count,
        now.excess,
        now.loc
    ));
    (ok, lines.join("\n"))
}

/// One `.lint-debt` line, with its number brought current. Every other line
/// -- every comment, every recorded reason -- passes through untouched: the
/// file is the audit trail (`V7`).
fn restate(line: &str, now: Measured) -> String {
    match line.split_once(' ').map(|(k, _)| k) {
        Some("density") => format!("density {}", per_kloc(now.density())),
        Some("shape") => format!("shape {}", per_kloc(now.shape())),
        Some("count") => format!("count {}", now.count),
        Some("excess") => format!("excess {}", now.excess),
        Some("loc") => format!("loc {}", now.loc),
        _ => line.to_string(),
    }
}

/// Rewrite `.lint-debt`'s numbers, refusing a RAISE (`V6`).
///
/// # Errors
/// The file could not be read or written, or either ratio rose -- a ratchet
/// that writes down whatever it measures is not a ratchet.
pub fn record(root: &Path, now: Measured) -> Result<String, String> {
    let path = root.join(".lint-debt");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{e}"))?;
    let was = recorded_ceilings(root).ok_or_else(|| {
        "no `density`/`shape` rows to record into".to_string()
    })?;
    if !verdict(now, was).0 {
        return Err(
            "refusing to RECORD a raise. A ratchet that writes down whatever \
             it measures is not a ratchet."
                .into(),
        );
    }
    let out: String = text.lines().fold(String::new(), |mut acc, l| {
        acc.push_str(&restate(l, now));
        acc.push('\n');
        acc
    });
    std::fs::write(&path, &out).map_err(|e| format!("{e}"))?;
    Ok(format!(
        "recorded density {} · shape {}%",
        per_kloc(now.density()),
        per_kloc(now.shape())
    ))
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

/// The coverage floor, in HUNDREDTHS. `9227` is `92.27%`.
///
/// Hundredths, where the lint ratios are tenths: `.coverage` has always
/// carried two decimals and rounding it to one would move the floor by up to
/// five hundredths, which is more than most of today's real changes.
pub const COVERAGE_SCALE: usize = 100;

/// `cargo llvm-cov`'s TOTAL line percentage, in hundredths.
///
/// The MIRROR of `density`: this one may only RISE. Everything else is the
/// same shape, which is why it lives here (`T2`).
///
/// `None` when the tool could not run or printed no TOTAL -- a gate that did
/// not EXECUTE is an ERROR, not a floor breach (`src/tdd:V26`).
#[must_use]
pub fn coverage(root: &Path, cargo: &str) -> Option<usize> {
    let o = Command::new(cargo)
        .args([
            "llvm-cov",
            "--workspace",
            "--all-features",
            "--summary-only",
        ])
        .current_dir(root)
        .output()
        .ok()?;
    if !o.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&o.stdout);
    let total = text.lines().find(|l| l.starts_with("TOTAL"))?;
    // itok's format: the tenth field is the line percentage.
    hundredths(total.split_whitespace().nth(9)?.trim_end_matches('%'))
}

/// The floor `.coverage` records, in hundredths.
#[must_use]
pub fn recorded_floor(root: &Path) -> Option<usize> {
    let text = std::fs::read_to_string(root.join(".coverage")).ok()?;
    text.lines()
        .find_map(|l| hundredths(l.strip_prefix("lines ")?))
}

/// `"92.27"` as `9227`. One decimal or none still parses.
fn hundredths(v: &str) -> Option<usize> {
    let v = v.trim();
    let (whole, frac) = v.split_once('.').unwrap_or((v, "0"));
    let mut d = frac.chars().filter(char::is_ascii_digit);
    let tens = d.next()?.to_digit(10)? as usize;
    let ones = d.next().and_then(|c| c.to_digit(10)).unwrap_or(0) as usize;
    whole
        .parse::<usize>()
        .ok()?
        .checked_mul(COVERAGE_SCALE)?
        .checked_add(tens.saturating_mul(10).saturating_add(ones))
}

/// Hundredths as the number a reader sees: `9227` is `92.27`.
#[must_use]
pub fn percent(h: usize) -> String {
    format!("{}.{:02}", h / COVERAGE_SCALE, h % COVERAGE_SCALE)
}

/// Did coverage FALL? The report, and whether it refuses.
#[must_use]
pub fn coverage_verdict(now: usize, was: usize) -> (bool, String) {
    if now < was {
        return (
            false,
            format!(
                "sherd/debt:V9: coverage FELL, {}% -> {}%. A line that \
                 stopped being covered is a test that stopped asserting. \
                 Cover it, or say why in .coverage with the drop.",
                percent(was),
                percent(now)
            ),
        );
    }
    // `.:V48`: state what was examined either way.
    (
        true,
        format!("  coverage {}% (floor {}%)", percent(now), percent(was)),
    )
}

/// Rewrite `.coverage`'s number, refusing a DROP.
///
/// # Errors
/// The file could not be read or written, or coverage fell -- recording a
/// drop is filing down the ratchet's own teeth.
pub fn record_coverage(root: &Path, now: usize) -> Result<String, String> {
    let path = root.join(".coverage");
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{e}"))?;
    let was = recorded_floor(root)
        .ok_or_else(|| "no `lines` row to record into".to_string())?;
    if !coverage_verdict(now, was).0 {
        return Err(format!(
            "refusing to RECORD a drop, {}% -> {}%. Cover the gap, or edit \
             .coverage by hand with the reason.",
            percent(was),
            percent(now)
        ));
    }
    let out: String = text.lines().fold(String::new(), |mut acc, l| {
        if l.starts_with("lines ") {
            acc.push_str(&format!("lines {}", percent(now)));
        } else {
            acc.push_str(l);
        }
        acc.push('\n');
        acc
    });
    std::fs::write(&path, &out).map_err(|e| format!("{e}"))?;
    Ok(format!("recorded coverage {}%", percent(now)))
}

#[cfg(test)]
#[path = "tests/debt.rs"]
mod tests;
