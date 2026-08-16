//! Landing generated work on `main` -- when it has earned it.
//!
//! `apply` writes code a local model produced and commits it to a run branch.
//! Nothing here decides whether that code is good; it decides whether the
//! evidence about it is strong enough to move onto the trunk, and refuses
//! with the number when it is not.

use std::path::Path;

/// Believability a node must reach before its work lands unattended.
///
/// Laplace is `(kept+1)/(tried+2)`, so 0.85 is five consecutive keeps
/// (6/7 = 0.857) and four is not enough (5/6 = 0.833). Deliberately a track
/// record rather than a property of the diff: the gate has gone green on
/// three stubs, and the blind lens is measured on ten samples drawn from
/// failures we already had names for.
pub const LAND_MIN: f64 = 0.85;

/// UTC `YYYY-MM-DD--HH-MM`.
///
/// Takes the epoch seconds rather than reading the clock, so the branch name
/// a run produces is a pure function of when the run started -- testable, and
/// stable across the many `apply` calls one run makes.
#[must_use]
pub fn stamp(secs: u64) -> String {
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // civil-from-days: shift the era to start in March so the leap day is last
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe as i64 + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}--{:02}-{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// The branch a run's work belongs on.
///
/// Reused when already on one, so an overnight run that calls `apply` twenty
/// times produces ONE branch with twenty commits rather than twenty branches.
/// Minute granularity would collide anyway between two applies in the same
/// minute, and a run is the unit anyone actually reviews.
#[must_use]
pub fn run_branch(current: &str, secs: u64) -> String {
    if current.starts_with("bbx/apply-") {
        current.to_string()
    } else {
        format!("bbx/apply-{}", stamp(secs))
    }
}

/// What is known about a branch's work at landing time.
#[derive(Debug, Clone)]
pub struct Evidence {
    pub commits: usize,
    /// `cargo build` + `cargo test` + `bbx check`, re-run on the branch head.
    pub gate_ok: bool,
    /// Mechanical review findings across every commit on the branch.
    pub findings: usize,
    /// Nodes this branch could be attributed to. Zero means UNKNOWN, which is
    /// not the same as trustworthy.
    pub nodes: usize,
    /// The LOWEST believability among the nodes this branch touched. A branch
    /// is as trustworthy as its least proven node, not its average one.
    pub believability: f64,
}

/// Whether this branch may fast-forward onto `main`.
///
/// The blind lens is not re-run here: `drive_from` already applied it to every
/// commit before it existed, and a second identical call would be a second
/// reading of one rule rather than a second opinion.
///
/// # Errors
/// Returns the reason, with the number that produced it, so a refusal is
/// actionable rather than a verdict.
pub fn landable(e: &Evidence) -> Result<(), String> {
    if e.commits == 0 {
        return Err("nothing to land -- no commits on this branch".into());
    }
    if !e.gate_ok {
        return Err("gate is red on the branch head".into());
    }
    if e.findings > 0 {
        return Err(format!(
            "{} review finding(s) -- the gate and review disagree, and review wins here",
            e.findings
        ));
    }
    if e.nodes == 0 {
        // No `pub fn` attributable to a node. Unknown believability, and an
        // unmeasured node must not read as a perfect one -- the same rule as
        // `.:V48`, where an unreadable file is a failure and never a quiet zero.
        return Err("cannot attribute this branch to a node -- believability \
                    unknown, so it does not land unattended"
            .into());
    }
    if e.believability < LAND_MIN {
        // Say what would change it. "Not believable enough" without the
        // arithmetic is the bare int V24 refuses.
        return Err(format!(
            "believability {:.2} < {LAND_MIN:.2} -- this node has not earned an \
             unattended merge. `bbx outcome <node> kept` after review is what \
             raises it; five consecutive keeps clears the bar",
            e.believability
        ));
    }
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let o = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// The configured push remote, when there is one.
///
/// `gitlab` by name: this repo has exactly one and naming it is honest about
/// that. Absent remote is not an error -- a clone with no remote is a valid
/// place to run, it just gets no CI.
#[must_use]
pub fn remote(root: &Path) -> Option<String> {
    git(root, &["remote"])
        .ok()?
        .lines()
        .find(|r| *r == "gitlab")
        .map(ToString::to_string)
}

/// Push `branch` to the remote, if there is one. Best-effort by design.
///
/// Every commit goes up so the remote runs CI on it -- an independent check
/// on a machine that is not the one that wrote the code. Pushing a RUN BRANCH
/// is not the same as publishing: `main` still only moves through [`land`],
/// so CI sees the work without the trunk carrying it.
///
/// A failed push is reported and never fatal. The commit is already made, and
/// losing it because a network was down would be worse than being unpushed.
pub fn push_branch(root: &Path, branch: &str) {
    let Some(r) = remote(root) else { return };
    match git(root, &["push", "-q", "--set-upstream", &r, branch]) {
        Ok(_) => eprintln!("  pushed {branch} to {r} -- CI runs on it there"),
        Err(e) => eprintln!("  push to {r} FAILED (commit is local only): {e}"),
    }
}

/// The branch `HEAD` is on.
///
/// # Errors
/// Propagates git failure.
pub fn current_branch(root: &Path) -> Result<String, String> {
    git(root, &["rev-parse", "--abbrev-ref", "HEAD"])
}

/// Gather the evidence about `branch` relative to `main`.
///
/// # Errors
/// Propagates git failure.
pub fn evidence(
    root: &Path,
    branch: &str,
    gate_ok: bool,
) -> Result<Evidence, String> {
    let shas = git(root, &["rev-list", &format!("main..{branch}")])?;
    let shas: Vec<&str> = shas.lines().filter(|l| !l.is_empty()).collect();
    let mut findings = 0;
    let mut lowest = 1.0_f64;
    let mut nodes = std::collections::BTreeSet::new();
    for sha in &shas {
        findings += crate::review::commit(root, sha)
            .map_err(|e| e.to_string())?
            .len();
        for (node, _) in crate::review::added_in_commit(root, sha) {
            // The node is the directory holding the file the commit touched.
            if let Some(dir) = node.parent() {
                lowest = lowest.min(crate::plan::believability(dir));
                nodes.insert(dir.to_path_buf());
            }
        }
    }
    Ok(Evidence {
        commits: shas.len(),
        gate_ok,
        findings,
        nodes: nodes.len(),
        believability: lowest,
    })
}

/// Fast-forward `main` to `branch`, or refuse and say why.
///
/// FF only. A branch that has diverged from `main` means someone committed to
/// the trunk meanwhile, and resolving a conflict inside generated code
/// unattended is exactly the operation nobody wants running at 3am.
///
/// # Errors
/// Refusal reason, or git failure.
pub fn land(root: &Path, push: bool) -> Result<String, String> {
    let branch = current_branch(root)?;
    if branch == "main" || branch == "master" {
        return Err(format!("already on {branch} -- nothing to land"));
    }
    if !git(root, &["status", "--porcelain"])?.is_empty() {
        return Err(
            "working tree dirty -- land moves committed work only".into()
        );
    }
    let gate_ok = crate::tdd::gate(root)?.0;
    let e = evidence(root, &branch, gate_ok)?;
    eprintln!(
        "land: {} commit(s) on {branch} · gate {} · {} finding(s) · {} node(s) · believability {:.2}",
        e.commits,
        if e.gate_ok { "green" } else { "RED" },
        e.findings,
        e.nodes,
        e.believability
    );
    landable(&e)?;

    git(root, &["checkout", "main"])?;
    if let Err(err) = git(root, &["merge", "--ff-only", &branch]) {
        // Leave the branch exactly where it is. It is the record of the try.
        git(root, &["checkout", &branch])?;
        return Err(format!("not a fast-forward -- main moved. {err}"));
    }
    if push && let Some(r) = remote(root) {
        git(root, &["push", "-q", &r, "main"])?;
        return Ok(format!("{branch} landed on main and pushed to {r}"));
    }
    Ok(format!(
        "{branch} landed on main{}",
        if push {
            " (no remote)"
        } else {
            " (local only)"
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamp_is_utc_civil_time() {
        assert_eq!(stamp(0), "1970-01-01--00-00");
        // 2026-08-02T14:35:00Z
        assert_eq!(stamp(1_785_681_300), "2026-08-02--14-35");
        // leap day, the case the era shift exists for
        assert_eq!(stamp(1_709_208_000), "2024-02-29--12-00");
    }

    #[test]
    fn a_run_gets_one_branch_not_one_per_apply() {
        let first = run_branch("main", 1_785_681_300);
        assert_eq!(first, "bbx/apply-2026-08-02--14-35");
        // The second apply of the same run is already on it, and a later clock
        // must not move it -- twenty applies, one branch.
        assert_eq!(run_branch(&first, 1_785_681_300 + 9_000), first);
    }

    fn ev(
        commits: usize,
        gate_ok: bool,
        findings: usize,
        believability: f64,
    ) -> Evidence {
        Evidence {
            commits,
            gate_ok,
            findings,
            nodes: 1,
            believability,
        }
    }

    #[test]
    fn an_unattributable_branch_is_unknown_not_trustworthy() {
        // The default `lowest` is 1.0, so a branch touching no node would
        // otherwise read as a PERFECT record. Absence is not evidence.
        let e = Evidence {
            commits: 1,
            gate_ok: true,
            findings: 0,
            nodes: 0,
            believability: 1.0,
        };
        assert!(landable(&e).unwrap_err().contains("cannot attribute"));
    }

    #[test]
    fn nothing_lands_until_the_node_has_earned_it() {
        // Today's real state: zero outcomes recorded, so every node sits at
        // the untried 0.50 and nothing may land unattended.
        let e = ev(1, true, 0, 0.50);
        assert!(landable(&e).unwrap_err().contains("believability"));
        // Four keeps is not enough, five is.
        assert!(landable(&ev(1, true, 0, 5.0 / 6.0)).is_err());
        assert!(landable(&ev(1, true, 0, 6.0 / 7.0)).is_ok());
    }

    #[test]
    fn a_perfect_record_does_not_excuse_a_finding_or_a_red_gate() {
        // Believability is a track record, not a pass. It cannot outvote the
        // evidence about THIS branch.
        assert!(
            landable(&ev(1, false, 0, 1.0))
                .unwrap_err()
                .contains("gate")
        );
        assert!(
            landable(&ev(1, true, 1, 1.0))
                .unwrap_err()
                .contains("finding")
        );
        assert!(
            landable(&ev(0, true, 0, 1.0))
                .unwrap_err()
                .contains("nothing to land")
        );
    }

    #[test]
    fn a_refusal_carries_the_number_that_caused_it() {
        let e = landable(&ev(1, true, 0, 0.50)).unwrap_err();
        assert!(e.contains("0.50") && e.contains("0.85"), "{e}");
    }
}
