use super::*;
use crate::testrepo::TestRepo;

/// A scripted `cargo` whose `llvm-cov` TOTAL depends on the build: `all`
/// with `--all-features`, `default` without. The two numbers differ, so a
/// build measured with the other's flags reads the wrong one.
pub(crate) fn two_builds(
    dir: &Path,
    all: &str,
    default: &str,
) -> Result<String, String> {
    let p = dir.join("two-build-cargo");
    let body = format!(
        "#!/bin/sh\ncase \" $* \" in\n\
         *' --all-features '*) pct={all} ;;\n\
         *) pct={default} ;;\n\
         esac\n\
         echo \"TOTAL 1 2 3.00% 4 5 6.00% 7 8 $pct% 0 0 -\"\nexit 0\n"
    );
    crate::testrepo::write_script(&p, &body)
}

/// `V4`: each build is measured with its OWN flags. The default build is
/// the one `cargo install` ships, and it was never measured (`B8`).
#[test]
fn each_build_is_measured_with_its_own_features() -> Result<(), String> {
    let r = TestRepo::new("cov-builds")?;
    let cargo = two_builds(r.path(), "90.89", "94.06")?;
    assert_eq!(coverage_of(r.path(), &cargo, Build::All), Some(9089));
    assert_eq!(coverage_of(r.path(), &cargo, Build::Default), Some(9406));
    assert_eq!(coverage(r.path(), &cargo), Some(9089), "`coverage` is All");
    Ok(())
}

/// The two rows are read apart: `lines` is not a prefix match for
/// `lines-default`, whichever comes first in the file.
#[test]
fn each_floor_is_read_from_its_own_row() -> Result<(), String> {
    let r = TestRepo::new("cov-rows")?;
    r.write(".coverage", "# why\nlines-default 94.06\nlines 90.89\n")?;
    assert_eq!(recorded_floor_of(r.path(), Build::All), Some(9089));
    assert_eq!(recorded_floor_of(r.path(), Build::Default), Some(9406));
    assert_eq!(recorded_floor(r.path()), Some(9089));

    r.write(".coverage", "lines 90.89\n")?;
    assert_eq!(
        recorded_floor_of(r.path(), Build::Default),
        None,
        "an absent row is absent, never the other build's number"
    );
    Ok(())
}

/// Recording one build rewrites its row and no other line, and a drop in
/// that build is refused with the row named.
#[test]
fn recording_one_build_leaves_the_other_row_alone() -> Result<(), String> {
    let r = TestRepo::new("cov-record")?;
    r.write(".coverage", "# why\nlines 90.89\nlines-default 94.06\n")?;
    record_coverage_of(r.path(), Build::Default, 9_420)?;
    let text = std::fs::read_to_string(r.path().join(".coverage"))
        .map_err(|e| e.to_string())?;
    assert_eq!(text, "# why\nlines 90.89\nlines-default 94.20\n");

    let err = record_coverage_of(r.path(), Build::Default, 9_000)
        .err()
        .ok_or("a drop was recorded")?;
    assert!(err.contains("`lines-default`"), "{err}");
    Ok(())
}
