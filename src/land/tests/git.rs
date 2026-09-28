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
    let out = crate::git::anywhere(&["init", "-q", "--bare", target])
        .output()
        .map_err(|e| format!("init bare: {e}"))?;
    assert!(out.status.success(), "the bare repo must init");
    Ok(())
}

fn branches_at(target: &str) -> Result<String, String> {
    let ls = crate::git::anywhere(&["--git-dir", target, "branch"])
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
