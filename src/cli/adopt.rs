//! `adopt` -- migrate a foreign single-file SPEC.md onto a federation, or propose the map for it.

use super::*;

/// `sherd adopt <dir> [--map FILE] [--check]` -- a foreign single-file
/// `SPEC.md` onto a federation.
///
/// Without `--map` this PROPOSES and writes nothing, exactly as `split` does
/// and for the same reason: which node owns which rule is a judgement
/// (`src/adopt:V2`). The proposal is printed AS the map file, so `sherd adopt
/// <dir> > map`, edit, `sherd adopt <dir> --map map` is the whole workflow.
///
/// Exit 1 means a migration is PENDING or was WRITTEN, which is `sync`'s
/// discipline one verb over: a tree that still has rows to move was not
/// clean when it was committed. A rerun over a migrated tree finds nothing
/// to move and exits 0 (`src/adopt:V6`).
pub(super) fn adopt_cmd(root: &Path, args: &[String]) -> ExitCode {
    let dry = args.iter().any(|a| a == "--check");
    // BEFORE any count is printed (`src/adopt:V8`). A source this reader
    // cannot parse produces `0 rows read`, which in adopt's exit scheme means
    // "nothing to move" -- the opposite answer, in the same words, at the
    // same exit code (`src/adopt:B4`).
    match crate::adopt::unreadable(root) {
        Ok(found) if !found.is_empty() => {
            return adopt_unreadable(root, &found);
        }
        Ok(_) => {}
        Err(e) => return adopt_failed(&e),
    }
    match flag_value(args, "--map") {
        None => adopt_propose(root),
        Some(file) => adopt_with_map(root, file, dry),
    }
}

/// Rows this reader could not parse, named with the line each sits on, and
/// exit 2 -- the USAGE code, because the file is in a form the verb does not
/// accept. Exit 0 keeps meaning "I read this and there is nothing to move".
pub(super) fn adopt_unreadable(
    root: &Path,
    found: &[crate::spec::Unreadable],
) -> ExitCode {
    let path = root.join("SPEC.md");
    eprintln!(
        "adopt: {}: {} id-shaped row(s) in a form this does not read",
        path.display(),
        found.len()
    );
    for u in found.iter().take(3) {
        eprintln!("  line {}: {}", u.line, u.text.trim());
    }
    if found.len() > 3 {
        eprintln!("  ... and {} more", found.len().saturating_sub(3));
    }
    eprintln!(
        "  a row opens its line with the id: `T1|status|task|cites`, \
         pipe-delimited (FORMAT.md). convert the table and run this again."
    );
    ExitCode::from(2)
}

/// The argument after a flag, e.g. the `FILE` of `--map FILE`.
pub(super) fn flag_value<'a>(
    args: &'a [String],
    flag: &str,
) -> Option<&'a String> {
    let at = args.iter().position(|a| a == flag)?;
    args.get(at.saturating_add(1))
        .filter(|v| !v.starts_with("--"))
}

/// PROPOSE, and print the proposal in the map's own format.
pub(super) fn adopt_propose(root: &Path) -> ExitCode {
    let proposal = match crate::adopt::propose(root) {
        Ok(p) => p,
        Err(e) => return adopt_failed(&e),
    };
    for p in &proposal.placements {
        println!("{} {}\t# {}", p.id, p.home, p.why.join(" "));
    }
    adopt_summary(&proposal);
    if proposal.placements.is_empty() {
        return ExitCode::SUCCESS;
    }
    ExitCode::from(1)
}

/// What the proposal found, on stderr so stdout stays a usable map file.
///
/// The unplaced rows are NAMED (`src/adopt:V2`). A row nobody claims is a
/// legitimate resting state at root, but it is a reader's decision to leave
/// it there, and a count that hid them would make the decision for them.
pub(super) fn adopt_summary(p: &crate::adopt::Proposal) {
    eprintln!(
        "  {} rows read · {} placed · {} unplaced",
        p.rows_in,
        p.placements.len(),
        p.unplaced.len()
    );
    if !p.unplaced.is_empty() {
        eprintln!("  staying at root: {}", p.unplaced.join(" "));
    }
}

/// Apply a map, or report what applying it would refuse.
pub(super) fn adopt_with_map(root: &Path, file: &str, dry: bool) -> ExitCode {
    let map = match read_map_file(file) {
        Ok(m) => m,
        Err(e) => return adopt_failed(&e),
    };
    if dry {
        return adopt_dry(root, &map);
    }
    match crate::adopt::apply(root, &map) {
        Ok(r) => adopt_wrote(&r),
        Err(e) => adopt_failed(&e),
    }
}

/// Read and parse a map file.
pub(super) fn read_map_file(
    file: &str,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(file)
        .map_err(|e| format!("adopt: {file}: {e}"))?;
    crate::adopt::read_map(&text)
}

/// `--check`: every refusal, and nothing written.
pub(super) fn adopt_dry(
    root: &Path,
    map: &std::collections::BTreeMap<String, String>,
) -> ExitCode {
    let refused = crate::adopt::refusals(root, map);
    for r in &refused {
        eprintln!("{r}");
    }
    if refused.is_empty() {
        eprintln!("  {} rows would move · nothing written", map.len());
        return ExitCode::SUCCESS;
    }
    ExitCode::from(1)
}

/// The counts a migration is judged by (`src/adopt:V1`), then exit 1 because
/// it WROTE.
pub(super) fn adopt_wrote(r: &crate::adopt::Report) -> ExitCode {
    for f in &r.files {
        println!("{f}");
    }
    for v in &r.carried {
        eprintln!("  carried in, not caused here: {v}");
    }
    eprintln!(
        "  {} rows read · {} moved · {} stayed at root · {} files written",
        r.rows_in,
        r.moved,
        r.stayed,
        r.files.len()
    );
    ExitCode::from(1)
}

pub(super) fn adopt_failed(msg: &str) -> ExitCode {
    eprintln!("{msg}");
    ExitCode::from(1)
}

#[cfg(test)]
mod tests {
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
}
