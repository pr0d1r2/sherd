use super::*;

const USAGE: &str = "\
sherd -- a tool

  sherd budget [dir]     token cost of every node
  sherd lens <dir> [--depth rule|why|all]  the context pack for one node

  -v, --verbose        dump every prompt

exit: 0 clean · 1 violation · 2 usage";

const DISPATCH: &str = r#"
    let x = match depth {
        Some("rule") => Depth::Rule,
    };
    match args.first().map(String::as_str) {
        Some("budget") => budget(&root, arg_dir(&args, &root)),
        Some("lens") => match (args.get(1), depth_arg(&args)) {
        Some("oneshot") => match (args.get(1), args.get(2)) {
            Some("--nested") => nested(),
        },
        Some("help" | "--help" | "-h") => { print!("{USAGE}"); }
    }
    match graph_flag {
        Some("--dot") => dot(),
    }
"#;

#[test]
fn a_usage_line_yields_its_verb_and_a_flag_line_does_not() {
    let got: Vec<String> =
        usage_lines(USAGE).into_iter().map(|(v, _)| v).collect();
    assert_eq!(got, vec!["budget", "lens"]);
}

/// SCOPED: the arms before and after the dispatch match are argument
/// matches spelled identically, and the unscoped version reported
/// `rule` and `--dot` as undocumented verbs.
#[test]
fn dispatch_verbs_come_from_the_dispatch_match_only() {
    let got = dispatched(DISPATCH);
    assert!(got.contains(&"budget".to_string()));
    assert!(got.contains(&"lens".to_string()));
    assert!(got.contains(&"oneshot".to_string()));
    assert!(!got.contains(&"rule".to_string()), "an argument arm before");
    assert!(!got.contains(&"--dot".to_string()), "and one after");
    assert!(
        !got.contains(&"--nested".to_string()),
        "and one nested INSIDE a dispatch arm, which scoping to the \
             match alone did not exclude"
    );
}

/// B2 exactly: a verb that dispatches and is in no usage line. This is
/// the assertion that would have caught it the day it landed.
#[test]
fn a_verb_that_dispatches_and_is_not_documented_is_reported() {
    assert_eq!(undocumented(USAGE, DISPATCH), vec!["oneshot"]);
}

#[test]
fn help_is_not_reported_as_undocumented() {
    let dispatch = "        Some(\"help\" | \"--help\" | \"-h\") => {}\n";
    assert!(undocumented(USAGE, dispatch).is_empty());
}

/// `src/cli:B10`'s residue: the fix for it adds a dispatch arm, and both
/// contracts over dispatch would otherwise demand a `--version` row in a
/// table of VERBS -- the README's Commands section and `§I`.
///
/// The fixture carries the `match args.first()` header `dispatched`
/// scopes to, and the assertion below proves the arm was SEEN. Without
/// it both checks pass over an empty verb list, which is the vacuous
/// green this repo keeps finding (`.:B17`).
#[test]
fn version_is_a_flag_in_neither_the_commands_table_nor_the_interface() {
    let dispatch = "\
    match args.first().map(String::as_str) {
        Some(\"budget\") => budget(),
        Some(\"-V\" | \"--version\") => version(),
    }
";
    assert_eq!(dispatched(dispatch), vec!["budget", "-V"]);
    assert!(undocumented(USAGE, dispatch).is_empty());
    assert!(
        interface_drift(
            "## §I INTERFACES\n\n- cmd: `sherd budget [dir]` → a table\n",
            dispatch
        )
        .is_empty()
    );
}

#[test]
fn every_documented_verb_reaches_the_table() {
    let table = render(USAGE);
    assert!(table.starts_with("| command | what it does |\n|---|---|\n"));
    assert!(
        table.contains("| `sherd budget [dir]` | token cost of every node |")
    );
    assert!(
        table.contains(
            "| `sherd lens <dir> [--depth rule\\|why\\|all]` | the context pack for one node |"
        ),
        "a pipe inside a cell is escaped, or the table renders broken: {table}"
    );
    assert!(!table.contains("--verbose"));
}
