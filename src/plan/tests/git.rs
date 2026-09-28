use super::*;
use crate::testrepo::TestRepo;

#[cfg(feature = "ollama")]
#[test]
fn preflight_refuses_a_tree_that_is_not_a_repo() {
    // `apply` COMMITS, so it needs a repo. Saying so beats failing later
    // with a git error nobody reads.
    let d = std::env::temp_dir().join("sherd-not-a-repo");
    let _ = std::fs::create_dir_all(&d);
    let r = preflight(&d);
    let _ = std::fs::remove_dir_all(&d);
    assert!(r.is_err(), "a non-repo must be refused");
}

#[cfg(feature = "ollama")]
#[test]
fn preflight_refuses_a_dirty_tree() {
    assert_eq!(check_dirty(), Ok(()));
}

/// A dirty tree means the generated diff would not be the only thing in
/// the commit, which is the whole point of the branch `apply` makes.
#[cfg(feature = "ollama")]
fn check_dirty() -> Result<(), String> {
    let r = TestRepo::new("plan-dirty")?;
    r.write("stray.txt", "uncommitted\n")?;
    let out = preflight(r.path());
    let Err(msg) = out else {
        return Err("a dirty tree must be refused".into());
    };
    assert!(msg.contains("dirty"), "the refusal must say why: {msg}");
    Ok(())
}

/// A real node, so `classify` gets past its `mod.rs` guard.
fn node() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/fed")
}

const MS_ROOT: &str = "# SPEC\n\n## \u{a7}G GOAL\n\ntoy\n\n\
## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\na|alpha|beta|-\nb|beta|alpha|-\n";

/// `a` declares milestones, `b` declares none; `T2` is done and `T2a`
/// rides it, so M1 claims `T2a` through `T1-T2`.
const MS_A: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nalpha\n\n## \u{a7}T TASKS\n\n\
| id | scope | tasks | done-when |\n|----|-------|-------|-----------|\n\
| M1 | first | T1-T2 | shipped |\n| M2 | later | T3 | shipped |\n\n\
id|status|task|cites\nT1|.|one|-\nT2|x|two|-\nT2a|.|two more|-\nT3|.|three|-\n";

const MS_B: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nbeta\n\n## \u{a7}T TASKS\n\n\
id|status|task|cites\nT1|.|unscheduled|-\n";

/// V24. A milestone keeps exactly the rows its OWN node's table claims,
/// and the open rows of a node with no milestones are COUNTED, not
/// silently dropped -- an empty horizon must never read as "all done".
#[test]
fn a_milestone_keeps_its_own_rows_and_counts_the_unscheduled()
-> Result<(), String> {
    let r = TestRepo::new("plan-milestone")?;
    r.write("SPEC.md", MS_ROOT)?;
    r.write("a/SPEC.md", MS_A)?;
    r.write("b/SPEC.md", MS_B)?;
    let (kept, outside) = in_milestone(r.path(), open_tasks(r.path()), "M1");
    let ids: Vec<String> = kept
        .iter()
        .map(|t| format!("{}:{}", t.node.display(), t.id))
        .collect();
    assert_eq!(ids, vec!["a:T1", "a:T2a"]);
    assert_eq!(outside, 1, "b:T1 is counted, not dropped");
    assert!(milestone_declared(r.path(), "M2"));
    assert!(!milestone_declared(r.path(), "M9"), "a typo is not a plan");
    Ok(())
}

/// A suffixed id rides its base, as `microlith/V14` orders it.
#[test]
fn a_task_number_ignores_its_suffix() {
    assert_eq!(task_number("T7"), Some(7));
    assert_eq!(task_number("T7a"), Some(7));
    assert_eq!(task_number("V7"), None);
    assert_eq!(task_number("T"), None);
}

#[test]
fn a_dir_with_no_module_is_never_actionable() {
    // The loop edits ONE node's `mod.rs`. A row whose node has none has
    // nowhere for the code to go, whatever the row says.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(classify(root, "add one pure function"), Kind::NoModule);
}

/// Rows the loop CANNOT do, each with the bug that put it there.
///
/// `classify` is five recorded defects deep -- B1, B4, B5, B8, B9 -- and
/// every one was a row read as ACTIONABLE that the loop then could not
/// perform. A misclassification does not fail loudly: it sends the 20B at
/// work it cannot do, which is `src/tdd:T13`'s zero.
#[test]
fn a_row_that_edits_existing_code_is_never_actionable() {
    let n = node();
    // B4: a BLACKLIST marked "replace the hand-rolled walk" actionable,
    // and the loop only appends.
    assert_eq!(classify(&n, "replace the hand-rolled walk"), Kind::Replaces);
    // B9: a POSITION word names where code goes RELATIVE to existing
    // code, so it needs an edited call site however it is phrased.
    assert_eq!(
        classify(&n, "retry around `Transport::post`"),
        Kind::Replaces
    );
    assert_eq!(classify(&n, "wrap the existing judge"), Kind::Replaces);
}

#[test]
fn a_row_that_is_not_a_function_at_all_is_named_as_such() {
    let n = node();
    assert_eq!(
        classify(&n, "blocked -- needs Rust source"),
        Kind::NotAFunction
    );
    assert_eq!(classify(&n, "promote the node"), Kind::NotAFunction);
    assert_eq!(classify(&n, "add a `sherd foo` verb"), Kind::Cli);
    assert_eq!(classify(&n, "audit the corpus upstream"), Kind::NotCode);
}

#[test]
fn a_multi_file_row_is_caught_whatever_the_word_order() {
    // B1: the first version looked for "derive `§n`" and the row said
    // "`§N` derive from parent `§F`". Widening a substring list is a
    // patch; word-order independence is the fix.
    let n = node();
    let k = Kind::MultiFile;
    assert_eq!(classify(&n, "`\u{a7}N` derive from parent `\u{a7}F`"), k);
    assert_eq!(classify(&n, "derive `\u{a7}N` from the parent table"), k);
}

#[test]
fn the_whitelist_still_says_yes_to_one_added_function() {
    // The classifier must not become a machine that refuses everything:
    // a detector tested only on the negative case is satisfied by
    // returning the negative (`src/fed:V10`).
    assert_eq!(
        classify(&node(), "add a pure function that counts cells"),
        Kind::NodeFn
    );
}

/// Every stem, in its `-ing` form, with the class it must land in.
const ING: &[(&str, Kind)] = &[
    ("wiring the detector into check", Kind::NotAFunction),
    ("moving the corpus to a fixture", Kind::NotAFunction),
    ("promoting the node to a sibling", Kind::NotAFunction),
    ("recording the outcome", Kind::NotAFunction),
    ("replacing the hand-rolled walk", Kind::Replaces),
    ("removing the stub", Kind::Replaces),
    ("migrating to itok::walk", Kind::Replaces),
    ("rewriting the judge", Kind::Replaces),
    ("deleting the dead arm", Kind::Replaces),
    ("superseding the old row", Kind::Replaces),
];

#[test]
fn every_stem_matches_its_own_ing_form() {
    // B11, and `V14`: B8 recorded "missed `wiring` (list had `wire`)"
    // and shipped a stem match that STILL did not match `wiring`, so the
    // row read as closed while its own example still failed. Nine of the
    // eighteen stems were affected -- every one ending in `e`.
    let n = node();
    for (row, want) in ING {
        assert_eq!(
            classify(&n, row),
            *want,
            "`{row}` must not read as actionable -- the loop cannot do it"
        );
    }
}

#[test]
fn report_is_not_a_replacement_even_though_it_contains_port() {
    // B5 exactly: a SUBSTRING list classified every `report ...` row as a
    // replacement, because "report" contains "port". The fix was to match
    // WORDS, and this is the assertion that holds it.
    assert_eq!(classify(&node(), "report the drift"), Kind::NodeFn);
}

/// A `§T` row citing `cites`, at `src/plan`.
fn cite_row(cites: &str) -> Task {
    Task {
        node: std::path::PathBuf::from("src/plan"),
        id: "T1".into(),
        text: String::new(),
        cites: cites.into(),
        status: '.',
    }
}

/// A store of its own, so this test races nothing.
fn store(tag: &str) -> crate::state::State {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    crate::state::State::at(
        std::env::temp_dir()
            .join(format!("sherd-score-{tag}-{}-{n}", std::process::id())),
    )
}

#[test]
fn an_untried_node_outranks_one_that_has_failed() {
    // Laplace: `(kept + 1) / (tried + 2)`. An untried node scores 0.5, so
    // it goes ahead of a node that failed three times WITHOUT pretending
    // to know it is good. A raw ratio would score the untried node 0/0
    // and the failed one 0.00, making them indistinguishable.
    let mut st = store("laplace");
    let untried = Path::new("src/never-tried");
    let failed = Path::new("src/failed");
    for _ in 0..3 {
        record_outcome_in(&mut st, failed, false);
    }
    let u = believability_in(&st, untried);
    let f = believability_in(&st, failed);
    assert!(
        (u - 0.5).abs() < 1e-9,
        "an untried node sits at 0.5, got {u}"
    );
    assert!(f < u, "three failures rank BELOW untried: {f} vs {u}");
    assert_eq!(record_in(&st, failed), (3, 0), "tried counted, kept not");
}

#[test]
fn five_consecutive_keeps_clears_the_landing_bar() {
    // `src/land:V2` puts LAND_MIN at 0.85 and calls it "5 consecutive
    // keeps under Laplace". That is an arithmetic claim in prose, and
    // nothing checked it: 6/7 = 0.857, so five is the number and four
    // (5/6 = 0.833) is not.
    let mut st = store("bar");
    let n = Path::new("src/proven");
    for _ in 0..4 {
        record_outcome_in(&mut st, n, true);
    }
    assert!(
        believability_in(&st, n) < 0.85,
        "four keeps is 5/6 = 0.833, below the bar"
    );
    record_outcome_in(&mut st, n, true);
    assert!(
        believability_in(&st, n) >= 0.85,
        "five keeps is 6/7 = 0.857, and V2 says that clears it"
    );
}

#[test]
fn a_kept_outcome_counts_in_both_tallies_and_a_reverted_one_in_neither() {
    // `kept` counts what survived REVIEW, not what passed the gate -- the
    // gate has gone green on three stubs, so counting commits would
    // measure the wrong thing.
    let mut st = store("tally");
    let n = Path::new("src/mixed");
    record_outcome_in(&mut st, n, true);
    record_outcome_in(&mut st, n, false);
    assert_eq!(record_in(&st, n), (2, 1), "two tries, one kept");
}

#[test]
fn a_bare_cite_belongs_to_the_row_s_own_node() {
    // Ids are node-scoped (`.:V10`): a bare `V9` and a namespaced
    // `src/fed:V9` must not resolve to the same file.
    assert_eq!(
        cited_invariant(&cite_row("V9")),
        Some((std::path::PathBuf::from("src/plan"), "V9".into()))
    );
}

#[test]
fn a_namespaced_cite_names_its_owner_and_dot_is_root() {
    assert_eq!(
        cited_invariant(&cite_row("`src/fed:V9`")),
        Some((std::path::PathBuf::from("src/fed"), "V9".into()))
    );
    assert_eq!(
        cited_invariant(&cite_row("`.:V73`")),
        Some((std::path::PathBuf::new(), "V73".into())),
        "`.` is the root node"
    );
}

#[test]
fn a_row_citing_no_invariant_yields_none() {
    // Not every row cites a §V, and inventing one would send the loop at
    // an invariant nobody wrote.
    assert_eq!(cited_invariant(&cite_row("R44,I")), None);
    assert_eq!(cited_invariant(&cite_row("Vx,V1a")), None, "not an id");
}

#[test]
fn a_row_naming_two_nodes_is_decomposed_not_moved() {
    // A row that names one node's vocabulary can MOVE; one that names
    // two is work for two nodes and moving it would just relocate the
    // ambiguity.
    assert!(matches!(propose("count the tokens"), Proposal::Move(_)));
    assert!(matches!(
        propose("count the tokens and render the lens pack"),
        Proposal::Decompose(v) if v.len() >= 2
    ));
    assert!(matches!(propose("think about it"), Proposal::Keep));
}

#[test]
fn open_tasks_reads_this_repo_and_skips_what_is_done() {
    // The federation's own §T rows. `x` is history, and a machine told to
    // test what already passes learns nothing (`src/fed:V9`).
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let ts = open_tasks(root);
    assert!(!ts.is_empty(), "this repo has open rows");
    assert!(
        ts.iter().all(|t| t.status != 'x'),
        "a done row is not an open task"
    );
    assert!(
        ts.iter().any(|t| t.node.ends_with("src/fed")),
        "rows are collected across nodes, not just root"
    );
}

#[test]
fn a_plan_ranks_what_it_can_act_on_and_counts_what_it_cannot() {
    // R46: 3 actionable of 78. The UNMANAGED list is the honest half --
    // a horizon of 3 that hid 75 rows would read as a nearly finished
    // project, and `.:B4` is exactly a ratio whose denominator lied.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = plan(root);
    assert!(p.total_open > 0, "this repo has open rows");
    assert!(
        p.steps.len() <= p.total_open,
        "the horizon cannot exceed the rows it came from"
    );
    assert!(
        !p.unmanaged.is_empty(),
        "R46: most rows are unmanaged, and they must be REPORTED"
    );
    assert!(
        p.steps.len() + p.unmanaged.len() <= p.total_open,
        "no row may be counted in both halves"
    );
}

#[cfg(feature = "ollama")]
#[test]
fn preflight_on_a_clean_repo_names_a_run_branch() {
    assert_eq!(check_clean(), Ok(()));
}

/// Generated code never lands on the trunk directly: `preflight` puts the
/// run on its own branch, and `sherd land` is what moves it, on evidence.
#[cfg(feature = "ollama")]
fn check_clean() -> Result<(), String> {
    let r = TestRepo::new("plan-clean")?;
    let branch = preflight(r.path())?;
    assert!(
        branch.starts_with("sherd/"),
        "a run gets its own branch, got {branch}"
    );
    let now = r.git(&["rev-parse", "--abbrev-ref", "HEAD"])?;
    assert_eq!(now, branch, "preflight must have switched to it");
    Ok(())
}
