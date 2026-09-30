use super::*;
use crate::testrepo::TestRepo;

/// A scripted `cargo` in `dir`. Each subcommand exits with the code in
/// `codes` (`test`, `fmt`, `clippy`) and `clippy` prints `lint` on stderr,
/// where `debt::measure` reads it.
fn toolchain(
    dir: &Path,
    codes: (u8, u8, u8),
    lint: &str,
) -> Result<String, String> {
    let p = dir.join("scripted-cargo");
    let (test, fmt, clippy) = codes;
    let body = format!(
        "#!/bin/sh\ncase \"$1\" in\n\
         test) echo 'test result: scripted'; exit {test} ;;\n\
         fmt) exit {fmt} ;;\n\
         clippy) printf '%s\\n' '{lint}' >&2; exit {clippy} ;;\n\
         esac\nexit 0\n"
    );
    std::fs::write(&p, body).map_err(|e| format!("write: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, PermissionsExt::from_mode(0o755))
            .map_err(|e| format!("chmod: {e}"))?;
    }
    Ok(p.display().to_string())
}

/// `V12`: a repo with no `.sherd-slices` has no slices to drift, so the gate
/// reports that and goes on. It returned `Err` on the missing file, so
/// `sherd land` refused in every repo that never declared a slice (`B6`).
#[test]
fn a_repo_with_no_slice_registry_is_gated_not_refused() -> Result<(), String> {
    let r = TestRepo::new("gate-no-slices")?;
    let cargo = toolchain(r.path(), (0, 0, 0), "")?;
    let (ok, report) = gate_with(r.path(), &cargo)?;
    assert!(ok, "every step green: {report}");
    assert!(report.contains("=== slice: none required ==="), "{report}");
    assert!(report.contains("=== cargo test: PASS ==="), "{report}");
    assert!(report.contains("=== fmt: PASS ==="), "{report}");
    Ok(())
}

/// `V12`'s other half: a registry that EXISTS and cannot be parsed is an
/// error, never "none required".
#[test]
fn a_registry_that_exists_and_cannot_be_read_is_an_error() -> Result<(), String>
{
    let r = TestRepo::new("gate-bad-slices")?;
    r.write(".sherd-slices", "one-field-only\n")?;
    let cargo = toolchain(r.path(), (0, 0, 0), "")?;
    let err = gate_with(r.path(), &cargo)
        .err()
        .ok_or("a malformed registry passed the gate")?;
    assert!(err.contains(".sherd-slices:1"), "{err}");
    Ok(())
}

/// A failing `cargo test` is a RED gate, reported as such -- not an error.
#[test]
fn a_failing_test_run_makes_the_gate_red() -> Result<(), String> {
    let r = TestRepo::new("gate-red-tests")?;
    let cargo = toolchain(r.path(), (101, 0, 0), "")?;
    let (ok, report) = gate_with(r.path(), &cargo)?;
    assert!(!ok);
    assert!(report.contains("=== cargo test: FAIL ==="), "{report}");
    Ok(())
}

/// A toolchain that cannot be RUN has said nothing, so it is an error and
/// never a red gate (`.:V48`).
#[test]
fn a_toolchain_that_cannot_run_is_an_error_not_a_red_gate() -> Result<(), String>
{
    let r = TestRepo::new("gate-no-cargo")?;
    let missing = r.path().join("no-such-cargo").display().to_string();
    let err = gate_with(r.path(), &missing)
        .err()
        .ok_or("an absent toolchain produced a verdict")?;
    assert!(err.contains("could not RUN"), "{err}");
    Ok(())
}

/// Unformatted code is refused here because `hk` refuses it at commit (B30).
#[test]
fn unformatted_code_makes_the_gate_red() -> Result<(), String> {
    let r = TestRepo::new("gate-fmt")?;
    let cargo = toolchain(r.path(), (0, 1, 0), "")?;
    let (ok, report) = gate_with(r.path(), &cargo)?;
    assert!(!ok);
    assert!(report.contains("=== fmt: FAIL ==="), "{report}");
    Ok(())
}

/// The ratchet: a density above the recorded ceiling is RED, and one at the
/// ceiling passes. Both halves, so neither verdict is a constant.
#[test]
fn lint_density_may_hold_but_never_rise() -> Result<(), String> {
    let r = TestRepo::new("gate-debt")?;
    r.write("src/a.rs", &"fn f() {}\n".repeat(100))?;
    let lint = "src/a.rs:1:1: warning: indexing may panic";
    let cargo = toolchain(r.path(), (0, 0, 0), lint)?;

    r.write(".lint-debt", "density 0.0\n")?;
    let (ok, report) = gate_with(r.path(), &cargo)?;
    assert!(!ok, "0.0 -> 10.0 per KLoC rose: {report}");
    assert!(report.contains("=== lint density: ROSE ==="), "{report}");

    r.write(".lint-debt", "density 10.0\n")?;
    let (ok, report) = gate_with(r.path(), &cargo)?;
    assert!(ok, "10.0 at a ceiling of 10.0 holds: {report}");
    assert!(report.contains("=== lint density: PASS ==="), "{report}");
    Ok(())
}

/// `gate` is `gate_with` over the process's own toolchain. A tree with no
/// `Cargo.toml` is a red gate: cargo RAN and refused, which is a verdict.
#[test]
fn the_default_toolchain_runs_and_a_tree_without_a_crate_is_red()
-> Result<(), String> {
    let r = TestRepo::new("gate-default")?;
    let (ok, report) = gate(r.path())?;
    assert!(!ok, "{report}");
    assert!(report.contains("=== cargo test: FAIL ==="), "{report}");
    Ok(())
}
