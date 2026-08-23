#![allow(
    clippy::expect_used,
    reason = "fixture setup: a test that cannot write its own scratch tree has \
              not failed an assertion, it has failed to run, and `expect` says \
              which file could not be created. The lint is warned on in \
              `dev/Cargo.toml` for the BINARY, where an unattended panic is a \
              run that stops with no record."
)]

//! `sherd-dev` end to end, against a FIXTURE repository.
//!
//! Every spawn sets `current_dir` to a scratch tree that this file built, and
//! one test asserts that tree is not this one. `src/cli:V6` is the rule and
//! `src/cli:B1` is the recording: two tests in the main crate walk up for a
//! git repo, find whichever tree the runner sits in, and therefore pass in a
//! checkout while failing anywhere else. A binary whose whole job is to
//! rewrite a README is the last one that should be pointed at the real
//! README by accident.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BEGIN: &str = "<!-- BEGIN badges -->";
const END: &str = "<!-- END badges -->";

/// Every block `sherd-dev readme` generates. A fixture missing one is reported
/// as NOT OPTED IN rather than stale, so the tests that assert staleness
/// have to carry all four.
const GRAPH_MARKERS: &str = "\n<!-- BEGIN graph-tree -->\n<!-- END graph-tree -->\n<!-- BEGIN graph-mermaid -->\n<!-- END graph-mermaid -->\n<!-- BEGIN graph-table -->\n<!-- END graph-table -->\n<!-- BEGIN commands -->\n<!-- END commands -->\n";

/// A repository-shaped directory: the files `sherd-dev badges` reads, and
/// nothing else.
fn fixture(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("sherd-dev-{name}"));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join(".git")).expect("fixture dir");
    fs::create_dir_all(dir.join(".github/workflows")).expect("workflow dir");
    let write = |rel: &str, body: &str| {
        fs::write(dir.join(rel), body).expect("fixture file");
    };
    // §I has to name what the dispatch fixture dispatches, or the V115
    // runner reports the fixture rather than the code under test.
    write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nfixture.\n\n## \u{a7}I INTERFACES\n\n- cmd: `sherd budget [dir]` \u{2192} the table\n",
    );
    write(
        "Cargo.toml",
        "[package]\nname = \"f\"\nedition = \"2024\"\nrust-version = \"1.95\"\n\n[dependencies]\nx = \"1\"\n",
    );
    write(".coverage", "lines 90.58\n");
    write(".lint-debt", "density 17.0\nshape 10.5\ncount 271\n");
    write(
        "hk.pkl",
        "local fast = new Mapping<String, Step> {\n  [\"fmt\"] {\n  }\n}\n",
    );
    // A node HEADER, a timestamp and a branch ref -- the three fields the
    // badge reads. Shaped like the real lock rather than minimally, because
    // `dev:B1` was a parser reading the wrong node of a real one.
    write(
        "flake.lock",
        "{\n  \"nodes\": {\n    \"nixpkgs\": {\n      \"locked\": {\n        \"lastModified\": 1786535285,\n        \"rev\": \"9f78f44a87948854445dae0b6bf82b2e87e4efb5\"\n      },\n      \"original\": {\n        \"ref\": \"nixos-26.05\"\n      }\n    }\n  }\n}\n",
    );
    write(".github/workflows/ci.yml", "        os: [macos-latest]\n");
    // The V7 runner reads the dispatch source, and the Commands table is
    // rendered from the usage const beside it, so a fixture repository needs
    // both to exist.
    fs::create_dir_all(dir.join("src").join("cli")).expect("cli dir");
    write(
        "src/cli/mod.rs",
        "match args.first().map(String::as_str) {
        Some(\"budget\") => budget(),
    }
",
    );
    dir
}

fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sherd-dev"))
        .args(args)
        .current_dir(dir)
        .output()
        .expect("sherd-dev runs")
}

/// The guard for every other test here: if the spawn directory were inside
/// this repository, each assertion below would be about OUR README.
#[test]
fn a_fixture_is_not_this_repository() {
    let dir = fixture("isolation");
    let ours = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(!dir.starts_with(ours.parent().unwrap_or(ours)));
    assert!(!dir.join("dev").exists());
}

#[test]
fn an_unknown_verb_is_a_usage_error() {
    let out = run(&fixture("usage"), &["nonsense"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("sherd-dev readme"));
}

#[test]
fn no_verb_at_all_is_a_usage_error() {
    let out = run(&fixture("noverb"), &[]);
    assert_eq!(out.status.code(), Some(2));
}

/// A README with no markers has not opted in. That is exit 1 naming the
/// markers, never a rewrite of a file that did not ask for one.
#[test]
fn a_readme_without_markers_is_named_and_left_alone() {
    let dir = fixture("nomarkers");
    fs::write(dir.join("README.md"), "# f\n\nno markers here\n")
        .expect("readme");
    let out = run(&dir, &["readme"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("markers"));
    let after = fs::read_to_string(dir.join("README.md")).expect("readme");
    assert_eq!(after, "# f\n\nno markers here\n");
}

#[test]
fn a_missing_owning_file_is_an_error_naming_it() {
    let dir = fixture("missing");
    fs::write(
        dir.join("README.md"),
        format!("# f\n{BEGIN}\n{END}\n{GRAPH_MARKERS}"),
    )
    .expect("readme");
    fs::remove_file(dir.join(".lint-debt")).expect("remove");
    let out = run(&dir, &["readme"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains(".lint-debt"));
}

/// Write, then check: the second run must be silent and clean, or `--check`
/// is not a diff of what the first run produced.
#[test]
fn writing_then_checking_is_clean_and_idempotent() {
    let dir = fixture("roundtrip");
    fs::write(
        dir.join("README.md"),
        format!("# f\n\n{BEGIN}\n{END}\n{GRAPH_MARKERS}\ntail\n"),
    )
    .expect("readme");

    let out = run(&dir, &["readme"]);
    assert_eq!(out.status.code(), Some(0));
    let once = fs::read_to_string(dir.join("README.md")).expect("readme");
    assert!(once.contains("edition-2024"));
    assert!(once.contains("MSRV-1.95"));
    assert!(once.ends_with("\ntail\n"));

    let out = run(&dir, &["readme", "--check"]);
    assert_eq!(out.status.code(), Some(0));

    let out = run(&dir, &["readme"]);
    assert_eq!(out.status.code(), Some(0));
    let twice = fs::read_to_string(dir.join("README.md")).expect("readme");
    assert_eq!(once, twice);
}

/// `--check` must REFUSE and write nothing. A checker that fixes what it
/// found is a green tick over a diff nobody has seen -- which is why CI runs
/// `hk check --check` rather than the fix half.
#[test]
fn check_refuses_a_stale_block_without_touching_it() {
    let dir = fixture("stale");
    let stale =
        format!("# f\n\n{BEGIN}\nstale content\n{END}\n{GRAPH_MARKERS}");
    fs::write(dir.join("README.md"), &stale).expect("readme");

    let out = run(&dir, &["readme", "--check"]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("STALE"));
    assert!(err.contains("want:"));
    assert!(err.contains("have: stale content"));
    assert_eq!(
        fs::read_to_string(dir.join("README.md")).expect("readme"),
        stale
    );
}

/// The hooks' entry point. One command runs every check, so a hook never
/// carries its own list of what the gate does.
#[test]
fn the_top_level_check_runs_every_check() {
    let dir = fixture("checkall");
    fs::write(
        dir.join("README.md"),
        format!("# f\n\n{BEGIN}\nstale\n{END}\n{GRAPH_MARKERS}"),
    )
    .expect("readme");
    let out = run(&dir, &["--check"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("readme"));
}

/// The narrow layer. A commit that touched nothing the README is rendered
/// from does not re-render it -- and this must not become a way for a stale
/// block to pass, which is why the wide run on push takes no paths at all.
#[test]
fn a_change_touching_no_input_is_clean_even_when_a_block_is_stale() {
    let dir = fixture("scoped");
    let stale = format!("# f\n\n{BEGIN}\nstale\n{END}\n{GRAPH_MARKERS}");
    fs::write(dir.join("README.md"), &stale).expect("readme");

    let scoped = run(&dir, &["--check", "src/lens/mod.rs"]);
    assert_eq!(scoped.status.code(), Some(0));

    let wide = run(&dir, &["--check"]);
    assert_eq!(wide.status.code(), Some(1));
    assert_eq!(
        fs::read_to_string(dir.join("README.md")).expect("readme"),
        stale
    );
}

/// `.:V115` end to end. The unit tests prove the comparison; this proves the
/// binary runs it, refuses, and says which direction the drift goes -- the
/// half `.:B17` needed, since §I read fine to every human who opened it.
#[test]
fn a_verb_missing_from_the_interface_section_refuses() {
    let dir = fixture("interface");
    fs::write(
        dir.join("README.md"),
        format!("# f\n\n{BEGIN}\n{END}\n{GRAPH_MARKERS}"),
    )
    .expect("readme");
    // The fixture dispatches `budget`; §I now names something else entirely.
    fs::write(
        dir.join("SPEC.md"),
        "# SPEC\n\n## \u{a7}G GOAL\n\nfixture.\n\n## \u{a7}I INTERFACES\n\n- cmd: `sherd lens <dir>` \u{2192} a pack\n",
    )
    .expect("spec");

    let out = run(&dir, &["readme", "--check"]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("V115"), "the rule is named: {err}");
    assert!(err.contains("`budget` dispatches and \u{a7}I does not name it"));
    assert!(err.contains("`lens` is in \u{a7}I, does not dispatch"));
}
