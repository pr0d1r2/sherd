//! V10: a script held in a variable and run through it is a call of the
//! value the file assigned, resolved as any other call.

use super::shell_tests::{DIR, needs, tree};
use super::*;

/// A script held in a variable the file assigned and then run through it is a
/// call of that value (V10): `"$TOOL" verb` runs `b/lib.sh`, and an
/// environment override's default is the call, runner and all. A variable that
/// holds no script, or one that is only echoed, runs nothing of a sibling's.
#[test]
fn a_script_run_through_a_variable_is_an_edge() -> Result<(), String> {
    let body = format!(
        "{DIR}TOOL=\"${{TOOL_CMD:-$DIR/../b/lib.sh}}\"\n\"$TOOL\" verb\n\
         GH=gh\n\"$GH\" api\necho \"$TOOL\"\n\
         RUN=\"${{RUN_CMD:-bash c/x.sh}}\"\n$RUN\n"
    );
    let r = tree("wave-sh-var", &[("a/run.sh", &body)])?;
    assert_eq!(needs(r.path(), "a"), vec!["b", "c"]);
    assert!(unresolved_calls(r.path(), r.path()).is_empty());
    Ok(())
}

/// The value is resolved like any call: one rooted on an argument is
/// INFERRED (V6), one built at run time is REPORTED, never dropped (V5).
#[test]
fn a_variable_holding_a_script_resolves_as_any_call() -> Result<(), String> {
    let body = "ROOT=\"$1\"\nT=\"$ROOT/c/x.sh\"\n${T} go\n\
        P=\"$(pick).sh\"\nbash \"$P\"\n";
    let r = tree("wave-sh-var-infer", &[("a/run.sh", body)])?;
    assert_eq!(needs(r.path(), "a"), vec!["c"]);
    let calls: Vec<String> = unresolved_calls(r.path(), r.path())
        .into_iter()
        .map(|u| u.call)
        .collect();
    assert_eq!(calls, vec!["$(pick).sh"]);
    Ok(())
}
