//! Brownfield on-ramp: a foreign single-file `SPEC.md` onto a federation.
//!
//! `init` refuses a file that exists and `split` proposes from CODE and never
//! writes, so a repository that already HAS a spec -- every repository but
//! this one -- had no path in. This verb is that path: it PROPOSES which node
//! owns each row, and writes only from a map handed back (V2).

use crate::{fed, spec};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// A proposed home for one row, and the words that proposed it.
#[derive(Debug, Clone)]
pub struct Placement {
    /// The row id, e.g. `V9`.
    pub id: String,
    /// The node path the lens matched, relative to root.
    pub home: String,
    /// The words of the row that the node's `§F` lens also uses.
    pub why: Vec<String>,
}

/// What a proposal found. Every row of the source appears exactly once, in
/// `placements` or in `unplaced` -- that is V1, and `rows_in` is what proves
/// it rather than assumes it.
#[derive(Debug, Clone)]
pub struct Proposal {
    /// Rows read from the source spec.
    pub rows_in: usize,
    /// Rows a single node's lens claimed.
    pub placements: Vec<Placement>,
    /// Rows no node claimed, or that two claimed equally. They stay at root.
    pub unplaced: Vec<String>,
}

/// What an application did, counted rather than assumed (V1).
#[derive(Debug, Clone)]
pub struct Report {
    /// Rows read from the source spec.
    pub rows_in: usize,
    /// Rows written into a node other than root.
    pub moved: usize,
    /// Rows that stayed at root.
    pub stayed: usize,
    /// Every file rewritten, root first.
    pub files: Vec<String>,
    /// Violations the tree carried IN, reported and not repaired (`B1`).
    pub carried: Vec<String>,
}

/// The source spec: the root `SPEC.md` of the tree being adopted.
fn read_source(root: &Path) -> Result<String, String> {
    let path = root.join("SPEC.md");
    std::fs::read_to_string(&path)
        .map_err(|e| format!("adopt: {}: {e}", path.display()))
}

/// PROPOSE a row-to-node map. Writes nothing, ever.
///
/// # Errors
/// When the root carries no readable `SPEC.md` -- there is nothing to adopt.
pub fn propose(root: &Path) -> Result<Proposal, String> {
    let source = read_source(root)?;
    let homes: Vec<fed::Home> = fed::declared(root)
        .into_iter()
        .filter(|h| h.node != ".")
        .collect();
    let rows = spec::rows(&source);
    let mut out = Proposal {
        rows_in: rows.len(),
        placements: vec![],
        unplaced: vec![],
    };
    for row in rows {
        place(&mut out, row, &homes);
    }
    Ok(out)
}

/// One row, onto the proposal: a home it earned, or its id in `unplaced`.
///
/// Both arms record the row, and that is V1 in one function -- a row read and
/// then silently dropped is the conservation failure that costs the memory of
/// a defect.
fn place(out: &mut Proposal, row: spec::Row, homes: &[fed::Home]) {
    match best(&row, homes) {
        Some((home, why)) => out.placements.push(Placement {
            id: row.id,
            home,
            why,
        }),
        None => out.unplaced.push(row.id),
    }
}

/// The single node whose lens matches this row best, or none.
///
/// RANKED, and a tie is not an answer. `src/plan`'s `route` learned that
/// rounding an ambiguous match down to its first hit makes the interesting
/// case indistinguishable from the certain one; here the cost is worse,
/// because the tool would be proposing to MOVE law on a coin flip. An
/// unclaimed row stays at root and is NAMED there (V2).
fn best(row: &spec::Row, homes: &[fed::Home]) -> Option<(String, Vec<String>)> {
    let words = significant(&subject(&row.text));
    let mut hits: Vec<(String, Vec<String>)> = homes
        .iter()
        .map(|h| (h.node.clone(), matched(&words, &h.owns)))
        .filter(|(_, w)| !w.is_empty())
        .collect();
    let top = hits.iter().map(|(_, w)| w.len()).max()?;
    hits.retain(|(_, w)| w.len() == top);
    (hits.len() == 1).then(|| hits.pop()).flatten()
}

/// The row's SUBJECT: its text with citations taken out.
///
/// A citation is a LINK, not a subject, and a moved row's citations carry the
/// OWNER's path -- `` `src/git:V2` `` -- whose words then match that owner's
/// own lens. Scoring the raw text makes each proposal depend on the last
/// migration rather than on what the row is about, and `B2` is what that
/// cost: a rerun over a migrated ashlar proposed twenty more moves, every one
/// of them justified by a path the previous run had written in.
fn subject(text: &str) -> String {
    text.split('`')
        .enumerate()
        .filter(|(i, part)| i % 2 == 0 || !is_citation(part))
        .map(|(_, part)| format!("{part} "))
        .collect()
}

/// Does this backticked span name `owner:ID` rather than code?
fn is_citation(span: &str) -> bool {
    span.split_once(':').is_some_and(|(owner, id)| {
        !owner.is_empty() && !spec::rows(&format!("{id}:")).is_empty()
    })
}

/// The words a lens can match on: four characters or longer, lowercased.
///
/// Shorter ones are the articles and operators of caveman prose, and one of
/// them matching would make every node a candidate -- `src/plan`'s vocabulary
/// draws the line in the same place for the same reason.
fn significant(text: &str) -> Vec<String> {
    let mut words: Vec<String> = text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 4)
        .map(str::to_string)
        .collect();
    words.sort();
    words.dedup();
    words
}

/// The words this row and this lens share.
fn matched(words: &[String], lens: &str) -> Vec<String> {
    let lens = significant(lens);
    words.iter().filter(|w| lens.contains(w)).cloned().collect()
}

/// Parse a row-to-node map: `<id> <node>` per line, `#` starts a comment.
///
/// This is the file a reader hands BACK (V2), and it is exactly the form
/// `propose` prints, so `sherd adopt <dir> > map` and then editing the map is
/// the whole workflow. A format only the tool could write would make the
/// judgement step harder than doing it by hand, and a judgement step people
/// route around is not a safeguard.
///
/// # Errors
/// On a line that is not `<id> <node>`, or an id given two homes -- which is
/// V1's "exactly once" failing in the input rather than the output.
pub fn read_map(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut out = BTreeMap::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let (id, node) = line
            .split_once(char::is_whitespace)
            .ok_or_else(|| format!("adopt: `{line}` is not `<id> <node>`"))?;
        let seen = out.insert(id.to_string(), node.trim().to_string());
        if seen.is_some() {
            return Err(format!("adopt: {id} is given two homes"));
        }
    }
    Ok(out)
}

/// Every reason this map may not be applied.
///
/// ALL of them, before anything is written. A migration is a batch, and a
/// reader who fixes one line only to be told about the next has to run the
/// whole thing again to learn about the third.
#[must_use]
pub fn refusals(root: &Path, map: &BTreeMap<String, String>) -> Vec<String> {
    let declared: BTreeSet<String> =
        fed::declared(root).into_iter().map(|h| h.node).collect();
    let source = read_source(root).unwrap_or_default();
    map.iter()
        .filter_map(|(id, node)| refusal(root, &declared, &source, (id, node)))
        .collect()
}

/// Why one mapping is refused, if it is.
fn refusal(
    root: &Path,
    declared: &BTreeSet<String>,
    source: &str,
    entry: (&String, &String),
) -> Option<String> {
    let (id, node) = entry;
    if !declared.contains(node) {
        return Some(format!(
            "adopt: no `\u{a7}F` row declares `{node}` -- adoption places rows onto structure that exists ({id})"
        ));
    }
    if !spec::declares(source, id) {
        return Some(format!("adopt: the source declares no `{id}`"));
    }
    let held = std::fs::read_to_string(root.join(node).join("SPEC.md"))
        .is_ok_and(|t| spec::declares(&t, id));
    held.then(|| {
        format!("adopt: `{node}` already declares `{id}` -- moving it would make two rows with one id")
    })
}

/// Where each row of the source ends up: the map, or root for the rest.
fn homes_of(
    rows: &[spec::Row],
    map: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    rows.iter()
        .map(|r| {
            let home =
                map.get(&r.id).cloned().unwrap_or_else(|| ".".to_string());
            (r.id.clone(), home)
        })
        .collect()
}

/// MIGRATE the source onto the federation the map names.
///
/// Nothing is written until every output has been CHECKED (V5): a migration
/// whose result the project's own checker rejects has shipped a second
/// dialect, and half-written is worse than not started.
///
/// # Errors
/// On a refused map, an unreadable source, a rejected output, or a failed
/// write.
pub fn apply(
    root: &Path,
    map: &BTreeMap<String, String>,
) -> Result<Report, String> {
    let refused = refusals(root, map);
    if !refused.is_empty() {
        return Err(refused.join("\n"));
    }
    let source = read_source(root)?;
    let rows = spec::rows(&source);
    let homes = homes_of(&rows, map);
    let writes = writes_for(root, &source, &rows, &homes);
    checked(&writes)?;
    // BEFORE the write, or it reads back what the migration just produced
    // and calls the tree's own violations its own.
    let carried = carried(&writes);
    commit(&writes)?;
    let mut out = report(&rows, &homes, &writes);
    out.carried = carried;
    Ok(out)
}

/// Every file this migration would write, and its whole new text.
fn writes_for(
    root: &Path,
    source: &str,
    rows: &[spec::Row],
    homes: &BTreeMap<String, String>,
) -> Vec<(PathBuf, String)> {
    let moved: BTreeSet<String> = homes
        .iter()
        .filter(|(_, h)| *h != ".")
        .map(|(id, _)| id.clone())
        .collect();
    let mut out =
        vec![(root.join("SPEC.md"), root_after(source, &moved, homes))];
    out.extend(
        destinations(homes)
            .into_iter()
            .map(|node| write_for(root, rows, homes, node)),
    );
    out
}

/// One receiving node's file and its whole new text.
fn write_for(
    root: &Path,
    rows: &[spec::Row],
    homes: &BTreeMap<String, String>,
    node: String,
) -> (PathBuf, String) {
    let batch: Vec<&spec::Row> = rows
        .iter()
        .filter(|r| homes.get(&r.id).is_some_and(|h| *h == node))
        .collect();
    let path = root.join(&node).join("SPEC.md");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let new = receive(&text, &node, &batch, homes);
    (path, new)
}

/// The nodes receiving rows, root excluded and each named once.
fn destinations(homes: &BTreeMap<String, String>) -> BTreeSet<String> {
    homes.values().filter(|h| *h != ".").cloned().collect()
}

/// The root file after the move: moved rows gone, everything that stays
/// requalified so its citations still resolve.
///
/// EVERY line is requalified, not only the row lines. `§G`, `§C` and `§I` are
/// prose that cites rules too, and a migration that fixed the tables while
/// leaving the prose pointing at rows that left would break exactly the links
/// a reader follows first.
fn root_after(
    source: &str,
    moved: &BTreeSet<String>,
    homes: &BTreeMap<String, String>,
) -> String {
    let kept: Vec<String> = source
        .lines()
        .filter(|l| !opens_a_moved_row(l, moved))
        .map(|l| spec::requalify(l, ".", homes))
        .collect();
    format!("{}\n", kept.join("\n").trim_end())
}

/// Does this line OPEN one of the rows that left?
fn opens_a_moved_row(line: &str, moved: &BTreeSet<String>) -> bool {
    spec::rows(line)
        .first()
        .is_some_and(|r| moved.contains(&r.id))
}

/// One node's file after it receives rows.
fn receive(
    text: &str,
    node: &str,
    rows: &[&spec::Row],
    homes: &BTreeMap<String, String>,
) -> String {
    let mut out = text.to_string();
    for letter in ['R', 'V', 'T', 'B'] {
        let batch: Vec<String> = rows
            .iter()
            .filter(|r| r.section == letter)
            .map(|r| spec::requalify(&r.text, node, homes))
            .collect();
        if !batch.is_empty() {
            out = add_rows(&out, letter, &batch);
        }
    }
    out
}

/// Append rows to a section, creating the section in FORMAT's position when
/// the node has none.
fn add_rows(text: &str, letter: char, batch: &[String]) -> String {
    let Some(anchor) = anchor_for(text, letter) else {
        return text.to_string();
    };
    let head = existing_body(text, letter).unwrap_or_else(|| {
        format!("## \u{a7}{}{}", name(letter), header(letter))
    });
    let body = format!("{head}\n{}", batch.join("\n"));
    let (sec, at) = (letter.to_string(), anchor.to_string());
    spec::upsert_section(text, &sec, body.trim_end(), &at)
}

/// The section as it stands, heading included, or `None` when absent.
fn existing_body(text: &str, letter: char) -> Option<String> {
    let head = format!("## \u{a7}{letter}");
    spec::sections(text)
        .into_iter()
        .find(|(h, _)| h.starts_with(&head))
        .map(|(h, b)| format!("{h}\n{}", b.trim_end()))
}

/// FORMAT's section order. A new section goes after the last one present that
/// FORMAT puts before it -- appending at the end would put `§B` above `§V`
/// the moment a node has both.
const ORDER: [char; 9] = ['G', 'F', 'N', 'C', 'I', 'R', 'V', 'T', 'B'];

/// The section a new one goes AFTER, or `None` when nothing precedes it.
fn anchor_for(text: &str, letter: char) -> Option<char> {
    if existing_body(text, letter).is_some() {
        return Some(letter);
    }
    let want = ORDER.iter().position(|c| *c == letter)?;
    let present: BTreeSet<char> = spec::sections(text)
        .iter()
        .filter_map(|(h, _)| h.strip_prefix("## \u{a7}")?.chars().next())
        .collect();
    ORDER
        .iter()
        .take(want)
        .rev()
        .find(|c| present.contains(c))
        .copied()
}

/// A section's full heading name.
fn name(letter: char) -> &'static str {
    match letter {
        'R' => "R RESEARCH",
        'V' => "V INVARIANTS",
        'T' => "T TASKS",
        _ => "B BUGS",
    }
}

/// What follows a fresh section's heading: the blank line FORMAT puts under
/// every heading, then the table header for the sections that have one.
///
/// The statement sections (`§V`) get the blank line and nothing else. Rows are
/// appended with a single newline, so anything ending in one here produces the
/// stray blank line that separated a `§B` header from its first row.
fn header(letter: char) -> &'static str {
    match letter {
        'R' => "\n\nid|topic|finding|src",
        'T' => "\n\nid|status|task|cites",
        'B' => "\n\nid|date|cause|fix",
        _ => "\n",
    }
}

/// V5: the migration may not ADD a violation.
///
/// The DELTA, not the total, and `B1` is why. A brownfield spec generally
/// does not pass `check` already -- ashlar's `§B` rows run B3, B2, B1 -- and
/// checking the total refused every such repository, which is every
/// repository this verb exists for. A violation the tree already carried is
/// the tree's own; it is reported, not fixed and not blamed on the move.
fn checked(writes: &[(PathBuf, String)]) -> Result<(), String> {
    let bad: Vec<String> =
        writes.iter().flat_map(|(p, t)| introduced(p, t)).collect();
    if bad.is_empty() {
        return Ok(());
    }
    Err(format!(
        "adopt: the migration would ADD these, so nothing was written:\n{}",
        bad.join("\n")
    ))
}

/// The violations this write adds to the ones its file already had.
fn introduced(path: &Path, after: &str) -> Vec<String> {
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut before = counted(&existing);
    spec::check(after)
        .into_iter()
        .map(|v| v.to_string())
        .filter(|v| !spend(&mut before, v))
        .map(|v| format!("{}: {v}", path.display()))
        .collect()
}

/// How many times over the file already carried each violation.
///
/// COUNTED rather than a set: two copies of one message is worse than one,
/// and a set would report the second as pre-existing.
fn counted(text: &str) -> BTreeMap<String, usize> {
    let mut out: BTreeMap<String, usize> = BTreeMap::new();
    for v in spec::check(text) {
        let n = out.entry(v.to_string()).or_default();
        *n = n.saturating_add(1);
    }
    out
}

/// Spend one pre-existing occurrence. True when this one was already there.
fn spend(before: &mut BTreeMap<String, usize>, v: &str) -> bool {
    match before.get_mut(v) {
        Some(n) if *n > 0 => {
            *n = n.saturating_sub(1);
            true
        }
        _ => false,
    }
}

/// The violations the tree carried in, reported rather than repaired.
fn carried(writes: &[(PathBuf, String)]) -> Vec<String> {
    writes
        .iter()
        .flat_map(|(p, _)| {
            let text = std::fs::read_to_string(p).unwrap_or_default();
            spec::check(&text)
                .into_iter()
                .map(move |v| format!("{}: {v}", p.display()))
        })
        .collect()
}

/// Write, once every output has passed.
fn commit(writes: &[(PathBuf, String)]) -> Result<(), String> {
    for (path, text) in writes {
        std::fs::write(path, text)
            .map_err(|e| format!("adopt: {}: {e}", path.display()))?;
    }
    Ok(())
}

/// Count in and out, which is what makes V1 a measurement.
fn report(
    rows: &[spec::Row],
    homes: &BTreeMap<String, String>,
    writes: &[(PathBuf, String)],
) -> Report {
    let moved = rows
        .iter()
        .filter(|r| homes.get(&r.id).is_some_and(|h| h != "."))
        .count();
    Report {
        rows_in: rows.len(),
        moved,
        stayed: rows.len().saturating_sub(moved),
        files: writes
            .iter()
            .map(|(p, _)| p.display().to_string())
            .collect(),
        carried: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testrepo::TestRepo;

    /// A monolith of the shape every unfederated repository has: rules,
    /// tasks and bugs in one file, citing each other by bare id.
    const SOURCE: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nparse input, render output\n\n\
## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\nsrc|parsing and rendering code|the goal itself|-\n\n\
## \u{a7}V INVARIANTS\n\nV1: the parser rejects malformed input rather than guessing\n\
V2: rendering never mutates the tree the parser produced\nV3: every release is tagged, see V1\n\n\
## \u{a7}T TASKS\n\nid|status|task|cites\nT1|.|nested groups|V1\nT2|.|render colour output|V2\n\n\
## \u{a7}B BUGS\n\nid|date|cause|fix\nB1|2026-01-01|the parser looped on empty input|V1 restated\n";

    /// The `§F` table that DECLARES the two nodes. Placement may name these
    /// and nothing else (V3).
    const CHILD: &str = "# SPEC\n\n## \u{a7}G GOAL\n\ncode nodes\n\n\
## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n\
parse|parser, input, malformed rejection|rendering, output colour|-\n\
render|rendering, output, colour|parsing, input|-\n";

    /// A repo mid-adoption: one monolith, a declared federation, and two
    /// nodes scaffolded by `init` and so carrying zero ids of their own.
    fn tree(tag: &str) -> Result<TestRepo, String> {
        let r = TestRepo::new(tag)?;
        r.write("SPEC.md", SOURCE)?;
        r.write("src/SPEC.md", CHILD)?;
        r.write("src/parse/SPEC.md", &spec::scaffold("src/parse", &[]))?;
        r.write("src/render/SPEC.md", &spec::scaffold("src/render", &[]))?;
        Ok(r)
    }

    fn read(root: &Path, rel: &str) -> Result<String, String> {
        std::fs::read_to_string(root.join(rel)).map_err(|e| e.to_string())
    }

    /// V1. Six rows in, six rows accounted for. A row read and then dropped
    /// costs the memory of a defect, and nothing else would notice.
    #[test]
    fn every_row_is_accounted_for_exactly_once() -> Result<(), String> {
        let r = tree("adopt-conserve")?;
        let p = propose(r.path())?;
        assert_eq!(p.rows_in, 6, "V1 V2 V3 T1 T2 B1");
        assert_eq!(
            p.placements.len().saturating_add(p.unplaced.len()),
            p.rows_in,
            "placed {:?}, unplaced {:?}",
            p.placements.iter().map(|x| &x.id).collect::<Vec<_>>(),
            p.unplaced
        );
        Ok(())
    }

    /// V2. `V2` names rendering AND the parser, so both lenses claim it
    /// equally. A tie is not an answer: the row stays at root and is named,
    /// rather than being rounded to whichever node was walked first.
    #[test]
    fn a_row_two_nodes_claim_equally_is_placed_nowhere() -> Result<(), String> {
        let r = tree("adopt-tie")?;
        let p = propose(r.path())?;
        assert!(
            p.unplaced.contains(&"V2".to_string()),
            "a tie is not a placement: {:?}",
            p.placements.iter().map(|x| &x.id).collect::<Vec<_>>()
        );
        Ok(())
    }

    /// V3. A home no `§F` row declares is refused. Inventing an edge here
    /// would place a row into a node no reader can reach by descent.
    #[test]
    fn a_home_no_table_declares_is_refused() -> Result<(), String> {
        let r = tree("adopt-undeclared")?;
        let map = read_map("V1 src/nowhere")?;
        let refused = refusals(r.path(), &map);
        assert_eq!(refused.len(), 1, "{refused:?}");
        assert!(
            refused.iter().any(|m| m.contains("src/nowhere")),
            "{refused:?}"
        );
        Ok(())
    }

    /// V1, in the direction that is easy to miss: a destination that already
    /// declares the id would end up with TWO rows numbered `V1`, which is the
    /// copy V1 forbids wearing the costume of a move.
    #[test]
    fn a_destination_already_holding_the_id_is_refused() -> Result<(), String> {
        let r = tree("adopt-collide")?;
        let held = read(r.path(), "src/parse/SPEC.md")?
            + "V1: a rule this node already had\n";
        r.write("src/parse/SPEC.md", &held)?;
        let map = read_map("V1 src/parse")?;
        assert!(
            refusals(r.path(), &map).iter().any(|m| m.contains("V1")),
            "two rows with one id must be refused"
        );
        Ok(())
    }

    /// V4. The number survives, the owner is added, and the row itself is
    /// carried verbatim rather than retyped.
    #[test]
    fn a_moved_row_keeps_its_number_and_its_citers_follow() -> Result<(), String>
    {
        let r = tree("adopt-requalify")?;
        apply(r.path(), &read_map("V1 src/parse\nT2 src/render")?)?;
        let root = read(r.path(), "SPEC.md")?;
        let parse = read(r.path(), "src/parse/SPEC.md")?;
        assert!(root.contains("tagged, see `src/parse:V1`"), "{root}");
        assert!(!root.contains("V1: the parser"), "the row left root");
        assert!(parse.contains("V1: the parser rejects"), "arrived intact");
        assert!(
            read(r.path(), "src/render/SPEC.md")?.contains("`.:V2`"),
            "a citation pointing back at root gains root's own owner"
        );
        Ok(())
    }

    /// `B1`. A brownfield spec generally does not pass `check` already, so a
    /// migration is judged on what it ADDS. ashlar's `§B` rows run B3, B2, B1
    /// -- refusing on the total refused every repository the verb is for.
    #[test]
    fn a_violation_the_source_already_had_does_not_refuse_the_move()
    -> Result<(), String> {
        let r = tree("adopt-carried")?;
        let unsorted = read(r.path(), "SPEC.md")?.replace(
            "B1|2026-01-01",
            "B3|2026-01-01|a later bug, filed first|V1 restated\n\
             B1|2026-01-01",
        );
        r.write("SPEC.md", &unsorted)?;
        assert!(
            !spec::check(&unsorted).is_empty(),
            "the source is already bad"
        );
        let out = apply(r.path(), &read_map("T2 src/render")?)?;
        assert_eq!(out.moved, 1, "the move still happens");
        assert!(!out.carried.is_empty(), "and the tree's own is reported");
        Ok(())
    }

    /// V5. The output is checked BEFORE it is offered: a migration whose
    /// result the project's own checker rejects has shipped a second dialect.
    #[test]
    fn the_migration_passes_the_checker_that_gates_every_spec()
    -> Result<(), String> {
        let r = tree("adopt-checked")?;
        let map = read_map("V1 src/parse\nB1 src/parse\nT2 src/render")?;
        let report = apply(r.path(), &map)?;
        assert_eq!(report.moved, 3);
        assert_eq!(report.moved.saturating_add(report.stayed), report.rows_in);
        for rel in ["SPEC.md", "src/parse/SPEC.md", "src/render/SPEC.md"] {
            let found = spec::check(&read(r.path(), rel)?);
            assert!(found.is_empty(), "{rel}: {found:?}");
        }
        Ok(())
    }

    /// V5, the half that matters more: a refused migration writes NOTHING.
    /// Half-applied is worse than not started, because the counts that would
    /// tell you so are the ones that never ran.
    #[test]
    fn a_refused_map_leaves_every_file_untouched() -> Result<(), String> {
        let r = tree("adopt-atomic")?;
        let before = read(r.path(), "SPEC.md")?;
        let map = read_map("V1 src/parse\nV3 src/nowhere")?;
        assert!(apply(r.path(), &map).is_err(), "the map names no such node");
        assert_eq!(read(r.path(), "SPEC.md")?, before);
        assert!(!read(r.path(), "src/parse/SPEC.md")?.contains("V1:"));
        Ok(())
    }

    /// V6. A foreign repo is edited between the proposal and the apply, so a
    /// verb unsafe to repeat is a verb nobody dares finish. A second run over
    /// a migrated tree finds nothing to move.
    #[test]
    fn a_second_run_over_a_migrated_tree_moves_nothing() -> Result<(), String> {
        let r = tree("adopt-rerun")?;
        let first = propose(r.path())?;
        let text: String = first
            .placements
            .iter()
            .map(|p| format!("{} {}\n", p.id, p.home))
            .collect();
        apply(r.path(), &read_map(&text)?)?;
        let again = propose(r.path())?;
        assert!(
            again.placements.is_empty(),
            "still proposing: {:?}",
            again.placements.iter().map(|p| &p.id).collect::<Vec<_>>()
        );
        Ok(())
    }

    /// `B2`. Requalification writes the destination's PATH into every row
    /// that cited a moved id, so scoring the raw text lets one migration
    /// justify the next: a rerun over a migrated ashlar proposed twenty more
    /// moves, each because the previous run had written `src/git` into the
    /// row. A citation is a link, not a subject.
    #[test]
    fn a_citation_is_not_a_word_the_row_matches_on() {
        let cited = spec::Row {
            section: 'T',
            id: "T9".to_string(),
            text: "T9|.|tagged release|`src/render:V2`".to_string(),
            line: 1,
        };
        let homes = vec![fed::Home {
            node: "src/render".to_string(),
            owns: "render, output, template".to_string(),
            not_owns: String::new(),
        }];
        assert!(
            best(&cited, &homes).is_none(),
            "`render` reached the row only through the citation"
        );
    }

    /// The map is a file a human edits, so it forgives comments and blank
    /// lines and refuses the two things that would silently change what gets
    /// moved: a line that is not a mapping, and an id given two homes.
    #[test]
    fn a_map_forgives_comments_and_refuses_ambiguity() {
        let ok = read_map("# a note\n\nV1 src/parse  # why\nT2 src/render\n");
        assert_eq!(ok.map(|m| m.len()), Ok(2));
        assert!(
            read_map("V1 src/parse\nV1 src/render").is_err(),
            "two homes"
        );
        assert!(read_map("V1").is_err(), "not a mapping");
    }
}
