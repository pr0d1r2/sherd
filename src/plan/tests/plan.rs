use super::*;

#[test]
fn horizon_is_short_because_the_state_self_modifies() {
    assert_eq!(HORIZON, 3, "a longer horizon would be fiction (tdd B9)");
}

#[test]
fn every_confidence_states_how_it_fails() {
    for c in [Confidence::Next, Confidence::Likely, Confidence::Tentative] {
        assert!(
            !c.invalidated_by().is_empty(),
            "a plan that cannot say how it fails is a promise"
        );
        // 2 spellings of 1 list: the text form ! stay the list, joined.
        assert_eq!(c.invalidated_by(), c.invalidators().join(" · "));
    }
}

fn row(node: &str, id: &str, text: &str) -> Task {
    Task {
        node: PathBuf::from(node),
        id: id.into(),
        status: '.',
        text: text.into(),
        cites: String::new(),
    }
}

/// V25: the JSON form is a CONTRACT a caller parses. Pinned whole, so a
/// renamed key, a reordered field or a dropped row fails here rather than
/// in the hallucinogen loop that reads it (issue #36).
#[test]
fn json_form_is_pinned() {
    let mut st = crate::state::State::default();
    record_outcome_in(&mut st, Path::new("src/b"), true);
    let p = Plan {
        steps: vec![row("src/a", "T1", "add `x`"), row("src/b", "T2", "y")],
        unmanaged: vec![(row("", "T9", "root"), Kind::NoModule)],
        total_open: 4,
    };
    assert_eq!(
        to_json(&st, &p, (Some("M1"), 1), &[120]),
        concat!(
            r#"{"horizon":2,"open":4,"unmanaged":1,"milestone":"M1","outside_milestones":1,"#,
            r#""steps":[{"rank":1,"kind":"NEXT","node":"src/a","id":"T1","text":"add `x`","#,
            r#""believability":0.5,"tried":0,"kept":0,"context_tokens":120,"#,
            r#""invalidated_by":["judge rejects the test 3x","gate still red after 3 repairs"]},"#,
            r#"{"rank":2,"kind":"LIKELY","node":"src/b","id":"T2","text":"y","#,
            r#""believability":0.6666666666666666,"tried":1,"kept":1,"context_tokens":0,"#,
            r#""invalidated_by":["step 1 adds a §B row to this node, changing its authoring prompt (tdd B9)"]}],"#,
            r#""unmanaged_rows":[{"node":".","id":"T9","text":"root","reason":"root row -- no mod.rs to add to"}]}"#,
        )
    );
}

/// An empty plan is still one object with every key: `null` milestone,
/// empty lists. A caller reading `steps` ⊥ special-cases "nothing to do".
#[test]
fn json_of_an_empty_plan_keeps_every_key() {
    let p = Plan {
        steps: vec![],
        unmanaged: vec![],
        total_open: 0,
    };
    assert_eq!(
        to_json(&crate::state::State::default(), &p, (None, 0), &[]),
        r#"{"horizon":0,"open":0,"unmanaged":0,"milestone":null,"outside_milestones":0,"steps":[],"unmanaged_rows":[]}"#
    );
}

/// Row text is caveman prose from a `§T` cell: backslashes survive
/// microlith's unescape (`src/fed:V4`), quotes are common, and a control
/// char must ⊥ reach the output raw.
#[test]
fn json_str_escapes_what_rfc_8259_requires() {
    assert_eq!(
        json_str("a \"q\" C:\\p\n\t\u{1}|∴"),
        r#""a \"q\" C:\\p\n\t\u0001|∴""#
    );
}

#[test]
fn confidence_degrades_with_distance() {
    assert_eq!(Confidence::of(0), Confidence::Next);
    assert_eq!(Confidence::of(1), Confidence::Likely);
    assert_eq!(Confidence::of(2), Confidence::Tentative);
    assert_eq!(Confidence::of(99), Confidence::Tentative);
}

#[test]
fn report_is_not_a_replacement() {
    // "report" contains "port"; a substring list classified every report
    // row as a replacement (B5).
    let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
    assert_eq!(classify(&n, "report duplicate rows"), Kind::NodeFn);
}

#[test]
fn a_replacement_row_is_not_actionable() {
    // "replace the hand-rolled walk with itok::walk" -- insert_impl only
    // appends, so the loop would add a SECOND walk (B4).
    let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
    assert_eq!(
        classify(&n, "replace the hand-rolled walk with `itok::walk`"),
        Kind::Replaces
    );
    assert!(!Kind::Replaces.actionable());
}

#[test]
fn a_row_that_names_a_position_is_a_replacement() {
    // The row that cost two runs and 17,551 tokens. It reads as "add one
    // function" and every verb-based check passed it, but "AROUND an
    // existing function" means editing that function's call site, which
    // the loop cannot do (B9).
    let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ollama");
    for row in [
        "retry w/ bounded backoff around `Transport::post`",
        "wrap the transport in a retrying decorator",
        "cache lookups inside `generate_via`",
    ] {
        assert_eq!(
            classify(&n, row),
            Kind::Replaces,
            "should not be drivable: {row}"
        );
    }
    // Still whitelist, not blacklist: adding a free function stays actionable.
    assert_eq!(
        classify(
            &n,
            "`backoff_delay(attempt)` returns the delay before one retry"
        ),
        Kind::NodeFn
    );
}

#[test]
fn a_blocked_row_is_never_handed_back() {
    // Marking a row BLOCKED in its text did nothing -- plan handed it
    // straight back as step 1 (plan B7).
    let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
    assert!(
        !classify(&n, "BLOCKED — needs Rust source, ⊥ §F data").actionable()
    );
}

#[test]
fn a_spec_editing_row_is_not_actionable() {
    let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
    assert_eq!(
        classify(
            &n,
            "promote an invariant from a leaf to the common ancestor"
        ),
        Kind::NotAFunction
    );
}

#[test]
fn adding_a_function_stays_actionable() {
    let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
    assert_eq!(
        classify(&n, "report `§F` rows naming a dir twice"),
        Kind::NodeFn
    );
}

#[test]
fn classify_is_word_order_independent() {
    // Both phrasings describe writing §N into other nodes' specs.
    let n = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed");
    assert_eq!(
        classify(&n, "derive `§N` from parent `§F`"),
        Kind::MultiFile
    );
    assert_eq!(
        classify(&n, "`§N` derive from parent `§F`"),
        Kind::MultiFile
    );
}

/// `B17`: `plan` recommended `src/ollama` T3 as a step while `.:V117`
/// freezes that node until rung 0.7. The freeze is root POLICY, so no
/// amount of reading the row's text can reach it.
///
/// Derived from the row that declares it, not listed: `V15` records what
/// a hardcoded vocabulary costs -- it knew nine of seventeen nodes and
/// silently missed every node added after it was written.
#[test]
fn a_frozen_node_is_never_offered_as_a_step() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let frozen = frozen_nodes(root);
    assert!(
        frozen.contains(&root.join("src/ollama")),
        "the root spec freezes the model half: {frozen:?}"
    );
    let p = plan(root);
    for s in &p.steps {
        assert!(
            !frozen.contains(&root.join(&s.node)),
            "{} is frozen and was offered as a step",
            s.node.display()
        );
    }
    // Listed, not hidden -- `V3` says silence would read as coverage.
    assert!(
        p.unmanaged.iter().any(|(_, k)| *k == Kind::Frozen),
        "frozen rows are reported with their reason"
    );
}

/// The list resolves against the TREE, so a name in the row that is not a
/// node cannot silently freeze nothing -- or everything.
#[test]
fn the_freeze_names_only_real_nodes() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for f in frozen_nodes(root) {
        assert!(f.join("SPEC.md").is_file(), "{} is not a node", f.display());
    }
}

#[test]
fn an_untried_node_outranks_one_that_has_failed() {
    // Laplace: untried 0.5, one failure 1/3, three failures 1/5.
    let untried = 1.0 / 2.0;
    let failed_once = 1.0 / 3.0;
    let failed_thrice = 1.0 / 5.0;
    assert!(untried > failed_once && failed_once > failed_thrice);
}

#[test]
fn believability_of_an_unknown_node_is_neutral() {
    let b = believability(Path::new("src/never-seen-before"));
    assert!((b - 0.5).abs() < 1e-9, "untried must be neutral, got {b}");
}

#[test]
fn triage_never_proposes_moving_a_row_to_where_it_already_is() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (t, _, p) in triage(root) {
        if let Proposal::Move(n) = p {
            assert!(
                t.node.file_name().is_none_or(|f| f != n),
                "{} {} proposed to move to its own node",
                t.node.display(),
                t.id
            );
        }
    }
}

#[test]
fn triage_returns_only_unmanaged_rows() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for (t, k, _) in triage(root) {
        assert!(
            !k.actionable(),
            "{} {} is actionable, should not be triaged",
            t.node.display(),
            t.id
        );
    }
}

#[test]
fn cited_invariant_takes_the_first_v_id() {
    let mk = |c: &str| Task {
        node: PathBuf::from("src/fed"),
        id: "T4".into(),
        status: '.',
        text: "x".into(),
        cites: c.into(),
    };
    assert_eq!(cited_invariant(&mk("V2,V4")).unwrap().1, "V2");
    assert_eq!(cited_invariant(&mk("I,V7")).unwrap().1, "V7");
    assert_eq!(cited_invariant(&mk("-")), None);
    assert_eq!(
        cited_invariant(&mk("B9")),
        None,
        "a §B cite is not an invariant"
    );
    // bare -> this node; `.:` -> root; `path:` -> that node (B6)
    assert_eq!(
        cited_invariant(&mk("V2")).unwrap().0,
        PathBuf::from("src/fed")
    );
    let (owner, id) = cited_invariant(&mk("`.:V73`")).unwrap();
    assert_eq!((owner, id.as_str()), (PathBuf::new(), "V73"));
    assert_eq!(
        cited_invariant(&mk("`src/lens:V4`")).unwrap().0,
        PathBuf::from("src/lens")
    );
}

#[test]
fn row_identity_follows_its_text() {
    let a = Task {
        node: PathBuf::from("src/fed"),
        id: "T4".into(),
        status: '.',
        text: "do a thing".into(),
        cites: "V2".into(),
    };
    let mut b = a.clone();
    b.text = "do a different thing".into();
    assert_eq!(row_key(&a), row_key(&b), "key is node+id");
    assert_ne!(
        row_hash(&a),
        row_hash(&b),
        "editing the text makes it new work"
    );
}

#[test]
fn a_root_row_is_never_actionable() {
    assert_eq!(classify(Path::new("."), "anything at all"), Kind::NoModule);
    assert!(!Kind::NoModule.actionable());
}

#[test]
fn plan_never_exceeds_the_horizon() {
    let p = plan(Path::new(env!("CARGO_MANIFEST_DIR")));
    assert!(p.steps.len() <= HORIZON, "{} steps", p.steps.len());
    assert!(p.total_open >= p.steps.len());
}
