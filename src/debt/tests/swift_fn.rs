use super::*;
use crate::testrepo::TestRepo;

/// A REAL export, trimmed to `Sources/`: Apple Swift 6.4's `swift test
/// --enable-code-coverage` over a toy package, `@ROOT@` for its root.
/// `Sources/Toy/Crypto.swift` is 27 lines, 9 covered -- and 27 is the SUM
/// of its functions' own lines: `closure #1 in helper()`'s 3 lines are
/// counted again inside `helper()`'s 6. `guarded()` is an `#unavailable`
/// guard, 6 lines with 5 run. Only `covered(_:)`, `init()` and `guarded()`
/// are called by the toy's test.
pub(crate) const TOY_EXPORT: &str = include_str!("toy-export.json");

/// What Apple Swift 6.4's `swift demangle --simplified` printed for each
/// mangled name above, tab-separated.
pub(crate) const TOY_DEMANGLED: &str = include_str!("toy-demangled.tsv");

/// A scripted `swift` over [`TOY_EXPORT`]: the two `test` runs write it
/// with `$PWD` for its root, `demangle --simplified` answers from
/// [`TOY_DEMANGLED`]. A `broken` file fails the test run.
pub(crate) fn fn_swift(r: &TestRepo) -> Result<String, String> {
    r.write("export.json", TOY_EXPORT)?;
    r.write("demangled.tsv", TOY_DEMANGLED)?;
    crate::testrepo::write_script(
        &r.path().join("fn-swift"),
        "#!/bin/sh\ncase \"$*\" in\n\
         'test --enable-code-coverage') [ -f broken ] && exit 1\n\
         \x20 sed \"s|@ROOT@|$PWD|g\" export.json > codecov.json ;;\n\
         'test --show-codecov-path') echo \"$PWD/codecov.json\" ;;\n\
         'demangle --simplified') awk -F'\\t' \
         'NR==FNR{m[$1]=$2;next}{print(($0 in m)?m[$0]:$0)}' \
         demangled.tsv - ;;\nesac\nexit 0\n",
    )
}

fn toy(root: &Path) -> String {
    TOY_EXPORT.replace("@ROOT@", &root.display().to_string())
}

fn table(names: &[String]) -> Option<Vec<String>> {
    let map: Vec<(&str, &str)> = TOY_DEMANGLED
        .lines()
        .filter_map(|l| l.split_once('\t'))
        .collect();
    Some(
        names
            .iter()
            .map(|n| {
                map.iter()
                    .find(|(m, _)| m == n)
                    .map_or_else(|| n.clone(), |(_, d)| (*d).to_string())
            })
            .collect(),
    )
}

fn row(function: &str) -> Exclusion {
    Exclusion {
        path: "Sources/Toy/Crypto.swift".into(),
        function: Some(function.into()),
    }
}

fn measured(rows: &[Exclusion]) -> Result<SwiftMeasure, String> {
    let root = Path::new("/r");
    measure_export(&toy(root), root, rows, &table)
}

/// `V11`: with no row the figure is `V10`'s, 9/27; one uncovered function
/// leaves numerator and denominator by ITS lines -- `make(_:)`'s 5 -- and
/// the figure rises to 9/22.
#[test]
fn excluding_an_uncovered_function_raises_the_figure() {
    let none = SwiftMeasure {
        lines: 3333,
        excluded: 0,
    };
    assert_eq!(measured(&[]), Ok(none));
    let one = SwiftMeasure {
        lines: 4091,
        excluded: 5,
    };
    assert_eq!(measured(&[row("KitCrypto.make(_:)")]), Ok(one));
}

/// Excluding COVERED code can only lower the figure (6/24) and still
/// counts against the cap -- it hides nothing in the floor's favour.
#[test]
fn excluding_a_covered_function_lowers_nothing_but_the_figure() {
    let m = SwiftMeasure {
        lines: 2500,
        excluded: 3,
    };
    assert_eq!(measured(&[row("KitCrypto.covered(_:)")]), Ok(m));
}

/// A row names a declaration: `helper()` takes `closure #1 in helper()`
/// with it, each record's own lines once (6 + 3, as llvm-cov counted
/// them); `static` is not part of the name; two rows on one function drop
/// it once.
#[test]
fn a_row_names_a_declaration_and_drops_it_once() {
    let helper = SwiftMeasure {
        lines: 5000,
        excluded: 9,
    };
    assert_eq!(measured(&[row("helper()")]), Ok(helper));
    let twice = [row("Keys.load(tag:)"), row("Keys.load(tag:)")];
    assert_eq!(
        measured(&twice),
        Ok(SwiftMeasure {
            lines: 3462,
            excluded: 1
        })
    );
}

/// A row that matches nothing is an ERROR, never a silent no-op: a stale
/// exclusion would sit in the file explaining lines that are gone.
#[test]
fn a_row_matching_nothing_is_an_error() {
    let fails = |rows: &[Exclusion]| measured(rows).err().unwrap_or_default();
    assert!(fails(&[row("make(_:)")]).contains("matches no function"));
    assert!(fails(&[row("KitCrypto.gone()")]).contains("KitCrypto.gone()"));
    let file = Exclusion {
        path: "Sources/Gone.swift".into(),
        function: None,
    };
    assert!(fails(&[file]).contains("no `Sources/` file"));
    let whole = Exclusion {
        path: row("x").path,
        function: None,
    };
    assert!(fails(&[whole, row("KitCrypto.gone()")]).contains("gone"));
}

/// `V3`: a demangler that did not run, and a file whose functions do not
/// sum to llvm-cov's own count, are refusals -- never a guessed figure.
#[test]
fn a_count_sherd_cannot_reproduce_is_refused() {
    let root = Path::new("/r");
    let rows = [row("KitCrypto.make(_:)")];
    let silent = |_: &[String]| None;
    assert!(measure_export(&toy(root), root, &rows, &silent).is_err());
    let torn = toy(root).replace("\"count\":27", "\"count\":28");
    let e = measure_export(&torn, root, &rows, &table).err();
    assert!(e.unwrap_or_default().contains("do not sum"));
}

/// The row format: `<path>:<function> <reason>` splits at the FIRST `:`;
/// with no reason a row is ignored, the cap row is `excluded <n>`.
#[test]
fn rows_parse_and_a_reasonless_one_is_ignored() -> Result<(), String> {
    let r = TestRepo::new("swift-fn-rows")?;
    assert_eq!(excluded_cap(r.path()), None);
    r.write(
        ".coverage",
        "lines 1.00\nexcluded 7\n\
         exclude Sources/Toy/Crypto.swift:KitCrypto.make(_:) needs an SE\n\
         exclude Sources/Toy/Crypto.swift:helper()\n\
         exclude ./Sources/T.swift a terminal\n",
    )?;
    let whole = Exclusion {
        path: "Sources/T.swift".into(),
        function: None,
    };
    assert_eq!(exclusions(r.path()), [row("KitCrypto.make(_:)"), whole]);
    assert_eq!(excluded_cap(r.path()), Some(7));
    assert_eq!(super::super::excluded_files(r.path()), ["Sources/T.swift"]);
    Ok(())
}

/// The cap may only FALL: `record_excluded` lowers it, refuses a raise,
/// and with no row and nothing excluded writes nothing.
#[test]
fn the_cap_records_down_and_refuses_up() -> Result<(), String> {
    let r = TestRepo::new("swift-fn-cap")?;
    r.write(".coverage", "# why\nlines 1.00\n")?;
    assert!(record_excluded(r.path(), 0).is_ok());
    assert!(record_excluded(r.path(), 1).is_err(), "absent caps at 0");
    r.write(".coverage", "# why\nexcluded 9\nlines 1.00\n")?;
    assert!(record_excluded(r.path(), 10).is_err());
    assert!(record_excluded(r.path(), 4).is_ok());
    let text = std::fs::read_to_string(r.path().join(".coverage"))
        .map_err(|e| e.to_string())?;
    assert_eq!(text, "# why\nexcluded 4\nlines 1.00\n");
    assert!(excluded_verdict(4, 4).0);
    assert!(!excluded_verdict(5, 4).0);
    Ok(())
}

/// End to end over the scripted toolchain, the demangler included.
#[test]
fn swift_measure_runs_the_demangler() -> Result<(), String> {
    let r = TestRepo::new("swift-fn-measure")?;
    let swift = fn_swift(&r)?;
    r.write(
        ".coverage",
        "lines 1.00\nexclude Sources/Toy/Crypto.swift:KitCrypto.make(_:) SE\n",
    )?;
    let m = swift_measure(r.path(), &swift)?;
    assert_eq!(
        m,
        SwiftMeasure {
            lines: 4091,
            excluded: 5
        }
    );
    r.write("broken", "")?;
    assert!(swift_measure(r.path(), &swift).is_err());
    Ok(())
}
