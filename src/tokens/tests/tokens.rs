use super::*;

#[test]
fn a_malformed_ceiling_line_is_named_and_numbered() {
    // A skipped line means a ceiling that silently stops being enforced,
    // which is `.:B7` -- chains drifting over unseen. Both malformations
    // name the LINE so it can be fixed rather than hunted.
    let short = Ceilings::parse("src/fed 9000\nonlyonefield\n")
        .err()
        .unwrap_or_default();
    assert!(short.contains(":2"), "name the line: {short}");
    assert!(short.contains("<path> <limit>"), "say the shape: {short}");
    let nan = Ceilings::parse("src/fed notanumber\n")
        .err()
        .unwrap_or_default();
    assert!(nan.contains(":1"), "name the line: {nan}");
    assert!(nan.contains("not a token count"), "say why: {nan}");
}

#[test]
fn a_file_that_cannot_be_read_is_an_error_never_a_zero() {
    // V48. A missing file contributing zero tokens makes an over-budget
    // chain read as comfortably under one, and every ceiling in this repo
    // is compared against these numbers.
    assert!(count_file(std::path::Path::new("/no/such/file.md")).is_err());
}

#[test]
fn counting_a_real_file_agrees_with_counting_its_text() {
    assert_eq!(file_matches_text(), Ok(()));
}

fn file_matches_text() -> Result<(), String> {
    let p = std::env::temp_dir()
        .join(format!("sherd-tok-{}.md", std::process::id()));
    let body = "# SPEC\n\nV1: something ! hold\n";
    std::fs::write(&p, body).map_err(|e| e.to_string())?;
    let from_file = count_file(&p).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&p);
    assert_eq!(
        from_file.tokens,
        count(body).tokens,
        "reading a file must not change what its text costs"
    );
    Ok(())
}

#[test]
fn count_carries_its_method() {
    let c = count("hello world");
    assert!(c.tokens > 0);
    assert_eq!(c.method, "o200k");
}

#[test]
fn ceilings_parse_the_itok_format() {
    let c = Ceilings::parse("# comment\n\nSPEC.md    31000\nsrc/fed 4000\n")
        .unwrap();
    assert_eq!(c.for_path("SPEC.md"), 31_000);
    assert_eq!(c.for_path("src/fed/mod.rs"), 4_000);
    assert_eq!(
        c.for_path("src/lens/mod.rs"),
        DEFAULT_NODE,
        "unlisted -> default"
    );
}

#[test]
fn longest_prefix_wins() {
    let c = Ceilings::parse("src 1000\nsrc/fed 4000\n").unwrap();
    assert_eq!(c.for_path("src/fed/mod.rs"), 4_000);
    assert_eq!(c.for_path("src/lens/mod.rs"), 1_000);
}

#[test]
fn an_unparsable_limit_is_an_error_not_a_skip() {
    // itok B7: a skipped row made a gate check nothing while exiting 0.
    let e = Ceilings::parse("SPEC.md 20.5k\n").unwrap_err();
    assert!(e.contains("not a token count"), "{e}");
    assert!(e.contains(":1:"), "must name the line: {e}");
}

#[test]
fn a_missing_file_is_a_cold_start() {
    let c = Ceilings::load(std::path::Path::new("/nonexistent")).unwrap();
    assert_eq!(c.for_path("anything"), DEFAULT_NODE);
}

/// `V5`: ABSENT is a cold start; PRESENT and unreadable is an error. Every
/// read error was taken for absence, so a `.context-limits` nobody could
/// read silently became the default ceilings (`B1`).
#[test]
fn an_unreadable_file_is_an_error_not_a_cold_start() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("tokens-unreadable")?;
    std::fs::create_dir_all(r.path().join(".context-limits"))
        .map_err(|e| e.to_string())?;
    let err = Ceilings::load(r.path())
        .err()
        .ok_or("an unreadable .context-limits read as a cold start")?;
    assert!(err.contains(".context-limits"), "names the file: {err}");
    Ok(())
}

#[test]
fn working_saturates_when_window_smaller_than_entry() {
    assert_eq!(working(16_384), 0);
    assert_eq!(working(131_072), 131_072 - ENTRY_COST);
}
