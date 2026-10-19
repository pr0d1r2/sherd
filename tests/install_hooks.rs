//! `scripts/install-hooks.sh`, the dev shell's hook installer, run against
//! throwaway repositories: it must set `core.hooksPath` in sherd's own work
//! tree, leave anything else alone, and say so when `hk` is missing.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/install-hooks.sh")
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("sherd-hooks-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap_or_else(|e| panic!("scratch: {e}"));
    d
}

fn git(dir: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("git: {e}"));
    String::from_utf8_lossy(&o.stdout).trim().to_string()
}

/// A repository, optionally shaped like sherd's own tree.
fn repo(tag: &str, ours: bool) -> PathBuf {
    let d = scratch(tag);
    git(&d, &["init", "-q"]);
    if ours {
        std::fs::write(d.join("hk.pkl"), "").unwrap_or_else(|e| panic!("{e}"));
        std::fs::create_dir(d.join(".githooks"))
            .unwrap_or_else(|e| panic!("{e}"));
    }
    d
}

/// Run the installer in `dir`; `PATH` is empty of `hk` unless `with_hk`.
fn run(dir: &Path, with_hk: bool) -> Output {
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap_or_else(|e| panic!("{e}"));
    let mut path = format!("{}:{}", tool_dir("git"), tool_dir("bash"));
    if with_hk {
        let hk = bin.join("hk");
        std::fs::write(&hk, "#!/bin/sh\n").unwrap_or_else(|e| panic!("{e}"));
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hk, PermissionsExt::from_mode(0o755))
            .unwrap_or_else(|e| panic!("{e}"));
        path = format!("{}:{path}", bin.display());
    }
    Command::new("bash")
        .arg(script())
        .current_dir(dir)
        .env("PATH", path)
        .output()
        .unwrap_or_else(|e| panic!("run: {e}"))
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

#[test]
fn sets_hooks_path_in_our_tree() {
    let d = repo("set", true);
    let o = run(&d, true);
    assert!(o.status.success());
    assert_eq!(
        git(&d, &["config", "--local", "core.hooksPath"]),
        ".githooks"
    );
    assert!(!stderr(&o).contains("hk is not on PATH"));
}

#[test]
fn keeps_an_existing_hooks_path() {
    let d = repo("keep", true);
    git(&d, &["config", "--local", "core.hooksPath", "elsewhere"]);
    run(&d, true);
    assert_eq!(
        git(&d, &["config", "--local", "core.hooksPath"]),
        "elsewhere"
    );
}

#[test]
fn leaves_a_foreign_repo_alone() {
    let d = repo("foreign", false);
    let o = run(&d, true);
    assert!(o.status.success());
    assert_eq!(git(&d, &["config", "--local", "core.hooksPath"]), "");
}

#[test]
fn leaves_a_subdirectory_alone() {
    let d = repo("sub", true);
    let sub = d.join("sub");
    std::fs::create_dir(&sub).unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(sub.join("hk.pkl"), "").unwrap_or_else(|e| panic!("{e}"));
    std::fs::create_dir(sub.join(".githooks"))
        .unwrap_or_else(|e| panic!("{e}"));
    run(&sub, true);
    assert_eq!(git(&d, &["config", "--local", "core.hooksPath"]), "");
}

#[test]
fn is_loud_but_still_installs_when_hk_is_missing() {
    let d = repo("nohk", true);
    let o = run(&d, false);
    assert!(o.status.success(), "must not lock the shell out");
    assert!(stderr(&o).contains("hk is not on PATH"));
    assert_eq!(
        git(&d, &["config", "--local", "core.hooksPath"]),
        ".githooks"
    );
}

#[test]
fn outside_any_repo_is_silent_success() {
    let d = scratch("norepo");
    let o = run(&d, false);
    assert!(o.status.success());
    assert_eq!(stderr(&o), "");
}

/// Directory holding `tool` on the ambient `PATH`; the run's own `PATH` is
/// built from these alone so a real `hk` cannot leak in.
fn tool_dir(tool: &str) -> String {
    let o = Command::new("sh")
        .args(["-c", &format!("command -v {tool}")])
        .output()
        .unwrap_or_else(|e| panic!("{e}"));
    let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
    Path::new(&p)
        .parent()
        .map(|d| d.display().to_string())
        .unwrap_or_default()
}
