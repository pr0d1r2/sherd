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
