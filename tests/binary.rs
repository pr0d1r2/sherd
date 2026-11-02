//! The installed binary, run as a process: `src/main.rs` and `cli::run`,
//! which every other test bypasses by calling `run_args` with an argv it
//! built. What `run` adds is reading the REAL argv and handing the exit code
//! to the OS, and only a spawned process can see either.
//!
//! Every spawn runs in a scratch directory that is not this repository
//! (`src/cli:V6`): a test that walks up into whichever tree the runner sits in
//! passes in a checkout and fails anywhere else (`src/cli:B1`).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A scratch directory unique to this test, outside any repository.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("sherd-bin-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap_or_else(|e| {
        panic!("scratch dir {}: {e}", d.display());
    });
    d
}

fn sherd(tag: &str, args: &[&str]) -> Output {
    let dir = scratch(tag);
    let out = sherd_in(&dir, args);
    let _ = std::fs::remove_dir_all(&dir);
    out
}

fn sherd_in(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_sherd"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn sherd: {e}"))
}

/// A repository as `repo_root_from` reads one, with a node at `src/code`.
fn repo_with_a_node(root: &Path) {
    for d in [".git", "src/code"] {
        std::fs::create_dir_all(root.join(d)).unwrap_or_else(|e| {
            panic!("fixture dir {d}: {e}");
        });
    }
    for f in ["SPEC.md", "src/code/SPEC.md"] {
        std::fs::write(root.join(f), "# SPEC\n").unwrap_or_else(|e| {
            panic!("fixture file {f}: {e}");
        });
    }
}

/// `src/cli:V16`: the version on STDOUT, exit 0, naming the version the
/// manifest declares -- from the process, where a wrapper reads it.
#[test]
fn the_binary_names_its_version_on_stdout() {
    let out = sherd("version", &["--version"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        stdout.trim(),
        format!("sherd {}", env!("CARGO_PKG_VERSION")),
        "{out:?}"
    );
}

/// `src/cli:V1`: the exit code reaches the OS. An unknown verb is usage, 2,
/// and names itself on stderr.
#[test]
fn an_unknown_verb_exits_2_from_the_process() {
    let out = sherd("unknown", &["no-such-verb"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no-such-verb"),
        "{out:?}"
    );
}

/// `src/cli:V17`, where a caller meets it: `[dir]` is ROOT-relative, so `lens
/// code` typed in `src/` is a miss that teaches the spelling that works, not a
/// hit that depends on where the caller stands.
#[test]
fn a_dir_typed_below_the_root_is_a_miss_that_teaches() {
    let root = scratch("v17");
    repo_with_a_node(&root);
    let miss = sherd_in(&root.join("src"), &["lens", "code"]);
    let hit = sherd_in(&root.join("src"), &["lens", "src/code"]);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(miss.status.code(), Some(2), "{miss:?}");
    assert!(
        String::from_utf8_lossy(&miss.stderr)
            .contains("did you mean `src/code`?"),
        "{miss:?}"
    );
    assert_eq!(hit.status.code(), Some(0), "{hit:?}");
}

/// `src/cli:B17`: `..` that climbs back to this repository's own root is still
/// this repository, so `[dir]` stays ROOT-relative. The walk from it returns
/// the root spelled `src/..`, and that spelling is not another repository.
#[test]
fn a_dir_climbing_to_its_own_root_stays_root_relative() {
    let root = scratch("b17");
    repo_with_a_node(&root);
    let miss = sherd_in(&root.join("src"), &["lens", "../src/code"]);
    let _ = std::fs::remove_dir_all(&root);
    assert_eq!(miss.status.code(), Some(2), "{miss:?}");
}

/// `src/cli:V21`: `--help`, its spellings and a bare invocation print `USAGE`
/// verbatim on stdout. It is the README's Commands source, so what the derive
/// would generate instead is not what is published. A verb's own `--help` IS
/// generated, and that is the one place its flags are listed.
#[test]
fn help_is_usage_verbatim_and_a_verb_documents_its_own_flags() {
    for (tag, argv) in [
        ("long", &["--help"][..]),
        ("short", &["-h"]),
        ("verb", &["help"]),
        ("bare", &[]),
        ("verbose", &["-v"]),
    ] {
        let out = sherd(&format!("help-{tag}"), argv);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout).trim_end(),
            sherd::cli::USAGE,
            "{argv:?}"
        );
    }
    let out = sherd("help-lens", &["lens", "--help"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("--depth"));
}

/// `src/cli:V21`: completions and the man page reach STDOUT, where a package
/// script redirects them from, and both come from the one grammar, so each
/// names every verb `USAGE` names.
#[test]
fn completions_and_the_man_page_name_every_verb_on_stdout() {
    let verbs: Vec<&str> = sherd::cli::USAGE
        .lines()
        .filter_map(|l| l.strip_prefix("  sherd "))
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    assert!(verbs.contains(&"completions"), "{verbs:?}");
    for argv in [
        &["completions", "bash"][..],
        &["completions", "zsh"],
        &["completions", "fish"],
        &["completions", "elvish"],
        &["completions", "powershell"],
        &["man"],
    ] {
        let out = sherd(&argv.join("-"), argv);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        let text = String::from_utf8_lossy(&out.stdout);
        for v in &verbs {
            assert!(text.contains(v), "{argv:?} does not name `{v}`");
        }
    }
}

/// `src/cli:B5`+`B9`: a relative `[dir]` naming another repository reads that
/// repository, with the name joined on once.
#[test]
fn a_relative_dir_naming_another_repository_reads_it() {
    let parent = scratch("foreign");
    let theirs = parent.join("theirs");
    repo_with_a_node(&theirs);
    let out = sherd_in(&parent, &["lens", "theirs"]);
    let _ = std::fs::remove_dir_all(&parent);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}
