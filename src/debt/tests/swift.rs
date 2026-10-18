use super::*;
use crate::testrepo::TestRepo;

/// A scripted `swift` over a toy package: `Sources/Toy.swift` has two
/// lines, and `Tests/ToyTests.swift` covers the second -- so 100.00% with
/// the test and 50.00% once a commit removes it. Like SwiftPM's, the JSON
/// also lists the test file and a `.build/` derived runner, and its
/// `totals` count them; neither may move the figure (`V10`). A `broken`
/// file makes the test run FAIL before it writes, leaving the last run's
/// JSON on disk.
pub(crate) fn toy_swift(dir: &Path) -> Result<String, String> {
    crate::testrepo::write_script(&dir.join("toy-swift"), TOY_SWIFT)
}

/// The script [`toy_swift`] writes.
const TOY_SWIFT: &str = "#!/bin/sh\nf() { printf '{\"filename\":\"%s\",\
    \"segments\":[],\"summary\":{\"functions\":{\"count\":1,\
    \"covered\":1},\"lines\":{\"count\":%s,\"covered\":%s}}}' \
    \"$PWD/$1\" \"$2\" \"$3\"; }\n\
    case \"$*\" in\n\
    'test --enable-code-coverage')\n\
    \x20 [ -f broken ] && exit 1\n\
    \x20 [ -f Tests/ToyTests.swift ] && c=2 || c=1\n\
    \x20 { printf '{\"data\":[{\"files\":[' ;\
    \x20 f .build/x/DerivedSources/test_entry_point.swift 28 16 ;\
    \x20 printf , ; f Sources/Toy.swift 2 \"$c\" ; printf , ;\
    \x20 f Tests/ToyTests.swift 9 9 ;\
    \x20 printf '],\"totals\":{\"lines\":{\"count\":39,\
    \"covered\":27}}}],\"type\":\"llvm.coverage.json.export\"}' ;\
    \x20 } > codecov.json ;;\n\
    'test --show-codecov-path') echo \"$PWD/codecov.json\" ;;\n\
    esac\nexit 0\n";

/// A toy SwiftPM package: one tested and one untested function.
pub(crate) fn toy_package(r: &TestRepo) -> Result<(), String> {
    r.write("Package.swift", "// swift-tools-version:5.9\n")?;
    r.write("Sources/Toy.swift", "func a() {}\nfunc b() {}\n")?;
    r.write("Tests/ToyTests.swift", "// calls b()\n")
}

/// One `data[0].files[]` entry as llvm-cov writes it: the `filename`,
/// then a `summary` whose `functions` row comes before `lines`.
fn file_entry(name: &str, count: usize, covered: usize) -> String {
    format!(
        "{{\"branches\":[],\"expansions\":[{{\"filenames\":[\"x\"]}}],\
         \"filename\":\"{name}\",\"segments\":[[1,2,3,true,true,false]],\
         \"summary\":{{\"functions\":{{\"count\":4,\"covered\":4}},\
         \"lines\":{{\"count\":{count},\"covered\":{covered},\
         \"percent\":0}}}}}}"
    )
}

/// An export over `files`, with a `totals` that counts everything -- the
/// figure `V10` must NOT read.
fn export(files: &[String]) -> String {
    format!(
        "{{\"data\":[{{\"files\":[{}],\"functions\":[{{\"filenames\":\
         [\"/r/Sources/A.swift\"],\"name\":\"a\"}}],\"totals\":{{\
         \"lines\":{{\"count\":3804,\"covered\":3648}}}}}}]}}",
        files.join(",")
    )
}

/// age-plugin-se a7bae66's `files[]`: path under the root, lines, covered.
const AGE_PLUGIN_SE: [(&str, usize, usize); 10] = [
    (
        ".build/Tests-p.build/DerivedSources/test_entry_point.swift",
        28,
        16,
    ),
    ("Sources/Base64.swift", 32, 32),
    ("Sources/Bech32.swift", 175, 159),
    ("Sources/CLI.swift", 149, 66),
    ("Sources/Crypto.swift", 32, 4),
    ("Sources/HPKE.swift", 141, 139),
    ("Sources/Plugin.swift", 598, 595),
    ("Sources/Stream.swift", 8, 0),
    ("Tests/CLITests.swift", 199, 199),
    ("Tests/PluginTests.swift", 1632, 1632),
];

/// `V10`, measured: age-plugin-se a7bae66's own files. `totals` read
/// 3648/3804 = 95.90%; its seven `Sources/` files are 995/1135 = 87.665...,
/// which llvm-cov prints as 87.67. `Tests/` and the `.build/` derived
/// runner are in the export and out of the figure.
#[test]
fn only_files_under_sources_count() {
    let root = Path::new("/r");
    let files: Vec<String> = AGE_PLUGIN_SE
        .iter()
        .map(|(f, n, c)| file_entry(&format!("/r/{f}"), *n, *c))
        .collect();
    assert_eq!(llvm_cov_lines(&export(&files), root), Some(8767));
}

/// `V10`'s boundary: a path RELATIVE to the root, a nested source and a
/// `Sources/` OUTSIDE the root (a `.build/checkouts` dependency, or another
/// tree) -- only the first two count. `Sources` as a prefix of a longer
/// name is not the directory.
#[test]
fn a_source_is_under_the_roots_own_sources_dir() {
    let root = Path::new("/r");
    let files = [
        file_entry("Sources/A.swift", 10, 5),
        file_entry("/r/Sources/Sub/B.swift", 10, 10),
        file_entry("/elsewhere/Sources/X.swift", 50, 0),
        file_entry("/r/.build/checkouts/dep/Sources/Y.swift", 50, 0),
        file_entry("/r/SourcesExtra/Z.swift", 50, 0),
        file_entry("/r/Tests/T.swift", 100, 100),
    ];
    assert_eq!(llvm_cov_lines(&export(&files), root), Some(7500));
}

/// `V3`: no lines under `Sources/` is no measurement, and neither is an
/// entry whose `summary` cannot be read -- never a figure from the others.
#[test]
fn no_readable_source_lines_is_unmeasurable() {
    let root = Path::new("/r");
    let tests_only = [file_entry("/r/Tests/T.swift", 3, 3)];
    assert_eq!(llvm_cov_lines(&export(&tests_only), root), None);
    let empty = [file_entry("/r/Sources/E.swift", 0, 0)];
    assert_eq!(llvm_cov_lines(&export(&empty), root), None);
    let torn = format!(
        "{},{{\"filename\":\"/r/Sources/T.swift\"}}",
        file_entry("/r/Sources/A.swift", 2, 1)
    );
    assert_eq!(llvm_cov_lines(&export(&[torn]), root), None, "no summary");
    assert_eq!(llvm_cov_lines("{\"data\":[]}", root), None);
}

/// Rounding is half up in hundredths, as llvm-cov's `%.2f` prints: 1/3
/// is 33.33, 2/3 is 66.67.
#[test]
fn source_lines_round_as_llvm_cov_prints() {
    let root = Path::new("/r");
    let one = [file_entry("/r/Sources/A.swift", 3, 1)];
    assert_eq!(llvm_cov_lines(&export(&one), root), Some(3333));
    let two = [file_entry("/r/Sources/A.swift", 3, 2)];
    assert_eq!(llvm_cov_lines(&export(&two), root), Some(6667));
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
