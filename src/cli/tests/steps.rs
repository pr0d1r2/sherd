use super::*;

/// `src/plan:V24`: a milestone no node declares is a usage error, never an
/// empty horizon that reads as "nothing left to do".
#[test]
fn plan_for_an_undeclared_milestone_is_usage() {
    let m = |a: &[&str]| run_args(a.iter().map(|s| (*s).to_string()).collect());
    assert_eq!(m(&["plan", "--milestone", "M999"]), ExitCode::from(2));
    assert_eq!(m(&["plan", "--milestone"]), ExitCode::from(2));
}

/// `src/plan:V25`: `--format` is a mode, found wherever it sits, and a
/// value it does ⊥ know is usage -- ⊥ a silent fall back to the text form
/// a JSON caller then fails to parse.
#[test]
fn plan_format_is_positional_agnostic_and_strict() {
    let s = |a: &[&str]| a.iter().map(|x| (*x).to_string()).collect();
    let v: Vec<String> = s(&["--milestone", "M1", "--format", "json"]);
    assert_eq!(take_format(&v), Ok((true, s(&["--milestone", "M1"]))));
    let v: Vec<String> = s(&["--format", "json", "--milestone", "M1"]);
    assert_eq!(take_format(&v), Ok((true, s(&["--milestone", "M1"]))));
    let v: Vec<String> = s(&["--format", "text"]);
    assert_eq!(take_format(&v), Ok((false, vec![])));
    assert_eq!(take_format(&[]), Ok((false, vec![])));
    let m = |a: &[&str]| run_args(a.iter().map(|x| (*x).to_string()).collect());
    assert_eq!(m(&["plan", "--format", "yaml"]), ExitCode::from(2));
    assert_eq!(m(&["plan", "--format"]), ExitCode::from(2));
    assert_eq!(
        m(&["plan", "--triage", "--format", "json"]),
        ExitCode::from(2)
    );
}

/// Two nodes: `a` declares milestone M1 over its T1, `b` declares none and
/// has an open row of its own.
fn milestone_fixture() -> Result<crate::testrepo::TestRepo, String> {
    let r = crate::testrepo::TestRepo::new("cli-plan-ms")?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\ntoy\n\n## \u{a7}F FEDERATION\n\n\
         dir|owns|\u{22a5}owns|tokens\na|alpha|beta|-\nb|beta|alpha|-\n",
    )?;
    r.write(
        "a/SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nalpha\n\n## \u{a7}T TASKS\n\n\
         | id | scope | tasks | done-when |\n|----|-------|-------|-----------|\n\
         | M1 | first | T1 | shipped |\n\nid|status|task|cites\nT1|.|one|-\n",
    )?;
    r.write(
        "b/SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nbeta\n\n## \u{a7}T TASKS\n\n\
         id|status|task|cites\nT1|.|unscheduled|-\n",
    )?;
    Ok(r)
}

/// A milestone plan with rows outside it renders in text and in JSON, and
/// both exit 0. The COUNT of rows set aside is asserted where it is
/// computed, `src/plan`'s `a_milestone_keeps_its_own_rows_...` (V24);
/// this holds the verb's two output paths.
#[test]
fn a_milestone_plan_renders_in_text_and_json() -> Result<(), String> {
    let r = milestone_fixture()?;
    assert_eq!(plan_cmd(r.path(), Some("M1"), false), ExitCode::SUCCESS);
    assert_eq!(plan_cmd(r.path(), Some("M1"), true), ExitCode::SUCCESS);
    assert_eq!(plan_cmd(r.path(), None, true), ExitCode::SUCCESS);
    Ok(())
}

/// `src/cli:B12`: `plan` READS the store and writes nothing, in either form.
/// The store already holds a `plan` key, so a plan that cleared or
/// rewrote it would change these bytes even if the new keys matched.
#[test]
fn plan_leaves_the_state_store_untouched() -> Result<(), String> {
    let r = milestone_fixture()?;
    let store = r.path().join("store");
    let before =
        "# a store written by someone else\nplan 1 a T9 STALE\napplied x 1\n";
    std::fs::write(&store, before).map_err(|e| e.to_string())?;
    let st = crate::state::State::at(&store);
    assert_eq!(plan_with(r.path(), None, false, &st), ExitCode::SUCCESS);
    assert_eq!(
        plan_with(r.path(), Some("M1"), true, &st),
        ExitCode::SUCCESS
    );
    let after = std::fs::read_to_string(&store).map_err(|e| e.to_string())?;
    assert_eq!(after, before);
    Ok(())
}
