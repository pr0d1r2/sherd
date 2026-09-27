use super::*;

/// `adopt` writes into ANOTHER repository, so the dir is not optional and
/// defaulting it to the CWD is the shape `B5` records -- a verb answering
/// confidently about the wrong tree. Here it would answer by writing.
#[test]
fn adopt_without_a_dir_is_usage_rather_than_this_repo() {
    assert_eq!(run_args(vec!["adopt".into()]), ExitCode::from(2));
    assert_eq!(
        run_args(vec!["adopt".into(), "--check".into()]),
        ExitCode::from(2),
        "a flag is not a dir"
    );
}

/// A monolith with one movable rule, and a declared node to move it to.
fn adopt_fixture() -> Result<crate::testrepo::TestRepo, String> {
    let r = crate::testrepo::TestRepo::new("cli-adopt")?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nx\n\n## \u{a7}F FEDERATION\n\n\
             dir|owns|\u{22a5}owns|tokens\nsrc|parser input|the goal|-\n\n\
             ## \u{a7}V INVARIANTS\n\nV1: the parser rejects bad input\n",
    )?;
    r.write("src/SPEC.md", &spec::scaffold("src", &[]))?;
    Ok(r)
}

/// The proposal writes nothing and exits 1 while a migration is pending;
/// a tree with nothing left to move exits 0 (`src/adopt:V6`).
#[test]
fn adopt_proposes_then_reruns_clean() -> Result<(), String> {
    let r = adopt_fixture()?;
    let dir = r.path().display().to_string();
    let args = vec!["adopt".to_string(), dir];
    assert_eq!(run_args(args.clone()), ExitCode::from(1), "V1 can move");
    let map = r.path().join("map");
    std::fs::write(&map, "V1 src\n").map_err(|e| e.to_string())?;
    let mut with = args.clone();
    with.extend(["--map".to_string(), map.display().to_string()]);
    assert_eq!(run_args(with), ExitCode::from(1), "it wrote");
    assert_eq!(run_args(args), ExitCode::SUCCESS, "nothing left to move");
    Ok(())
}

/// `src/adopt:B4`. Exit 0 from `adopt` means "I read this and there is
/// nothing to move", and a source written in the bracketed table dialect
/// produced exactly that while carrying rows. The two answers now differ,
/// and the difference is in the exit code as well as the words -- a
/// wrapper reads the code.
#[test]
fn a_source_this_reader_cannot_parse_is_usage_not_nothing_to_move()
-> Result<(), String> {
    let r = adopt_fixture()?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nx\n\n## \u{a7}F FEDERATION\n\n\
             dir|owns|\u{22a5}owns|tokens\nsrc|parser input|the goal|-\n\n\
             ## \u{a7}T TASKS\n\n| id | status | task | cites |\n\
             | --- | --- | --- | --- |\n| T1 | . | first task | - |\n",
    )?;
    let args = vec!["adopt".to_string(), r.path().display().to_string()];
    assert_eq!(run_args(args.clone()), ExitCode::from(2));

    // The same refusal on the `--map` path: a map cannot be applied to a
    // file this reader did not read, and that path had its own exit code.
    let map = r.path().join("map");
    std::fs::write(&map, "T1 src\n").map_err(|e| e.to_string())?;
    let mut with = args;
    with.extend(["--map".to_string(), map.display().to_string()]);
    assert_eq!(run_args(with), ExitCode::from(2));
    Ok(())
}
