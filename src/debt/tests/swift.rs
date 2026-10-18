use super::*;
use crate::testrepo::TestRepo;

/// A scripted `swift` over a toy package: `Sources/Toy.swift` has two
/// lines, and `Tests/ToyTests.swift` covers the second -- so 100.00% with
/// the test and 50.00% once a commit removes it. A `broken` file makes the
/// test run FAIL before it writes, leaving the last run's JSON on disk.
pub(crate) fn toy_swift(dir: &Path) -> Result<String, String> {
    let body = "#!/bin/sh\ncase \"$*\" in\n\
         'test --enable-code-coverage')\n\
         \x20 [ -f broken ] && exit 1\n\
         \x20 [ -f Tests/ToyTests.swift ] && c=2 || c=1\n\
         \x20 printf '{\"data\":[{\"files\":[{\"filename\":\"Sources/Toy.swift\",\
         \"summary\":{\"lines\":{\"count\":9,\"covered\":0,\"percent\":0}}}],\
         \"totals\":{\"functions\":{\"count\":4,\"covered\":4,\"percent\":100},\
         \"lines\":{\"count\":2,\"covered\":%s,\"percent\":0}}}],\
         \"type\":\"llvm.coverage.json.export\"}' \"$c\" > codecov.json ;;\n\
         'test --show-codecov-path') echo \"$PWD/codecov.json\" ;;\n\
         esac\nexit 0\n";
    crate::testrepo::write_script(&dir.join("toy-swift"), body)
}

/// A toy SwiftPM package: one tested and one untested function.
pub(crate) fn toy_package(r: &TestRepo) -> Result<(), String> {
    r.write("Package.swift", "// swift-tools-version:5.9\n")?;
    r.write("Sources/Toy.swift", "func a() {}\nfunc b() {}\n")?;
    r.write("Tests/ToyTests.swift", "// calls b()\n")
}

/// `V10`: the line total is `data[0].totals.lines` -- not a file's own
/// `summary.lines`, which comes first, and not the `functions` total.
/// 3648/3804 is age-plugin-se's measured total, 95.899..., which llvm-cov
/// prints as 95.90.
#[test]
fn the_line_total_is_read_from_totals_lines() {
    let json = "{\"data\":[{\"files\":[{\"summary\":{\"lines\":\
                {\"count\":9,\"covered\":1}}}],\"totals\":{\"functions\":\
                {\"count\":4,\"covered\":4},\"lines\":{\"count\":3804,\
                \"covered\":3648,\"percent\":95.899053627760253}}}]}";
    assert_eq!(llvm_cov_lines(json), Some(9590));
    let spaced = "{ \"totals\" : { \"lines\" : { \"covered\" : 1 , \
                  \"count\" : 3 } } }";
    assert_eq!(llvm_cov_lines(spaced), Some(3333), "key order, whitespace");
    let empty = "{\"totals\":{\"lines\":{\"count\":0,\"covered\":0}}}";
    assert_eq!(llvm_cov_lines(empty), None, "no lines is no measurement");
    assert_eq!(llvm_cov_lines("{\"data\":[]}"), None);
}

/// `V10`: `Package.swift` alone is SwiftPM; a `Cargo.toml` beside it wins,
/// and a tree with neither stays on cargo as it always was.
#[test]
fn a_cargo_manifest_wins_over_a_swift_package() -> Result<(), String> {
    let r = TestRepo::new("swift-detect")?;
    assert_eq!(toolchain_of(r.path()), Toolchain::Cargo, "neither");
    r.write("Package.swift", "// swift-tools-version:5.9\n")?;
    assert_eq!(toolchain_of(r.path()), Toolchain::SwiftPm);
    r.write("Cargo.toml", "[package]\n")?;
    assert_eq!(toolchain_of(r.path()), Toolchain::Cargo, "both");
    Ok(())
}

/// `V10` end to end over the scripted toolchain, and `V3`: once the test
/// run fails, the JSON the LAST run left behind is not a measurement.
#[test]
fn a_failed_test_run_is_unmeasurable_despite_an_old_json() -> Result<(), String>
{
    let r = TestRepo::new("swift-cov")?;
    toy_package(&r)?;
    let swift = toy_swift(r.path())?;
    assert_eq!(swift_coverage(r.path(), &swift), Some(10_000));
    let _ = std::fs::remove_file(r.path().join("Tests/ToyTests.swift"));
    assert_eq!(swift_coverage(r.path(), &swift), Some(5_000));
    r.write("broken", "")?;
    assert!(
        r.path().join("codecov.json").exists(),
        "the old JSON is there"
    );
    assert_eq!(swift_coverage(r.path(), &swift), None);
    assert_eq!(swift_coverage(r.path(), "definitely-not-a-swift"), None);
    Ok(())
}
