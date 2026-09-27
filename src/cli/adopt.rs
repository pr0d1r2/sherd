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
#[path = "tests/adopt.rs"]
mod tests;
