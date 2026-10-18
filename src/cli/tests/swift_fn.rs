//! `sherd coverage` over a SwiftPM package with function exclusions and
//! the `excluded` cap (`src/debt:V11`).

use super::*;
use crate::testrepo::TestRepo;

fn coverage_text(r: &TestRepo) -> Result<String, String> {
    std::fs::read_to_string(r.path().join(".coverage"))
        .map_err(|e| e.to_string())
}

/// The real toy export of `src/debt:V11`, committed with `rows` under a
/// `lines 0.00` floor, and its scripted `swift`.
fn excluding_toy(tag: &str, rows: &str) -> Result<(TestRepo, String), String> {
    let r = TestRepo::new(tag)?;
    r.write("Package.swift", "// swift-tools-version:5.9\n")?;
    r.write(".coverage", &format!("# why\nlines 0.00\n{rows}"))?;
    let swift = crate::debt::swift_fn::swift_fn_tests::fn_swift(&r)?;
    Ok((r, swift))
}

const MAKE: &str =
    "exclude Sources/Toy/Crypto.swift:KitCrypto.make(_:) needs a real SE\n";

/// `src/debt:V11`: one function excluded records the raised figure and
/// LOWERS the cap to what was measured; `--check` then passes.
#[test]
fn a_function_exclusion_records_its_figure_and_lowers_the_cap()
-> Result<(), String> {
    let (r, swift) =
        excluding_toy("cli-swift-fn", &format!("excluded 20\n{MAKE}"))?;
    let cov = |mode| coverage_in(r.path(), Some(mode), "not-a-cargo", &swift);
    assert_eq!(cov("--record"), ExitCode::SUCCESS);
    let text = coverage_text(&r)?;
    assert_eq!(text, format!("# why\nlines 40.91\nexcluded 5\n{MAKE}"));
    assert_eq!(cov("--check"), ExitCode::SUCCESS);
    Ok(())
}

/// More excluded lines than the cap fail `--check`, and `--record`
/// writes NOTHING -- neither the floor those lines would raise nor a
/// higher cap. An absent cap is 0.
#[test]
fn over_the_cap_fails_and_records_nothing() -> Result<(), String> {
    for cap in ["excluded 4\n", ""] {
        let (r, swift) =
            excluding_toy("cli-swift-cap", &format!("{cap}{MAKE}"))?;
        let cov = |mode| coverage_in(r.path(), Some(mode), "x", &swift);
        let before = coverage_text(&r)?;
        assert_eq!(cov("--check"), ExitCode::from(1), "{cap:?}");
        assert_eq!(cov("--record"), ExitCode::from(1), "{cap:?}");
        assert_eq!(coverage_text(&r)?, before, "{cap:?}");
    }
    Ok(())
}

/// A row matching no function is an ERROR, exit 2, and a reasonless row
/// is no exclusion at all -- the cap of 0 holds.
#[test]
fn a_stale_row_is_two_and_a_reasonless_one_nothing() -> Result<(), String> {
    let gone = "exclude Sources/Toy/Crypto.swift:KitCrypto.gone() was SE\n";
    let (r, swift) = excluding_toy("cli-swift-stale", gone)?;
    let cov = coverage_in(r.path(), Some("--check"), "x", &swift);
    assert_eq!(cov, ExitCode::from(2));
    let bare = "exclude Sources/Toy/Crypto.swift:KitCrypto.make(_:)\n";
    let (r, swift) = excluding_toy("cli-swift-bare", bare)?;
    assert_eq!(
        coverage_in(r.path(), Some("--record"), "x", &swift),
        ExitCode::SUCCESS
    );
    assert!(coverage_text(&r)?.contains("\nlines 33.33\n"));
    Ok(())
}

/// A cargo tree reads neither `exclude` nor `excluded`: its `.coverage`
/// changes in the `lines` row only, byte for byte as before.
#[test]
fn a_cargo_tree_ignores_exclusions_and_the_cap() -> Result<(), String> {
    let r = TestRepo::new("cli-swift-cargo-cap")?;
    r.write("Cargo.toml", "[package]\n")?;
    let rows = format!("excluded 0\n{MAKE}exclude Sources/Gone.swift x\n");
    r.write(".coverage", &format!("lines 90.00\n{rows}"))?;
    let cargo = crate::testrepo::write_script(
        &r.path().join("fake-cargo"),
        "#!/bin/sh\necho 'TOTAL 1 2 3.00% 4 5 6.00% 7 8 92.50% 0 0 -'\nexit 0\n",
    )?;
    let cov = coverage_in(r.path(), Some("--record"), &cargo, "not-a-swift");
    assert_eq!(cov, ExitCode::SUCCESS);
    assert_eq!(coverage_text(&r)?, format!("lines 92.50\n{rows}"));
    Ok(())
}
