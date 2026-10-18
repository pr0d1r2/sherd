use super::*;
use crate::debt::swift::swift_tests::{toy_package, toy_swift};
use crate::testrepo::TestRepo;

/// The toy package of `src/debt:V10`'s tests, committed with a floor and a
/// cargo-only row, and its scripted `swift`.
fn floored_toy(tag: &str) -> Result<(TestRepo, String), String> {
    let r = TestRepo::new(tag)?;
    toy_package(&r)?;
    r.write(".coverage", "# why\nlines 0.00\nlines-default 1.00\n")?;
    r.commit("a toy package with a floor")?;
    let swift = toy_swift(r.path())?;
    Ok((r, swift))
}

fn coverage_text(r: &TestRepo) -> Result<String, String> {
    std::fs::read_to_string(r.path().join(".coverage"))
        .map_err(|e| e.to_string())
}

/// #138's "done when", first half: a toy SwiftPM package with one tested
/// and one untested function records the right percentage. The reason and
/// the cargo-only row survive (`src/debt:V10`).
#[test]
fn a_swift_package_records_its_line_coverage() -> Result<(), String> {
    let (r, swift) = floored_toy("cli-swift-record")?;
    let cov = |mode| coverage_in(r.path(), Some(mode), "not-a-cargo", &swift);
    assert_eq!(cov("--record"), ExitCode::SUCCESS);
    let text = coverage_text(&r)?;
    assert_eq!(text, "# why\nlines 100.00\nlines-default 1.00\n");
    assert_eq!(cov("--check"), ExitCode::SUCCESS);
    Ok(())
}

/// #138's "done when", second half: `--check` exits 1 after a commit
/// removes the test, and `--record` refuses to write that drop down.
#[test]
fn a_removed_swift_test_fails_the_check() -> Result<(), String> {
    let (r, swift) = floored_toy("cli-swift-drop")?;
    let cov = |mode| coverage_in(r.path(), Some(mode), "not-a-cargo", &swift);
    assert_eq!(cov("--record"), ExitCode::SUCCESS);
    std::fs::remove_file(r.path().join("Tests/ToyTests.swift"))
        .map_err(|e| e.to_string())?;
    r.commit("remove the test")?;
    assert_eq!(cov("--check"), ExitCode::from(1), "50.00 under 100.00");
    assert_eq!(cov("--record"), ExitCode::from(1), "a drop is not recorded");
    assert!(coverage_text(&r)?.contains("\nlines 100.00\n"));
    Ok(())
}

/// A SwiftPM package that cannot be measured, or has no `lines` floor, is
/// 2 -- an ERROR, never a breach and never a clean bill (`src/debt:V3`).
#[test]
fn an_unmeasurable_swift_package_is_two() -> Result<(), String> {
    let r = TestRepo::new("cli-swift-two")?;
    toy_package(&r)?;
    let swift = toy_swift(r.path())?;
    let cov = |sw: &str| coverage_in(r.path(), Some("--check"), "x", sw);
    assert_eq!(cov(&swift), ExitCode::from(2), "no `.coverage`");
    r.write(".coverage", "lines 10.00\n")?;
    assert_eq!(cov(&swift), ExitCode::SUCCESS);
    assert_eq!(cov("definitely-not-a-swift"), ExitCode::from(2));
    r.write("broken", "")?;
    assert_eq!(cov(&swift), ExitCode::from(2), "a failed test run");
    Ok(())
}

/// A `Cargo.toml` beside `Package.swift` keeps the tree on cargo: the
/// scripted cargo answers, and a `swift` that cannot run is never asked.
#[test]
fn a_cargo_tree_never_asks_swift() -> Result<(), String> {
    let r = TestRepo::new("cli-swift-cargo")?;
    toy_package(&r)?;
    r.write("Cargo.toml", "[package]\n")?;
    r.write(".coverage", "lines 90.00\n")?;
    let cargo = crate::testrepo::write_script(
        &r.path().join("fake-cargo"),
        "#!/bin/sh\necho 'TOTAL 1 2 3.00% 4 5 6.00% 7 8 92.50% 0 0 -'\nexit 0\n",
    )?;
    let cov = coverage_in(r.path(), Some("--check"), &cargo, "not-a-swift");
    assert_eq!(cov, ExitCode::SUCCESS);
    Ok(())
}
