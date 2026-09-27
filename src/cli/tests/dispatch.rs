use super::fixtures::*;
use super::*;

/// The read-only verbs, driven through `run_args` against THIS repo.
///
/// Exit codes are the contract `§I` states -- `0 clean / 1 violation /
/// 2 usage` -- so asserting them is asserting the documented surface, not
/// merely executing lines. The gate runs `sherd check` and `sherd budget` on
/// every commit and requires them clean, so SUCCESS here is a claim the
/// gate independently holds true.
///
/// `cargo test` runs with the package root as CWD, so `repo_root` finds
/// the real tree. None of these verbs writes: `slice` is given
/// `--check`, and `outcome`, `apply`, `tdd` and `ask` are excluded --
/// they write state or need an endpoint.
#[test]
fn the_read_only_verbs_all_exit_clean_on_this_repo() {
    for verb in [
        vec!["check"],
        vec!["budget"],
        vec!["fed"],
        vec!["plan"],
        vec!["plan", "--triage"],
        vec!["slice", "--check"],
        vec!["seam"],
        vec!["wave"],
    ] {
        assert_eq!(
            run_args(argv(&verb)),
            ExitCode::SUCCESS,
            "`sherd {}` must exit 0 on a clean tree",
            verb.join(" ")
        );
    }
}

/// Every `graph` rendering, including the default.
///
/// B15 is why there are four: a mermaid diagram that no renderer draws is
/// a diagram nobody reads, so `--tree` renders in any markdown forever.
/// A flag whose branches agree is a claim with no runner (`.:V105`), so
/// the renderings must also DIFFER.
#[test]
fn every_graph_rendering_succeeds_and_they_are_not_the_same_render() {
    for flag in [
        vec!["graph"],
        vec!["graph", "--dot"],
        vec!["graph", "--table"],
        vec!["graph", "--tree"],
    ] {
        assert_eq!(run_args(argv(&flag)), ExitCode::SUCCESS, "{flag:?}");
    }
    let root = repo_root();
    let (dot, table) = (fed::dot(&root), fed::table(&root));
    let (tree, mermaid) = (fed::tree(&root), fed::mermaid(&root));
    assert!(dot.contains("digraph"), "--dot must emit dot");
    assert!(table.contains('|'), "--table must emit a markdown table");
    assert_ne!(dot, mermaid, "a flag whose branches agree is no flag");
    assert_ne!(tree, mermaid);
    assert_ne!(table, tree);
}

/// A verb pointed at a directory that is not a node.
///
/// USAGE (2), not violation (1): you named the wrong directory, which is
/// a bad argument -- distinct from `check` finding a real defect inside a
/// node that does exist. `§I` separates the two codes and something has
/// to hold them apart.
#[test]
fn a_dir_with_no_spec_is_a_usage_error_not_a_violation() {
    assert_eq!(
        run_args(argv(&["fed", "target"])),
        ExitCode::from(2),
        "no SPEC.md there is a bad ARGUMENT, never a silent zero"
    );
    assert_eq!(
        run_args(argv(&["check"])),
        ExitCode::SUCCESS,
        "and a real node still checks clean -- the codes differ"
    );
}

#[test]
fn an_unknown_command_is_a_usage_error() {
    assert_eq!(run_args(argv(&["nope"])), ExitCode::from(2));
}

#[test]
fn help_and_no_args_both_succeed() {
    assert_eq!(run_args(argv(&["--help"])), ExitCode::SUCCESS);
    assert_eq!(run_args(argv(&["-h"])), ExitCode::SUCCESS);
    assert_eq!(run_args(argv(&["help"])), ExitCode::SUCCESS);
    assert_eq!(run_args(argv(&[])), ExitCode::SUCCESS);
}

#[test]
fn a_verb_missing_its_argument_is_usage_not_a_crash() {
    // Each of these needs an argument it is not given. Usage, never a
    // panic: `sherd` runs unattended inside the loop, and a panic there is
    // a run that stops with no record.
    assert_eq!(run_args(argv(&["lens"])), ExitCode::from(2));
    assert_eq!(run_args(argv(&["outcome"])), ExitCode::from(2));
    assert_eq!(run_args(argv(&["outcome", "src/fed"])), ExitCode::from(2));
}

#[test]
fn an_outcome_verdict_outside_the_three_words_is_refused() {
    // `kept`, `reverted`, `failed`. Anything else must not be read as one
    // of them -- believability is computed from these and a typo silently
    // scored as `kept` would corrupt the record it exists to keep.
    assert_eq!(
        run_args(argv(&["outcome", "src/fed", "probably"])),
        ExitCode::from(2)
    );
}

/// The read-only verbs, run against THIS repo.
///
/// A fixture would be a second repo to keep honest; the gate already
/// requires these green here, so running them on the real tree asserts
/// the same thing the gate does and covers the dispatch that reaches
/// them. `.:V27` -- this repo must be a valid federation -- is exactly
/// the claim being exercised.
const VERBS: [&[&str]; 10] = [
    &["graph"],
    &["graph", "--dot"],
    &["graph", "--table"],
    &["graph", "--tree"],
    &["fed"],
    &["check"],
    &["budget"],
    &["slice", "--list"],
    &["seam"],
    &["wave"],
];

#[test]
fn the_read_only_verbs_succeed_on_this_repo() {
    for a in VERBS {
        let code = run_args(argv(a));
        assert_eq!(
            code,
            ExitCode::SUCCESS,
            "`sherd {}` must exit 0",
            a.join(" ")
        );
    }
}

#[test]
fn a_dir_matching_no_node_is_usage_not_a_vacuous_pass() {
    // Examining NOTHING is not passing. An empty table and a clean repo
    // were indistinguishable until T10, which is `src/tdd:V26`'s shape.
    assert_eq!(
        run_args(argv(&["budget", "no-such-dir"])),
        ExitCode::from(2)
    );
}

/// V16. Both spellings answer, and they answer with the version this
/// binary was BUILT from -- `B10` is the state where neither did, and a
/// CI gate recording tool versions had to parse the usage banner.
#[test]
fn both_version_spellings_exit_clean_and_name_the_crate_version() {
    assert_eq!(run_args(argv(&["--version"])), ExitCode::SUCCESS);
    assert_eq!(run_args(argv(&["-V"])), ExitCode::SUCCESS);

    // Read from the manifest rather than restated here: a literal in
    // this assertion is the second spelling `VERSION` exists to avoid,
    // and it would need editing at every release.
    let manifest =
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    let declared = manifest
        .lines()
        .find_map(|l| l.strip_prefix("version = \""))
        .and_then(|v| v.split_once('"').map(|(v, _)| v));
    assert_eq!(declared, Some(VERSION), "{VERSION}");

    let parts: Vec<&str> = VERSION.split('.').collect();
    assert_eq!(parts.len(), 3, "semver, three components: {VERSION}");
}

#[test]
fn the_usage_text_names_every_exit_code_it_returns() {
    // The three codes the tests above assert are the three §I documents.
    assert!(USAGE.contains("0 clean"), "{USAGE}");
    assert!(USAGE.contains("1 violation"), "{USAGE}");
    assert!(USAGE.contains("2 usage"), "{USAGE}");
}
