use super::*;

/// `V5`, in its stronger form: there is no SECOND statement to drift
/// from. The gate CALLS `sherd debt`; it does not re-derive the formula.
///
/// This test used to assert that `hk.pkl`'s flags matched `GATE_ARGS`,
/// which was the best available check while the rule was written twice.
/// Asserting the absence is better: a mirror can be kept faithfully and
/// still be two things.
#[test]
fn the_gate_calls_this_rather_than_restating_it() {
    // Reads a file this repo has and the published crate excludes.
    crate::testrepo::dogfood(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let pkl =
            std::fs::read_to_string(root.join("hk.pkl")).unwrap_or_default();
        assert!(!pkl.is_empty(), "this repository has a gate");
        let step = pkl
            .split_once("[\"lint-debt\"]")
            .map(|(_, r)| r.split_once("\n  }").map_or(r, |(s, _)| s))
            .unwrap_or_default();
        assert!(
            step.contains("sherd -- debt --check")
                && step.contains("sherd -- debt --record"),
            "the ratchet step calls the verb: {step}"
        );
        for restated in ["10000/l", "too many lines", "grep -cE"] {
            assert!(
                !step.contains(restated),
                "the gate restated `{restated}`, which is `B6` again"
            );
        }
    });
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

/// Coverage is the MIRROR: a floor that may only RISE, where the lint
/// ratios are ceilings that may only fall.
#[test]
fn a_floor_refuses_a_fall_and_states_what_it_saw() {
    let (ok, r) = coverage_verdict(9227, 9227);
    assert!(ok, "holding is not a fall");
    assert!(r.contains("92.27% (floor 92.27%)"), "{r}");
    let (rose, rr) = coverage_verdict(9300, 9227);
    assert!(rose, "rising is not a fall");
    assert!(rr.contains("93.00%"), "{rr}");
    let (fell, fr) = coverage_verdict(9226, 9227);
    assert!(!fell, "one hundredth down is a fall");
    assert!(fr.contains("coverage FELL, 92.27% -> 92.26%"), "{fr}");
}

/// HUNDREDTHS, where the lint ratios are tenths: `.coverage` has always
/// carried two decimals and rounding to one would move the floor by up
/// to five hundredths -- more than most of today's real changes.
#[test]
fn a_percentage_keeps_both_decimals() {
    assert_eq!(hundredths("92.27"), Some(9227));
    assert_eq!(hundredths("92.2"), Some(9220), "one decimal still parses");
    assert_eq!(hundredths("92"), Some(9200), "none does too");
    assert_eq!(hundredths("92.27%"), Some(9227), "a trailing sign is fine");
    assert_eq!(percent(9227), "92.27");
    assert_eq!(percent(9200), "92.00", "the zeroes are kept");
    assert_eq!(percent(9205), "92.05", "and so is the leading one");
}

/// `V6`'s mirror: the FIX half may only RAISE. Recording a drop is
/// filing down the ratchet's own teeth -- which is how a floor got
/// lowered by hand today (`B7`).
#[test]
fn recording_raises_and_refuses_to_lower() {
    let dir =
        std::env::temp_dir().join(format!("sherd-cov-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let read =
        || std::fs::read_to_string(dir.join(".coverage")).unwrap_or_default();
    let _ = std::fs::write(
        dir.join(".coverage"),
        "# why it is 90.00\nlines 90.00\n",
    );
    assert!(record_coverage(&dir, 9250).is_ok());
    assert!(read().contains("lines 92.50"), "{}", read());
    assert!(read().contains("# why it is 90.00"), "the reason survives");

    let before = read();
    assert!(
        record_coverage(&dir, 8000).is_err_and(|e| e.contains("drop")),
        "a fall is refused"
    );
    assert_eq!(read(), before, "and the file is untouched");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A tree with no `.coverage` is an ERROR, never a floor of zero.
#[test]
fn recording_into_a_tree_with_no_floor_is_an_error() {
    assert!(record_coverage(Path::new("/definitely-not-a-repo"), 1).is_err());
}

/// `src/tdd:V26`: a gate that did not EXECUTE is an ERROR, not a floor
/// breach. A missing toolchain returning "0%" is indistinguishable from
/// a suite that covers nothing.
#[test]
fn a_toolchain_that_did_not_run_yields_no_measurement() {
    let dir = std::env::temp_dir()
        .join(format!("sherd-cov-none-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    assert_eq!(coverage(&dir, "definitely-not-a-cargo"), None);
    assert_eq!(recorded_floor(&dir), None, "no file is not a floor of 0");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The denominator is the gate's, and it is not zero on this tree.
#[test]
fn the_denominator_counts_both_measured_directories() {
    // Reads a file this repo has and the published crate excludes.
    crate::testrepo::dogfood(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let all = measured_lines(root);
        let src_only = crate::fed::rust_files(&root.join("src"))
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .map(|t| t.lines().count())
            .sum::<usize>();
        assert!(all > src_only, "`dev` is measured too: {all} vs {src_only}");
    });
}

/// `V1`: the two ratios are computed from ONE measurement, so they cannot
/// disagree about the denominator -- which is `B6` in one sentence.
#[test]
fn both_ratios_come_from_one_denominator() {
    let m = Measured {
        count: 257,
        excess: 1648,
        loc: 17701,
    };
    assert_eq!(m.density(), 145, "14.5 per KLoC");
    assert_eq!(m.shape(), 93, "9.3% of lines");
    // Nothing measured is not zero debt: a ratio needs a denominator.
    let empty = Measured {
        count: 0,
        excess: 0,
        loc: 0,
    };
    assert_eq!(empty.density(), 0);
    assert_eq!(empty.shape(), 0);
}

/// The LIMIT is read from clippy's own message, so raising it in
/// `clippy.toml` cannot leave this measuring the old one.
#[test]
fn excess_is_lines_over_the_limit_clippy_reports() {
    assert_eq!(
        excess_lines("x: warning: too many lines (269/15)"),
        Some(254)
    );
    assert_eq!(excess_lines("x: warning: too many lines (20/18)"), Some(2));
    assert_eq!(excess_lines("x: warning: indexing may panic"), None);
}

/// `V1`+`V2`: BOTH breaches are reported, never just the first. A run
/// that stops at one hides the other and the next commit meets it alone.
#[test]
fn a_verdict_names_every_ratio_that_rose() {
    let worse = Measured {
        count: 300,
        excess: 2000,
        loc: 10_000,
    };
    let (ok, report) = verdict(worse, (100, 100));
    assert!(!ok);
    assert!(report.contains("DENSITY rose"), "{report}");
    assert!(report.contains("SHAPE rose"), "{report}");
    // `.:V48`: what was EXAMINED is stated either way.
    let (held, r2) = verdict(worse, (999, 999));
    assert!(held);
    assert!(r2.contains("300 warnings"), "{r2}");
    assert!(!r2.contains("rose"), "{r2}");
}

/// `V7`: every line that is not a number passes through untouched. The
/// file is the audit trail, and a recorded reason outlives its number.
#[test]
fn recording_rewrites_numbers_and_keeps_every_reason() {
    let now = Measured {
        count: 7,
        excess: 3,
        loc: 1_000,
    };
    assert_eq!(
        restate("# RAISED for T1, and here is why", now),
        "# RAISED for T1, and here is why"
    );
    assert_eq!(restate("density 99.9", now), "density 7.0");
    assert_eq!(restate("shape 5.0", now), "shape 0.3");
    assert_eq!(restate("count 1", now), "count 7");
    assert_eq!(restate("", now), "");
}

/// A breach message names BOTH numbers, because "it rose" without the
/// pair is a verdict nobody can argue with.
#[test]
fn a_breach_names_the_ceiling_and_the_measurement() {
    let d = breach_density(150, 140).unwrap_or_default();
    assert!(d.contains("14.0") && d.contains("15.0"), "{d}");
    assert_eq!(breach_density(140, 140), None, "holding is not a breach");
    assert_eq!(breach_shape(90, 92), None, "falling is not a breach");
}

/// Both ceilings are read, and a file missing either is unusable rather
/// than half-read -- half a ratchet gates half the tree.
#[test]
fn both_ceilings_are_read_or_neither_is() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let (d, s) = recorded_ceilings(root).unwrap_or_default();
    assert!(d > 0 && s > 0, "this repo records both: {d} {s}");
    assert_eq!(recorded_ceilings(Path::new("/definitely-not-a-repo")), None);
}

/// `V6`: the FIX half may only LOWER. `pre-commit` runs the fast set in
/// fix mode, so a fix that recorded whatever it measured would file down
/// the ratchet's own teeth -- writing the raise it exists to refuse.
#[test]
fn recording_lowers_and_refuses_to_raise() {
    let dir =
        std::env::temp_dir().join(format!("sherd-debt-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let write = |body: &str| {
        let _ = std::fs::write(dir.join(".lint-debt"), body);
    };
    let read =
        || std::fs::read_to_string(dir.join(".lint-debt")).unwrap_or_default();

    write(
        "# why it is 10.0\ndensity 10.0\nshape 5.0\ncount 1\nexcess 1\nloc 1\n",
    );
    // A FALL is written, and the comment survives it (`V7`).
    let better = Measured {
        count: 5,
        excess: 3,
        loc: 1_000,
    };
    assert!(record(&dir, better).is_ok());
    let after = read();
    assert!(after.contains("density 5.0"), "{after}");
    assert!(after.contains("shape 0.3"), "{after}");
    assert!(after.contains("count 5") && after.contains("loc 1000"));
    assert!(after.contains("# why it is 10.0"), "the reason survives");

    // A RISE is refused, and nothing is written.
    let worse = Measured {
        count: 900,
        excess: 900,
        loc: 1_000,
    };
    let before = read();
    assert!(
        record(&dir, worse).is_err_and(|e| e.contains("not a ratchet")),
        "a raise is refused"
    );
    assert_eq!(read(), before, "and the file is untouched");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A tree with no `.lint-debt` is an error, never a silent success.
#[test]
fn recording_debt_with_no_ratchet_is_an_error() {
    let m = Measured {
        count: 1,
        excess: 1,
        loc: 100,
    };
    assert!(record(Path::new("/definitely-not-a-repo"), m).is_err());
}

/// A scripted `cargo` in `dir`, printing `out` on stderr.
fn scripted_cargo(dir: &Path, out: &str) -> String {
    let p = dir.join("fake-cargo");
    let body = format!("#!/bin/sh\ncat <<'EOF' >&2\n{out}\nEOF\nexit 0\n");
    let _ = std::fs::write(&p, body);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            &p,
            std::fs::Permissions::from_mode(0o755),
        );
    }
    p.display().to_string()
}

/// `V3`: a count from a build that did NOT COMPILE is not a measurement.
/// Clippy emits no warnings for a target that fails to build, and the
/// ratchet would read that as debt paid -- 270 -> 180 (`B1`).
#[test]
fn a_failed_build_yields_no_measurement() {
    let dir = std::env::temp_dir()
        .join(format!("sherd-measure-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(dir.join("src"));
    let _ = std::fs::write(dir.join("src/a.rs"), "fn f() {}\n".repeat(100));

    let good = scripted_cargo(
        &dir,
        "src/a.rs:1:1: warning: too many lines (40/15)\n\
             dev/b.rs:2:2: warning: indexing may panic\n\
             tests/c.rs:3:3: warning: ignored",
    );
    let m = measure(&dir, &good).unwrap_or(Measured {
        count: 0,
        excess: 0,
        loc: 0,
    });
    assert_eq!(m.count, 2, "`tests/` is not measured");
    assert_eq!(m.excess, 25, "40 lines over a limit of 15");
    assert_eq!(m.loc, 100, "the denominator is the tree's own Rust");

    let _ = std::fs::remove_dir_all(&dir);
}

/// An ERROR anywhere means there is nothing to count, and a toolchain
/// that is not there is not a clean tree either (`V3`).
#[test]
fn a_broken_build_and_a_missing_toolchain_both_yield_nothing() {
    let dir = std::env::temp_dir()
        .join(format!("sherd-broken-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(dir.join("src"));
    let _ = std::fs::write(dir.join("src/a.rs"), "fn f() {}\n");
    let broken = scripted_cargo(
        &dir,
        "error[E0425]: cannot find value\n\
             src/a.rs:1:1: warning: indexing may panic",
    );
    assert_eq!(measure(&dir, &broken), None);
    assert_eq!(measure(&dir, "definitely-not-a-cargo"), None);
    let _ = std::fs::remove_dir_all(&dir);
}
