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
