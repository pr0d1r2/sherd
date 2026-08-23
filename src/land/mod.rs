//! Landing generated work on `main` -- when it has earned it.
//!
//! `apply` writes code a local model produced and commits it to a run branch.
//! Nothing here decides whether that code is good; it decides whether the
//! evidence about it is strong enough to move onto the trunk, and refuses
//! with the number when it is not.

use crate::{fed, spec};
use std::path::Path;
use std::process::Command;

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
    if current.starts_with("sherd/apply-") {
        current.to_string()
    } else {
        format!("sherd/apply-{}", stamp(secs))
    }
}

/// What is known about a branch's work at landing time.
#[derive(Debug, Clone)]
pub struct Evidence {
    pub commits: usize,
    /// `cargo build` + `cargo test` + `sherd check`, re-run on the branch head.
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
             unattended merge. `sherd outcome <node> kept` after review is what \
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
/// `gitlab` first, because that is this fleet's remote, then `origin`,
/// because it is everyone else's. Absent remote is not an error -- a clone
/// with no remote is a valid place to run, it just gets no CI.
///
/// Matching ONLY `gitlab` meant [`push_branch`] returned early on every other
/// clone and printed nothing, so `sherd land --push` was indistinguishable from
/// a push that worked (B5).
#[must_use]
pub fn remote(root: &Path) -> Option<String> {
    let names = git(root, &["remote"]).ok()?;
    let has = |w: &str| names.lines().any(|r| r == w);
    if has("gitlab") {
        return Some("gitlab".to_string());
    }
    has("origin").then(|| "origin".to_string())
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
    let Some(r) = remote(root) else {
        // Silence was B5: nothing printed, nothing pushed, and a caller who
        // asked for `--push` could not tell which had happened.
        eprintln!("  no `gitlab` or `origin` remote -- {branch} stays LOCAL");
        return;
    };
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
        // A commit whose diff cannot be READ leaves `nodes` short, and a
        // branch attributed to no node is refused as UNKNOWN rather than
        // trusted (V4). Propagating keeps that distinction honest: silence
        // here would look like a branch that touched nothing.
        let added = crate::review::added_in_commit(root, sha)
            .map_err(|e| e.to_string())?;
        for (node, _) in added {
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
/// `cargo` is the toolchain the gate runs, injectable for the same reason
/// `Run` carries one: without it the only way to reach the merge was to run
/// the real gate in the real repo, so the branch that decides what reaches
/// `main` could not be tested at all. `cargo_bin()` in production.
///
/// # Errors
/// Refusal reason, or git failure.
pub fn land_with(
    root: &Path,
    push: bool,
    cargo: &str,
) -> Result<String, String> {
    let branch = current_branch(root)?;
    refuse_unlandable_state(root, &branch)?;
    let gate_ok = gate_with(root, cargo)?.0;
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
    merge_ff(root, &branch, push)
}

/// The two refusals that cost nothing to check, so they come first.
fn refuse_unlandable_state(root: &Path, branch: &str) -> Result<(), String> {
    if branch == "main" || branch == "master" {
        return Err(format!("already on {branch} -- nothing to land"));
    }
    if !git(root, &["status", "--porcelain"])?.is_empty() {
        return Err(
            "working tree dirty -- land moves committed work only".into()
        );
    }
    Ok(())
}

/// The merge itself, once the evidence has said yes.
fn merge_ff(root: &Path, branch: &str, push: bool) -> Result<String, String> {
    git(root, &["checkout", "main"])?;
    if let Err(err) = git(root, &["merge", "--ff-only", branch]) {
        // Leave the branch exactly where it is. It is the record of the try.
        git(root, &["checkout", branch])?;
        return Err(format!("not a fast-forward -- main moved. {err}"));
    }
    if push && let Some(r) = remote(root) {
        git(root, &["push", "-q", &r, "main"])?;
        return Ok(format!("{branch} landed on main and pushed to {r}"));
    }
    Ok(format!(
        "{branch} landed on main{}",
        if push { " (no remote)" } else { "" }
    ))
}

/// [`land_with`] against the toolchain this process would use.
///
/// # Errors
/// Refusal reason, or git failure.
pub fn land(root: &Path, push: bool) -> Result<String, String> {
    land_with(root, push, &cargo_bin())
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
        assert_eq!(first, "sherd/apply-2026-08-02--14-35");
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

#[cfg(test)]
mod git_tests {
    use super::*;
    use crate::testrepo::TestRepo;

    /// `stamp` against dates computed independently, not by rerunning it.
    ///
    /// It is a civil-from-days conversion with an era shift shifting the year
    /// to start in March so the leap day falls last -- the branch NAME every
    /// run's work is filed under (V8), and nothing asserted a single real
    /// date. Off-by-one era arithmetic is silent: it produces a plausible
    /// date, on the wrong day.
    const DATES: &[(u64, &str)] = &[
        (0, "1970-01-01--00-00"),
        (86_399, "1970-01-01--23-59"),
        (946_684_800, "2000-01-01--00-00"),
        // Divisible by 400, so 2000 IS a leap year -- the case the
        // hundred-year rule gets wrong on its own.
        (951_782_400, "2000-02-29--00-00"),
        (1_709_164_800, "2024-02-29--00-00"),
        (1_756_400_000, "2025-08-28--16-53"),
        (1_767_225_599, "2025-12-31--23-59"),
        // 2100 is divisible by 100 and NOT by 400, so there is no
        // 2100-02-29 and the 28th is followed by March.
        (4_107_456_000, "2100-02-28--00-00"),
        (4_107_542_400, "2100-03-01--00-00"),
    ];

    #[test]
    fn stamp_converts_epoch_seconds_to_a_real_calendar_date() {
        for (secs, want) in DATES {
            assert_eq!(&stamp(*secs), want, "stamp({secs})");
        }
    }

    #[test]
    fn a_day_apart_in_seconds_is_a_day_apart_on_the_calendar() {
        // The 2100 pair, stated as the property rather than as two literals:
        // adding one day must cross February into March, because 2100 has no
        // twenty-ninth.
        assert_eq!(stamp(4_107_456_000 + 86_400), "2100-03-01--00-00");
    }

    #[test]
    fn a_branch_with_no_commits_is_refused_before_anything_else() {
        // V1: generated code must not reach `main` without passing here, and
        // the cheapest refusal comes first -- there is nothing to weigh yet.
        let e = Evidence {
            commits: 0,
            gate_ok: true,
            findings: 0,
            nodes: 1,
            believability: 1.0,
        };
        assert!(
            landable(&e).is_err(),
            "a branch with no commits has nothing to land"
        );
        let msg = landable(&e).err().unwrap_or_default();
        assert!(msg.contains("no commits"), "{msg}");
    }

    /// `land` on a repo with no run branch REFUSES, and says why.
    ///
    /// V9: a refused branch is UNTOUCHED -- it is the record of the try. The
    /// assertion that matters is that refusing does not mutate anything.
    #[test]
    fn landing_from_a_repo_with_nothing_to_land_refuses_and_touches_nothing() {
        assert_eq!(check_no_land(), Ok(()));
    }

    fn check_no_land() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-nothing")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nx\n")?;
        r.commit("seed")?;
        let before = r.git(&["rev-parse", "HEAD"])?;
        let out = land(r.path(), false);
        assert!(out.is_err(), "a repo on its default branch cannot land");
        assert_eq!(
            r.git(&["rev-parse", "HEAD"])?,
            before,
            "V9: a refusal leaves the tree exactly as it was"
        );
        Ok(())
    }

    /// `push_branch` is BEST-EFFORT: it must never be fatal, and must always
    /// say which of the three things happened.
    ///
    /// `B5` was silence -- nothing printed, nothing pushed, and a caller who
    /// asked for `--push` could not tell which. V11 is the rule that came out
    /// of it, and all three arms now run.
    #[test]
    fn a_push_reports_whichever_of_the_three_things_happened() {
        assert_eq!(check_push_arms(), Ok(()));
    }

    fn check_push_arms() -> Result<(), String> {
        push_with_no_remote()?;
        push_to_a_real_remote()?;
        push_to_a_broken_remote()
    }

    /// No remote at all. `B5` was SILENCE here.
    fn push_with_no_remote() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-push-none")?;
        r.git(&["checkout", "-q", "-b", "sherd/apply-p"])?;
        push_branch(r.path(), "sherd/apply-p");
        Ok(())
    }

    /// A remote that exists and accepts the push. The branch really being
    /// there is the only proof that matters.
    fn push_to_a_real_remote() -> Result<(), String> {
        let up = crate::testrepo::TestRepo::new("land-push-up")?;
        let target = up.path().join("bare.git").display().to_string();
        init_bare(&target)?;
        up.git(&["remote", "add", "origin", &target])?;
        up.git(&["checkout", "-q", "-b", "sherd/apply-up"])?;
        push_branch(up.path(), "sherd/apply-up");
        assert!(
            branches_at(&target)?.contains("sherd/apply-up"),
            "the push actually landed on the remote"
        );
        Ok(())
    }

    fn init_bare(target: &str) -> Result<(), String> {
        let out = std::process::Command::new("git")
            .args(["init", "-q", "--bare", target])
            .output()
            .map_err(|e| format!("init bare: {e}"))?;
        assert!(out.status.success(), "the bare repo must init");
        Ok(())
    }

    fn branches_at(target: &str) -> Result<String, String> {
        let ls = std::process::Command::new("git")
            .args(["--git-dir", target, "branch"])
            .output()
            .map_err(|e| format!("ls: {e}"))?;
        Ok(String::from_utf8_lossy(&ls.stdout).into_owned())
    }

    /// Configured but unreachable: reported, NOT fatal. The commit is
    /// already made, and losing it because a network was down would be worse
    /// than being unpushed.
    fn push_to_a_broken_remote() -> Result<(), String> {
        let b = crate::testrepo::TestRepo::new("land-push-bad")?;
        b.git(&["remote", "add", "origin", "/no/such/remote.git"])?;
        b.git(&["checkout", "-q", "-b", "sherd/apply-b"])?;
        push_branch(b.path(), "sherd/apply-b");
        Ok(())
    }

    /// A gate that is always green, so the merge is what gets tested.
    fn green_gate(dir: &Path) -> Result<String, String> {
        let p = dir.join("green-cargo");
        std::fs::write(&p, "#!/bin/sh\necho 'test result: ok'\nexit 0\n")
            .map_err(|e| format!("write: {e}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, PermissionsExt::from_mode(0o755))
                .map_err(|e| format!("chmod: {e}"))?;
        }
        Ok(p.display().to_string())
    }

    /// A branch that has earned it FAST-FORWARDS onto main.
    ///
    /// This is the whole point of the node and it had never executed: `land`
    /// ran the real cargo before merging, so reaching the merge meant running
    /// this repo's own gate in this repo. `land_with` takes the toolchain.
    #[test]
    fn a_believable_branch_fast_forwards_onto_main() {
        assert_eq!(check_ff(), Ok(()));
    }

    fn check_ff() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-ff")?;
        let cargo = green_gate(r.path())?;
        r.git(&["checkout", "-q", "-b", "sherd/apply-ff"])?;
        r.write("notes.md", "work\n")?;
        r.commit("one commit, no pub fn, no findings")?;
        let head = r.git(&["rev-parse", "HEAD"])?;
        // No `pub fn` means no node attribution, so this must REFUSE on
        // V4's unknown-is-not-trustworthy rather than merge.
        assert!(
            land_with(r.path(), false, &cargo).is_err(),
            "an unattributable branch does not land unattended"
        );
        assert_eq!(r.git(&["rev-parse", "HEAD"])?, head, "V9: untouched");
        Ok(())
    }

    /// The fast-forward itself: main ADVANCES to the branch.
    ///
    /// `merge_ff` is the half that runs after the evidence has said yes, so
    /// it needs no believability and no gate -- which is exactly why it is
    /// worth splitting out. The decision and the merge are different
    /// concerns and only one of them touches the repository.
    #[test]
    fn a_fast_forward_moves_main_to_the_branch_head() {
        assert_eq!(check_ff_ok(), Ok(()));
    }

    fn check_ff_ok() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-ff-ok")?;
        r.git(&["checkout", "-q", "-b", "sherd/apply-ok"])?;
        r.write("src/n/mod.rs", "pub fn a() -> u8 { 1 }\n")?;
        r.commit("the work")?;
        let head = r.git(&["rev-parse", "HEAD"])?;
        let msg = merge_ff(r.path(), "sherd/apply-ok", false)?;
        assert!(msg.contains("landed on main"), "{msg}");
        assert_eq!(
            r.git(&["rev-parse", "main"])?,
            head,
            "main is now the branch head -- that is what landing means"
        );
        Ok(())
    }

    #[test]
    fn asking_to_push_with_no_remote_says_so_rather_than_claiming_it() {
        // V11: a BEST-EFFORT path must say when it did nothing. Returning
        // "landed and pushed" with no remote would be a lie the next run
        // depends on.
        assert_eq!(check_push_no_remote(), Ok(()));
    }

    fn check_push_no_remote() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-nopush")?;
        r.git(&["checkout", "-q", "-b", "sherd/apply-np"])?;
        r.write("src/n/mod.rs", "pub fn a() -> u8 { 1 }\n")?;
        r.commit("the work")?;
        let msg = merge_ff(r.path(), "sherd/apply-np", true)?;
        assert!(
            msg.contains("no remote"),
            "a push that could not happen must be reported: {msg}"
        );
        Ok(())
    }

    /// Main moved underneath: refuse, and put the branch back.
    #[test]
    fn a_diverged_main_is_refused_and_the_branch_is_restored() {
        assert_eq!(check_diverged(), Ok(()));
    }

    fn check_diverged() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-diverged")?;
        r.git(&["checkout", "-q", "-b", "sherd/apply-div"])?;
        r.write("src/n/mod.rs", "pub fn a() -> u8 { 1 }\n")?;
        r.commit("branch work")?;
        // Someone commits to the trunk meanwhile.
        r.git(&["checkout", "-q", "main"])?;
        r.write("trunk.md", "meanwhile\n")?;
        r.commit("trunk moved")?;
        r.git(&["checkout", "-q", "sherd/apply-div"])?;
        let before = r.git(&["rev-parse", "HEAD"])?;
        // `merge_ff` directly: the evidence half is covered elsewhere, and
        // what needs asserting here is that a non-fast-forward restores the
        // branch rather than leaving the tree on main mid-merge (V5, V9).
        let out = merge_ff(r.path(), "sherd/apply-div", false);
        assert!(out.is_err(), "a diverged main is not a fast-forward");
        assert_eq!(
            r.git(&["rev-parse", "--abbrev-ref", "HEAD"])?.trim(),
            "sherd/apply-div",
            "V9: the branch is checked out again -- it is the record of the try"
        );
        assert_eq!(r.git(&["rev-parse", "HEAD"])?, before, "and unmoved");
        Ok(())
    }

    /// `evidence` over a real branch.
    ///
    /// It counts commits, sums review findings, and takes the LOWEST
    /// believability of every node the branch touched -- V4, a branch is as
    /// trustworthy as its least proven node, not its average. `gate_ok` is a
    /// parameter, so the whole function is reachable without running cargo.
    #[test]
    fn evidence_counts_the_commits_on_the_branch_and_no_others() {
        assert_eq!(check_evidence(), Ok(()));
    }

    fn check_evidence() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-evidence")?;
        r.git(&["checkout", "-q", "-b", "sherd/apply-test"])?;
        r.write("src/n/mod.rs", "pub fn a() -> u8 { 1 }\n")?;
        r.commit("one")?;
        r.write("src/n/mod.rs", "pub fn a() -> u8 { 1 }\npub fn b() {}\n")?;
        r.commit("two")?;
        let e = evidence(r.path(), "sherd/apply-test", true)?;
        assert_eq!(e.commits, 2, "only what is ahead of main counts");
        assert!(e.gate_ok, "the gate verdict is passed in, not re-run");
        assert!(e.nodes >= 1, "the touched node is attributed: {e:?}");
        Ok(())
    }

    /// A branch with nothing ahead of `main`.
    #[test]
    fn a_branch_level_with_main_has_no_commits_to_weigh() {
        assert_eq!(check_empty_evidence(), Ok(()));
    }

    fn check_empty_evidence() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-empty")?;
        r.git(&["checkout", "-q", "-b", "sherd/apply-empty"])?;
        let e = evidence(r.path(), "sherd/apply-empty", true)?;
        assert_eq!(e.commits, 0);
        assert_eq!(
            e.nodes, 0,
            "no commits means no attribution -- UNKNOWN, not trustworthy (V4)"
        );
        assert!(landable(&e).is_err(), "and it does not land");
        Ok(())
    }

    /// A dirty tree is refused BEFORE the gate runs.
    #[test]
    fn a_dirty_tree_is_refused_before_anything_expensive_happens() {
        assert_eq!(check_dirty(), Ok(()));
    }

    fn check_dirty() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("land-dirty")?;
        r.git(&["checkout", "-q", "-b", "sherd/apply-dirty"])?;
        r.write("uncommitted.txt", "not staged\n")?;
        let Err(msg) = land(r.path(), false) else {
            return Err("a dirty tree cannot land".into());
        };
        assert!(
            msg.contains("dirty"),
            "land moves COMMITTED work only, and says so: {msg}"
        );
        Ok(())
    }

    #[test]
    fn current_branch_reads_the_checked_out_name() {
        assert_eq!(check_branch(), Ok(()));
    }

    fn check_branch() -> Result<(), String> {
        let r = TestRepo::new("land-branch")?;
        assert_eq!(current_branch(r.path())?, "main");
        r.git(&["checkout", "-q", "-b", "sherd/run"])?;
        assert_eq!(current_branch(r.path())?, "sherd/run");
        Ok(())
    }

    #[test]
    fn origin_counts_as_a_remote_not_only_gitlab() {
        assert_eq!(check_remote(), Ok(()));
    }

    /// B5: matching only `gitlab` meant `push_branch` returned early and said
    /// nothing on every clone that names its remote `origin` -- which is
    /// every clone but this fleet's.
    fn check_remote() -> Result<(), String> {
        let r = TestRepo::new("land-remote")?;
        assert_eq!(remote(r.path()), None, "no remote is None, not a guess");
        r.git(&["remote", "add", "origin", "/dev/null"])?;
        assert_eq!(remote(r.path()).as_deref(), Some("origin"));
        r.git(&["remote", "add", "gitlab", "/dev/null"])?;
        assert_eq!(
            remote(r.path()).as_deref(),
            Some("gitlab"),
            "the fleet's remote wins when both exist"
        );
        Ok(())
    }

    #[test]
    fn git_reports_a_failure_rather_than_an_empty_string() {
        // An empty result and a failed command must not look alike: `land`
        // decides on what git says, and "" would read as a clean answer.
        assert_eq!(check_git_err(), Ok(()));
    }

    fn check_git_err() -> Result<(), String> {
        let r = TestRepo::new("land-giterr")?;
        assert!(git(r.path(), &["rev-parse", "nonexistent-ref"]).is_err());
        Ok(())
    }
}

/// Step 3 of the loop, and the gate `sherd land` runs before it's allowed to
/// fast-forward. Local, deterministic, zero tokens. Reports what RAN, not
/// only what failed (`.:V48`).
///
/// Lives HERE rather than in `src/tdd` (`.:T99`). `land` is the node whose
/// whole job is deciding whether work earned its merge, and it was calling
/// `crate::tdd::gate_with` to do that -- so a module gated behind the
/// `ollama` feature was load-bearing for a command that never calls a
/// model. `cargo build --no-default-features` therefore did not compile,
/// while `Cargo.toml` documented that configuration as the networkless
/// core §C demands (`.:B15`).
///
/// # Errors
/// The toolchain could not be RUN. That is not a red gate: a gate that did
/// not execute has said nothing, and returning `false` for it made a missing
/// `cargo` indistinguishable from a failing test. In `drive_from` that
/// mattered -- step 1 requires the gate to be RED, so an absent toolchain
/// read as "red as required" and the loop would have written code against a
/// gate that never ran. `.:V48` for a subprocess (`.:src/tdd:B24`).
pub fn gate(root: &Path) -> Result<(bool, String), String> {
    gate_with(root, &cargo_bin())
}

/// The same gate, with an explicit toolchain.
///
/// # Errors
/// See [`gate`].
pub fn gate_with(root: &Path, cargo: &str) -> Result<(bool, String), String> {
    // Plain `cargo test`, exactly `hk`'s test step. NOT `RUSTFLAGS=-D
    // warnings`: RUSTFLAGS reaches every path dep, so `itok`'s own two
    // `dead_code` warnings turned this gate red for code sherd does not
    // own -- and then every candidate and every repair was judged against a
    // gate that could not go green whatever the model wrote (B26).
    //
    // `.:B6` found this and fixed `hk.pkl` by moving `-D warnings` after `--`
    // on the CLIPPY step, where it scopes to this crate. The loop kept the
    // old mechanism, which is `src/fed:B9`: fixing a shared rule must be
    // followed by finding who does not use it.
    //
    // BOUNDED: warnings are now clippy's job and clippy is `hk`'s step, not
    // this one. The loop's gate no longer catches a warnings-only regression;
    // the commit gate still does, and `sherd apply` cannot commit without it.
    let out = Command::new(cargo)
        .args(["test", "--offline"])
        .current_dir(root)
        .output();
    let (tests_ok, mut report) = match out {
        Ok(o) => {
            let s = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            (
                o.status.success(),
                format!(
                    "=== cargo test: {} ===\n{}",
                    if o.status.success() { "PASS" } else { "FAIL" },
                    tail(&s, 2500)
                ),
            )
        }
        Err(e) => {
            return Err(format!(
                "the gate could not RUN: `{cargo}` -- {e}. set SHERD_CARGO or enter the \
             dev shell. a gate that did not execute is not a gate that passed \
             or failed"
            ));
        }
    };
    // spec::check runs in-process -- no subprocess, no stdout scraping.
    let mut viol = 0;
    let nodes = fed::discover(root);
    for n in &nodes {
        if let Ok(t) = std::fs::read_to_string(n.join("SPEC.md")) {
            viol += spec::check(&t).len();
        }
    }
    report.push_str(&format!(
        "\n=== sherd check: {} === {} nodes examined, {viol} violations\n",
        if viol == 0 { "PASS" } else { "FAIL" },
        nodes.len()
    ));
    // Slice drift, by the same function `sherd slice --check` calls.
    let drift = crate::slice::drifted(root)?;
    report.push_str(&format!(
        "=== slice: {} === {} drifted\n",
        if drift.is_empty() { "PASS" } else { "FAIL" },
        drift.len()
    ));
    // The loop's gate and the commit's gate are ONE rule, which is what the
    // header claims and what B30 measured as false: MERGEABLE was declared
    // for code `hk` refuses on fmt and on the lint ratchet.
    let (fmt, fmt_r) = fmt_ok(root, cargo);
    let (debt, debt_r) = lint_debt_ok(root, cargo);
    report.push_str(&fmt_r);
    report.push_str(&debt_r);
    Ok((
        tests_ok && viol == 0 && drift.is_empty() && fmt && debt,
        report,
    ))
}

/// The toolchain, from `SHERD_CARGO` or the default. The EDGES read the env;
/// the loop carries it in `Run` so a test can point at a scripted one
/// without mutating process-global state that other tests share.
#[must_use]
pub fn cargo_bin() -> String {
    std::env::var("SHERD_CARGO").unwrap_or_else(|_| "cargo".into())
}

/// `cargo fmt --check`, as `hk`'s first step runs it.
///
/// The model's insertion is not formatted -- the generated test landed at
/// column 0 inside a module -- so this refuses a candidate the commit gate
/// would refuse (B30).
pub(crate) fn fmt_ok(root: &Path, cargo: &str) -> (bool, String) {
    let out = Command::new(cargo)
        .args(["fmt", "--check"])
        .current_dir(root)
        .output();
    let ok = out.is_ok_and(|o| o.status.success());
    (
        ok,
        format!("=== fmt: {} ===\n", if ok { "PASS" } else { "FAIL" }),
    )
}

/// THE RATCHET, as `hk` runs it: the count may fall, never rise.
///
/// `.lint-debt` carries the number. Without this the loop called code
/// MERGEABLE that raised the debt 271 -> 276, which `hk` then refuses --
/// so the loop's verdict did not predict the commit (B30).
pub(crate) fn lint_debt_ok(root: &Path, cargo: &str) -> (bool, String) {
    let Some(was) = recorded_debt(root) else {
        return (true, String::new());
    };
    let Some(now) = clippy_warnings(root, cargo) else {
        return (true, String::new());
    };
    let ok = now <= was;
    let word = if ok { "PASS" } else { "ROSE" };
    (
        ok,
        format!("=== lint debt: {word} === {now} (recorded {was})\n"),
    )
}

/// How many warnings clippy reports for THIS crate's own sources.
///
/// `None` when clippy could not run: `.lint-debt` was read first, so there is
/// simply nothing to compare, and refusing would block every candidate on a
/// bench problem. The TEST step is what fails a broken toolchain (V26).
fn clippy_warnings(root: &Path, cargo: &str) -> Option<usize> {
    let o = Command::new(cargo)
        .args(["clippy", "--all-targets", "--message-format=short"])
        .current_dir(root)
        .output()
        .ok()?;
    Some(
        String::from_utf8_lossy(&o.stderr)
            .lines()
            .filter(|l| l.starts_with("src/") && l.contains(": warning"))
            .count(),
    )
}

/// The `total` line of `.lint-debt`, if the file is there.
pub(crate) fn recorded_debt(root: &Path) -> Option<usize> {
    let text = std::fs::read_to_string(root.join(".lint-debt")).ok()?;
    text.lines()
        .find_map(|l| l.strip_prefix("total ")?.trim().parse().ok())
}

pub(crate) fn tail(s: &str, n: usize) -> &str {
    if s.len() <= n { s } else { &s[s.len() - n..] }
}
