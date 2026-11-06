use super::super::fixtures::*;
use super::*;

fn side(chains: &[(&str, u64, u64)], files: &[(&str, u64)]) -> Side {
    Side {
        chains: chains
            .iter()
            .map(|(n, t, c)| ((*n).to_string(), (*t, *c)))
            .collect(),
        files: files.iter().map(|(n, l)| ((*n).to_string(), *l)).collect(),
    }
}

/// The fat root split in two: rows added, file rows raised, every chain down.
#[test]
fn a_split_that_shrinks_every_chain_passes_with_rows_added() {
    let base = side(&[(".", 6000, 9000), ("a", 7000, 9400)], &[("a", 500)]);
    let head = side(
        &[(".", 2700, 4000), ("a", 5000, 6000), ("b", 3000, 5000)],
        &[("a", 700), ("b", 600)],
    );
    assert_eq!(judge(&base, &head), Vec::<String>::new());
}

#[test]
fn a_plain_chain_ceiling_raise_fails_naming_the_row() {
    let base = side(&[(".", 100, 9000), ("a", 100, 9000)], &[]);
    let head = side(&[(".", 100, 9000), ("a", 100, 9500)], &[]);
    let got = judge(&base, &head);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].starts_with("a: chain ceiling raised 9000 -> 9500"));
}

#[test]
fn a_file_raise_without_a_falling_chain_fails_naming_both_rows() {
    let base = side(&[(".", 100, 9000), ("a", 100, 9000)], &[("a", 500)]);
    let head = side(&[(".", 100, 9000), ("a", 100, 9000)], &[("a", 700)]);
    let got = judge(&base, &head);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].contains("a/SPEC.md") && got[0].contains("row `a`"));
}

#[test]
fn a_new_node_larger_than_every_base_chain_fails() {
    let base = side(&[(".", 100, 9000)], &[]);
    let head = side(&[(".", 100, 9000), ("n", 100, 9001)], &[]);
    let got = judge(&base, &head);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].starts_with("n: new node"));
}

#[test]
fn a_chain_over_its_ceiling_fails_even_when_nothing_rose() {
    let base = side(&[(".", 100, 9000)], &[]);
    let head = side(&[(".", 9100, 9000)], &[]);
    let got = judge(&base, &head);
    assert_eq!(got.len(), 1, "{got:?}");
    assert!(got[0].contains("OVER") && got[0].contains("by 100"));
}

fn fixture(tag: &str) -> Result<crate::testrepo::TestRepo, String> {
    let r = crate::testrepo::TestRepo::new(tag)?;
    r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nroot goal\n")?;
    write_spec(r.path(), "a", "alpha goal");
    r.write(".context-limits", "SPEC.md 5000\na 5000\n")?;
    r.commit("fat root and one node")?;
    Ok(r)
}

/// End to end through git: the base is read from a REVISION, not the tree.
#[test]
fn the_cli_passes_a_split_and_refuses_a_plain_raise() -> Result<(), String> {
    let r = fixture("cli-budget-base")?;
    let root = r.path().to_path_buf();
    write_spec(&root, "b", "beta goal");
    r.write(
        ".context-limits",
        "SPEC.md 4000\na 4000\na/SPEC.md 700\nb 4000\nb/SPEC.md 600\n",
    )?;
    assert_eq!(
        budget_base(&root, &root, "HEAD", true),
        ExitCode::SUCCESS,
        "a split that lowers every chain ceiling passes with rows added"
    );
    r.write(".context-limits", "SPEC.md 5000\na 6000\n")?;
    assert_eq!(
        budget_base(&root, &root, "HEAD", true),
        ExitCode::from(1),
        "a plain raise fails"
    );
    assert_eq!(
        budget_base(&root, &root, "no-such-rev", true),
        ExitCode::from(2),
        "an unreadable base is an error, not a pass"
    );
    Ok(())
}

/// A base revision predating the first SPEC.md measured nothing: it is an
/// error naming the revision, never a base every head node is judged against.
#[test]
fn a_base_with_no_node_is_an_error_not_a_vacuous_judgement()
-> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-budget-base-empty")?;
    // `new` commits a root SPEC.md; the base revision is one with none at all.
    r.git(&["rm", "-q", "SPEC.md"])?;
    r.commit("no spec yet")?;
    let root = r.path().to_path_buf();
    r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nroot goal\n")?;
    r.write(".context-limits", "SPEC.md 5000\n")?;
    assert_eq!(
        budget_base(&root, &root, "HEAD", true),
        ExitCode::from(2),
        "a base with no node is refused, not judged against a ceiling of 0"
    );
    Ok(())
}

/// Without `--check` the table prints and the verb never judges.
#[test]
fn without_check_it_reports_both_sides_and_exits_zero() -> Result<(), String> {
    let r = fixture("cli-budget-base-report")?;
    let root = r.path().to_path_buf();
    r.write(".context-limits", "SPEC.md 5000\na 9999\n")?;
    assert_eq!(budget_base(&root, &root, "HEAD", false), ExitCode::SUCCESS);
    assert_eq!(budget_base(&root, &root, "HEAD", true), ExitCode::from(1));
    Ok(())
}

#[test]
fn the_flags_parse_on_the_budget_verb() {
    use clap::Parser;
    let parsed = parse::Cli::try_parse_from([
        "sherd", "budget", "--base", "main", "--check",
    ]);
    assert!(matches!(
        parsed,
        Ok(parse::Cli {
            cmd: Some(Cmd::Budget {
                base: Some(_),
                check: true,
                ..
            }),
            ..
        })
    ));
}
