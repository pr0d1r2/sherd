//! The installed binary, run as a process: `src/main.rs` and `cli::run`,
//! which every other test bypasses by calling `run_args` with an argv it
//! built. What `run` adds is reading the REAL argv and handing the exit code
//! to the OS, and only a spawned process can see either.
//!
//! Every spawn runs in a scratch directory that is not this repository
//! (`src/cli:V6`): a test that walks up into whichever tree the runner sits in
//! passes in a checkout and fails anywhere else (`src/cli:B1`).

use std::path::PathBuf;
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
    let out = Command::new(env!("CARGO_BIN_EXE_sherd"))
        .args(args)
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn sherd: {e}"));
    let _ = std::fs::remove_dir_all(&dir);
    out
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
