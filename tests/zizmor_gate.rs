//! `scripts/zizmor-gate.sh`, the body of the `zizmor` step in `hk.pkl`, run
//! against a STUB zizmor on `PATH` (`.:T110`).
//!
//! The step used to print "found a workflow security finding" for ANY
//! non-zero exit, including a run that never audited: in a Claude Code cloud
//! session the placeholder `GH_TOKEN` made GitHub answer 401 and the gate
//! blamed the workflow. zizmor's own exit codes separate the two -- 1 is a
//! tool failure, 10-14 a finding by severity -- so the stub answers with
//! those and the script is judged on what it SAYS and whether it still FAILS.
//!
//! A stub rather than the real binary: the real one needs the dev shell and,
//! online, the network, and neither belongs in a unit-speed test.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// What one stub run did: the script's verdict and every zizmor invocation.
struct Ran {
    out: Output,
    calls: String,
}

impl Ran {
    fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.out.stderr).into_owned()
    }
}

/// A scratch directory unique to this test, outside any repository.
fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("sherd-zizmor-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("bin")).unwrap_or_else(|e| {
        panic!("scratch dir {}: {e}", d.display());
    });
    d
}

/// Write an executable stub and probe it until it execs. A child forked while
/// the file is still open for writing makes exec fail `ETXTBSY` (`.:B32`);
/// `src/testrepo` solves it but is `cfg(test)` and invisible to this target.
fn write_script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, body).unwrap_or_else(|e| panic!("stub: {e}"));
    std::fs::set_permissions(path, PermissionsExt::from_mode(0o755))
        .unwrap_or_else(|e| panic!("stub chmod: {e}"));
    for _ in 0..100 {
        match Command::new(path).arg("--sherd-probe").output() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => panic!("probe {}: {e}", path.display()),
            Ok(_) => return,
        }
    }
    panic!("{} stayed busy", path.display());
}

/// Run the gate with a zizmor that exits `online` (or `offline` when handed
/// `--offline`) and prints `err` to stderr. `cloud` sets
/// `CLAUDE_CODE_REMOTE=true` the way a cloud session does, along with the
/// placeholder tokens it ships.
fn run(tag: &str, cloud: bool, online: i32, offline: i32, err: &str) -> Ran {
    let dir = scratch(tag);
    let log = dir.join("calls.log");
    let stub = format!(
        "#!/bin/sh\n\
         [ \"$1\" = --sherd-probe ] && exit 0\n\
         echo \"args=$* GH_TOKEN=${{GH_TOKEN-unset}} GITHUB_TOKEN=${{GITHUB_TOKEN-unset}}\" >> '{}'\n\
         echo '{err}' >&2\n\
         case \" $* \" in *' --offline '*) exit {offline};; esac\n\
         exit {online}\n",
        log.display()
    );
    write_script(&dir.join("bin/zizmor"), &stub);
    let path = format!(
        "{}:{}",
        dir.join("bin").display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut cmd = Command::new("bash");
    cmd.arg(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/zizmor-gate.sh"),
    )
    .arg("wf.yml")
    .current_dir(&dir)
    .env("PATH", path)
    .env("GH_TOKEN", "proxy-injected")
    .env("GITHUB_TOKEN", "proxy-injected")
    .env_remove("CLAUDE_CODE_REMOTE");
    if cloud {
        cmd.env("CLAUDE_CODE_REMOTE", "true");
    }
    let out = cmd.output().unwrap_or_else(|e| panic!("spawn gate: {e}"));
    let calls = std::fs::read_to_string(&log).unwrap_or_default();
    Ran { out, calls }
}

#[test]
fn a_clean_audit_passes_in_silence() {
    let r = run("clean", false, 0, 0, "");
    assert!(r.out.status.success());
    assert!(!r.stderr().contains("hk:"), "{}", r.stderr());
}

#[test]
fn a_finding_keeps_the_finding_message_and_fails() {
    let r = run("finding", false, 14, 14, "error[template-injection]");
    assert_eq!(r.out.status.code(), Some(1));
    assert!(
        r.stderr()
            .contains("zizmor found a workflow security finding"),
        "{}",
        r.stderr()
    );
    assert!(!r.stderr().contains("could not run"), "{}", r.stderr());
}

#[test]
fn a_run_that_never_audited_says_so_and_fails() {
    let r = run("error", false, 1, 1, "fatal: no audit was performed");
    assert_eq!(r.out.status.code(), Some(1));
    assert!(
        r.stderr()
            .contains("zizmor could not run: fatal: no audit was performed"),
        "{}",
        r.stderr()
    );
    assert!(
        !r.stderr().contains("found a workflow security finding"),
        "a finding was reported that no audit made: {}",
        r.stderr()
    );
}

#[test]
fn outside_the_cloud_the_tokens_and_the_online_run_are_untouched() {
    let r = run("local", false, 1, 0, "fatal: no audit was performed");
    assert!(r.calls.contains("GH_TOKEN=proxy-injected"), "{}", r.calls);
    assert!(!r.calls.contains("--offline"), "{}", r.calls);
    assert_eq!(r.out.status.code(), Some(1));
}

#[test]
fn in_the_cloud_zizmor_runs_with_both_tokens_unset() {
    let r = run("cloud-anon", true, 0, 0, "");
    assert!(r.out.status.success());
    assert!(
        r.calls.contains("GH_TOKEN=unset GITHUB_TOKEN=unset"),
        "{}",
        r.calls
    );
    assert!(!r.calls.contains("--offline"), "{}", r.calls);
}

#[test]
fn in_the_cloud_an_unreachable_github_falls_back_to_offline_audits() {
    let r = run("cloud-offline", true, 1, 0, "request error");
    assert!(r.out.status.success(), "{}", r.stderr());
    assert!(r.calls.contains("--offline"), "{}", r.calls);
    assert!(r.stderr().contains("offline"), "{}", r.stderr());
}

#[test]
fn in_the_cloud_a_finding_is_not_retried_offline() {
    let r = run("cloud-finding", true, 13, 0, "error[artipacked]");
    assert_eq!(r.out.status.code(), Some(1));
    assert!(!r.calls.contains("--offline"), "{}", r.calls);
    assert!(
        r.stderr().contains("found a workflow security finding"),
        "{}",
        r.stderr()
    );
}

#[test]
fn in_the_cloud_an_offline_failure_still_fails_as_could_not_run() {
    let r = run("cloud-dead", true, 1, 1, "fatal: no audit was performed");
    assert_eq!(r.out.status.code(), Some(1));
    assert!(
        r.stderr().contains("zizmor could not run:"),
        "{}",
        r.stderr()
    );
}
