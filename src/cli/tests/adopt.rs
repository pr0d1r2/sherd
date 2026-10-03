use super::super::fixtures::argv;
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

/// `src/adopt:B6`. The dir argument is the SOURCE: `adopt <root>/src` reads
/// `src/SPEC.md` and moves its row into `src/deep`. Dispatch used to hand
/// on the root, and the same map was refused as "the source declares no".
#[test]
fn adopt_reads_the_spec_of_the_dir_it_names() -> Result<(), String> {
    let r = adopt_fixture()?;
    r.write(
        "src/SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nsrc\n\n## \u{a7}F FEDERATION\n\n\
             dir|owns|\u{22a5}owns|tokens\ndeep|deep things|-|-\n\n\
             ## \u{a7}V INVARIANTS\n\nV4: deep things stay deep\n",
    )?;
    r.write("src/deep/SPEC.md", &spec::scaffold("src/deep", &[]))?;
    let map = r.path().join("map");
    std::fs::write(&map, "V4 src/deep\n").map_err(|e| e.to_string())?;
    let dir = r.path().join("src").display().to_string();
    let with = vec![
        "adopt".into(),
        dir,
        "--map".into(),
        map.display().to_string(),
    ];
    assert_eq!(run_args(with), ExitCode::from(1), "it wrote");
    let deep = std::fs::read_to_string(r.path().join("src/deep/SPEC.md"))
        .map_err(|e| e.to_string())?;
    assert!(deep.contains("V4: deep things stay deep"), "{deep}");
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

/// `src/cli:V1`: 1 is a VERDICT -- for `adopt`, a migration pending or
/// written. A verb that could not read its INPUT has no verdict to give, so
/// that is 2, as `coverage` and `debt` already answer (`src/cli:B11`). A
/// missing map exited 1, which a wrapper reads as "migration pending".
#[test]
fn input_adopt_cannot_read_is_usage_not_a_pending_migration()
-> Result<(), String> {
    let r = adopt_fixture()?;
    let dir = r.path().display().to_string();
    let with_map = |map: &str| {
        vec![
            "adopt".to_string(),
            dir.clone(),
            "--map".to_string(),
            map.to_string(),
        ]
    };
    let missing = r.path().join("no-such-map").display().to_string();
    assert_eq!(run_args(with_map(&missing)), ExitCode::from(2), "no map");

    let bad = r.path().join("bad-map");
    std::fs::write(&bad, "one-field-only\n").map_err(|e| e.to_string())?;
    let bad = bad.display().to_string();
    assert_eq!(run_args(with_map(&bad)), ExitCode::from(2), "bad map");

    // A dir with no SPEC.md and no repository above it: nothing to adopt.
    // Outside the fixture, or the root walk finds the fixture's own spec.
    let bare = std::env::temp_dir()
        .join(format!("sherd-adopt-bare-{}", std::process::id()));
    std::fs::create_dir_all(&bare).map_err(|e| e.to_string())?;
    let code = run_args(vec!["adopt".to_string(), bare.display().to_string()]);
    let _ = std::fs::remove_dir_all(&bare);
    assert_eq!(code, ExitCode::from(2), "no source");

    // A map the verb READ and refuses is still a verdict: 1.
    let refused = r.path().join("refused-map");
    std::fs::write(&refused, "V1 no-such-node\n").map_err(|e| e.to_string())?;
    let refused = refused.display().to_string();
    assert_eq!(run_args(with_map(&refused)), ExitCode::from(1), "refused");
    Ok(())
}

/// `--check --map`: every refusal and NOTHING written. A map that would
/// apply cleanly exits 0 and leaves the source as it was; a refused one
/// exits 1 (`src/adopt:V2`).
#[test]
fn a_dry_run_reports_and_writes_nothing() -> Result<(), String> {
    let r = adopt_fixture()?;
    let before = std::fs::read_to_string(r.path().join("SPEC.md"))
        .map_err(|e| e.to_string())?;
    let dry = |map: &str| -> Result<ExitCode, String> {
        let p = r.path().join("map");
        std::fs::write(&p, map).map_err(|e| e.to_string())?;
        Ok(run_args(argv(&[
            "adopt",
            &r.path().display().to_string(),
            "--map",
            &p.display().to_string(),
            "--check",
        ])))
    };
    assert_eq!(dry("V1 src\n")?, ExitCode::SUCCESS, "would apply");
    assert_eq!(dry("V1 no-such-node\n")?, ExitCode::from(1), "refused");
    let after = std::fs::read_to_string(r.path().join("SPEC.md"))
        .map_err(|e| e.to_string())?;
    assert_eq!(before, after, "--check wrote");
    Ok(())
}

/// A row no node claims stays at root and is NAMED (`src/adopt:V2`); the
/// proposal still exits 1 because V1 has a home to go to.
#[test]
fn a_row_nobody_claims_is_named_and_stays() -> Result<(), String> {
    let r = adopt_fixture()?;
    r.write(
        "SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nx\n\n## \u{a7}F FEDERATION\n\n\
             dir|owns|\u{22a5}owns|tokens\nsrc|parser input|the goal|-\n\n\
             ## \u{a7}V INVARIANTS\n\nV1: the parser rejects bad input\n\
             V2: the weather is nice\n",
    )?;
    let p = crate::adopt::propose(r.path())?;
    assert_eq!(p.unplaced, vec!["V2".to_string()]);
    assert_eq!(adopt_propose(r.path()), ExitCode::from(1));
    Ok(())
}

/// More than three unreadable rows: three are quoted and the rest counted,
/// and the answer is still usage (2), never "nothing to move".
#[test]
fn many_unreadable_rows_are_counted_not_all_quoted() -> Result<(), String> {
    let r = adopt_fixture()?;
    let rows: String = (1..=5)
        .map(|n| format!("| T{n} | . | task {n} | - |\n"))
        .collect();
    r.write(
        "SPEC.md",
        &format!(
            "# SPEC\n\n## \u{a7}G GOAL\n\nx\n\n## \u{a7}T TASKS\n\n\
             | id | status | task | cites |\n| --- | --- | --- | --- |\n{rows}"
        ),
    )?;
    let found = crate::adopt::unreadable(r.path())?;
    assert_eq!(found.len(), 5);
    assert_eq!(adopt_unreadable(r.path(), &found), ExitCode::from(2));
    Ok(())
}
