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

/// The node being split, as a path from the root -- `.` for the root.
///
/// `adopt <dir>` names it (V11). The root is the case every entry point
/// without `_at` means, so the verb on `.` behaves exactly as it did.
const ROOT: &str = ".";

/// A node's directory: the root itself for `.`, so a message names
/// `/repo/SPEC.md` rather than `/repo/./SPEC.md`.
fn dir_of(root: &Path, node: &str) -> PathBuf {
    if node == ROOT {
        root.to_path_buf()
    } else {
        root.join(node)
    }
}

/// The source spec: the `SPEC.md` of the node being split (V11).
fn read_source(root: &Path, node: &str) -> Result<String, String> {
    let path = dir_of(root, node).join("SPEC.md");
    std::fs::read_to_string(&path)
        .map_err(|e| format!("adopt: {}: {e}", path.display()))
}

/// Is `home` a node strictly BELOW `node`? Rows move down the tree, into
/// the source's own descendants (V11) -- the root's are every other node.
fn below(home: &str, node: &str) -> bool {
    if node == ROOT {
        return home != ROOT;
    }
    home.strip_prefix(node).is_some_and(|r| r.starts_with('/'))
}

/// The spec a citation path names, as a message spells it.
fn spec_of(node: &str) -> String {
    if node == ROOT {
        "SPEC.md".into()
    } else {
        format!("{node}/SPEC.md")
    }
}

/// Rows the source carries in a form this reader does not accept (V8).
///
/// Separate from [`propose`] and consulted BEFORE it, because the answer
/// changes what every other count means: `0 rows read` over a file full of
/// bracketed rows is not "nothing to move", it is "I could not read this"
/// (`B4`). One reading for every adopt invocation, propose and `--map`
/// alike, so no path can skip it.
///
/// # Errors
/// When the root carries no readable `SPEC.md` -- there is nothing to adopt.
pub fn unreadable(root: &Path) -> Result<Vec<spec::Unreadable>, String> {
    unreadable_at(root, ROOT)
}

/// As [`unreadable`], for the spec of `node` (V11).
///
/// # Errors
/// When `node` carries no readable `SPEC.md`.
pub fn unreadable_at(
    root: &Path,
    node: &str,
) -> Result<Vec<spec::Unreadable>, String> {
    Ok(spec::unreadable_rows(&read_source(root, node)?))
}

/// PROPOSE a row-to-node map. Writes nothing, ever.
///
/// # Errors
/// When the root carries no readable `SPEC.md` -- there is nothing to adopt.
pub fn propose(root: &Path) -> Result<Proposal, String> {
    propose_at(root, ROOT)
}

/// As [`propose`], splitting `node` among the nodes declared below it (V11).
///
/// # Errors
/// When `node` carries no readable `SPEC.md`.
pub fn propose_at(root: &Path, node: &str) -> Result<Proposal, String> {
    let source = read_source(root, node)?;
    let homes: Vec<fed::Home> = fed::declared(root)
        .into_iter()
        .filter(|h| below(&h.node, node))
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
    refusals_at(root, ROOT, map)
}

/// As [`refusals`], for a map splitting `node` (V11).
#[must_use]
pub fn refusals_at(
    root: &Path,
    node: &str,
    map: &BTreeMap<String, String>,
) -> Vec<String> {
    let declared: BTreeSet<String> =
        fed::declared(root).into_iter().map(|h| h.node).collect();
    let source = read_source(root, node).unwrap_or_default();
    let mut out: Vec<String> = map
        .iter()
        .filter_map(|entry| refusal(root, &declared, (node, &source), entry))
        .collect();
    out.extend(ranges_that_would_break(&source, map));
    out
}

/// Milestone rows that list a RANGE the migration would split apart (V9).
///
/// A range is the format's own affordance and the cheap way to maintain the
/// column (`microlith/V15`), so it is the shape a brownfield spec most often
/// carries -- and the rewrite that namespaces moved ids works token by token,
/// which turns `T1-T3` into `` `child:T1` ``-`T3`. What `check` then reported
/// was `T3 is in no milestone`: a SYMPTOM, three steps from the cause, on a
/// migration that wrote nothing (`B5`).
///
/// Refused up front and named, rather than expanded here. Expanding is a
/// judgement about the column's future: the ids of one milestone may end up
/// in two nodes, and which of them keeps the milestone row -- or whether the
/// table moves out of `SPEC.md` entirely, as this repository's own reporter
/// did -- is the reader's call, not a rewrite's.
///
/// A range NOBODY moves is left alone. `V6` says a second run over a migrated
/// tree finds nothing to move and says so, and a refusal that fired on a
/// range no row of the map touches would make every rerun fail.
fn ranges_that_would_break(
    source: &str,
    map: &BTreeMap<String, String>,
) -> Vec<String> {
    spec::milestone_cells(source)
        .into_iter()
        .filter_map(|(id, cell)| {
            let claimed = claims_of(source, &id)?;
            // A RANGE is the difference between what the row claims and what
            // it spells out -- measured, not re-parsed. `T1-T3` claims three
            // and writes two, and that gap is the whole detection.
            let spelled = cell.split(',').count();
            if claimed.len() <= spelled {
                return None;
            }
            let moving: Vec<String> = claimed
                .iter()
                .map(|n| format!("T{n}"))
                .filter(|t| map.contains_key(t))
                .collect();
            (!moving.is_empty()).then(|| {
                format!(
                    "adopt: `{id}` lists its tasks as a RANGE (`{cell}`) and this map moves {} -- a range cannot survive a split. Expand it into ids before adopting.",
                    moving.join(", ")
                )
            })
        })
        .collect()
}

/// What one milestone row claims, ranges expanded -- microlith's reading.
fn claims_of(source: &str, id: &str) -> Option<Vec<u32>> {
    spec::milestones(source)
        .into_iter()
        .find(|(m, _)| m == id)
        .map(|(_, claimed)| claimed)
}

/// Why one mapping is refused, if it is.
fn refusal(
    root: &Path,
    declared: &BTreeSet<String>,
    (from, source): (&str, &str),
    entry: (&String, &String),
) -> Option<String> {
    let (id, node) = entry;
    if !declared.contains(node) {
        return Some(format!(
            "adopt: no `\u{a7}F` row declares `{node}` -- adoption places rows onto structure that exists ({id})"
        ));
    }
    // Mapping a row onto its own source is how a map says "stays".
    if node != from && !below(node, from) {
        return Some(format!(
            "adopt: `{node}` is not below `{from}` -- a row moves down the tree, into a node the source declares ({id})"
        ));
    }
    if !spec::declares(source, id) {
        return Some(format!(
            "adopt: the source, {}, declares no `{id}`",
            spec_of(from)
        ));
    }
    let held = std::fs::read_to_string(root.join(node).join("SPEC.md"))
        .is_ok_and(|t| spec::declares(&t, id));
    held.then(|| {
        format!("adopt: `{node}` already declares `{id}` -- moving it would make two rows with one id")
    })
}

/// Where each row of the source ends up: the map, or the source for the rest.
fn homes_of(
    rows: &[spec::Row],
    map: &BTreeMap<String, String>,
    from: &str,
) -> BTreeMap<String, String> {
    rows.iter()
        .map(|r| {
            let home =
                map.get(&r.id).cloned().unwrap_or_else(|| from.to_string());
            (r.id.clone(), home)
        })
        .collect()
}

/// One migration: the node split, and where each of its rows goes.
struct Move<'a> {
    /// The source node, `.` for the root (V11).
    from: &'a str,
    /// Every source row's home, the source itself for a row that stays.
    homes: BTreeMap<String, String>,
}

impl Move<'_> {
    /// Only the rows that LEAVE, each with its new home.
    fn moved(&self) -> BTreeMap<String, String> {
        self.homes
            .iter()
            .filter(|(_, h)| *h != self.from)
            .map(|(id, h)| (id.clone(), h.clone()))
            .collect()
    }

    /// One line of `here`'s file with every citation of a moved row
    /// pointing at the row's new home (V11).
    fn rehome(&self, text: &str, here: &str) -> String {
        let moved = self.moved();
        let lines: Vec<String> = text
            .split_inclusive('\n')
            .map(|l| spec::rehome(l, here, self.from, &moved))
            .collect();
        lines.concat()
    }
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
    apply_at(root, ROOT, map)
}

/// As [`apply`], splitting `node` onto the nodes below it (V11).
///
/// # Errors
/// As [`apply`].
pub fn apply_at(
    root: &Path,
    node: &str,
    map: &BTreeMap<String, String>,
) -> Result<Report, String> {
    let refused = refusals_at(root, node, map);
    if !refused.is_empty() {
        return Err(refused.join("\n"));
    }
    let source = read_source(root, node)?;
    let rows = spec::rows(&source);
    let mv = Move {
        from: node,
        homes: homes_of(&rows, map, node),
    };
    let writes = writes_for(root, &source, &rows, &mv);
    checked(&writes)?;
    // BEFORE the write, or it reads back what the migration just produced
    // and calls the tree's own violations its own.
    let carried = carried(&writes);
    commit(&writes)?;
    let mut out = report(&rows, &mv, &writes);
    out.carried = carried;
    Ok(out)
}

/// Every file this migration would write, and its whole new text.
///
/// The source, each receiver, and then EVERY other node whose citations of
/// a moved row would otherwise dangle (V11) -- a node the move does not
/// write rows into still names those rows by their old owner.
fn writes_for(
    root: &Path,
    source: &str,
    rows: &[spec::Row],
    mv: &Move,
) -> Vec<(PathBuf, String)> {
    let moved: BTreeSet<String> = mv.moved().into_keys().collect();
    let mut out = vec![(
        dir_of(root, mv.from).join("SPEC.md"),
        mv.rehome(&source_after(source, &moved, mv), mv.from),
    )];
    out.extend(
        destinations(mv)
            .into_iter()
            .map(|node| write_for(root, rows, mv, node)),
    );
    out.extend(bystanders(root, mv, &out));
    out
}

/// Nodes neither giving nor receiving rows whose citations still move.
fn bystanders(
    root: &Path,
    mv: &Move,
    written: &[(PathBuf, String)],
) -> Vec<(PathBuf, String)> {
    fed::discover(root)
        .into_iter()
        .map(|dir| dir.join("SPEC.md"))
        .filter(|p| written.iter().all(|(w, _)| w != p))
        .filter_map(|path| {
            let text = std::fs::read_to_string(&path).ok()?;
            let here = node_name(root, path.parent()?);
            let new = mv.rehome(&text, &here);
            (new != text).then_some((path, new))
        })
        .collect()
}

/// A directory as a citation names its node: `.` for the root.
fn node_name(root: &Path, dir: &Path) -> String {
    let rel = dir.strip_prefix(root).unwrap_or(dir);
    if rel.as_os_str().is_empty() {
        ROOT.into()
    } else {
        rel.to_string_lossy().into_owned()
    }
}

/// One receiving node's file and its whole new text.
fn write_for(
    root: &Path,
    rows: &[spec::Row],
    mv: &Move,
    node: String,
) -> (PathBuf, String) {
    let batch: Vec<&spec::Row> = rows
        .iter()
        .filter(|r| mv.homes.get(&r.id).is_some_and(|h| *h == node))
        .collect();
    let path = root.join(&node).join("SPEC.md");
    let text = std::fs::read_to_string(&path).unwrap_or_default();
    let new = mv.rehome(&receive(&text, &node, &batch, &mv.homes), &node);
    (path, new)
}

/// The nodes receiving rows, the source excluded and each named once.
fn destinations(mv: &Move) -> BTreeSet<String> {
    mv.moved().into_values().collect()
}

/// The source file after the move: moved rows gone, everything that stays
/// requalified so its citations still resolve.
///
/// EVERY line is requalified, not only the row lines. `§G`, `§C` and `§I` are
/// prose that cites rules too, and a migration that fixed the tables while
/// leaving the prose pointing at rows that left would break exactly the links
/// a reader follows first.
fn source_after(source: &str, moved: &BTreeSet<String>, mv: &Move) -> String {
    let kept: Vec<String> = source
        .lines()
        .filter(|l| !opens_a_moved_row(l, moved))
        .map(|l| spec::requalify(l, mv.from, &mv.homes))
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

/// Add rows to a section, creating the section in FORMAT's position when
/// the node has none.
fn add_rows(text: &str, letter: char, batch: &[String]) -> String {
    let Some(anchor) = anchor_for(text, letter) else {
        return text.to_string();
    };
    let head = existing_body(text, letter).unwrap_or_else(|| {
        format!("## \u{a7}{}{}", name(letter), header(letter))
    });
    let body = in_id_order(&head, batch);
    let (sec, at) = (letter.to_string(), anchor.to_string());
    spec::upsert_section(text, &sec, body.trim_end(), &at)
}

/// A section body with each received row placed before the first resident
/// row numbered higher (V7). Appending instead put a moved `T3` below a
/// resident `T88`, the order `microlith/V14` rejects -- so V5 refused the
/// very migration that produced it (`B3`). Lines that are not rows (heading,
/// table header, prose) keep their place.
fn in_id_order(head: &str, batch: &[String]) -> String {
    let mut lines: Vec<String> = head.lines().map(str::to_string).collect();
    for row in batch {
        let n = id_number(row).unwrap_or(u64::MAX);
        let at = lines
            .iter()
            .position(|l| id_number(l).is_some_and(|m| m > n))
            .unwrap_or(lines.len());
        lines.insert(at, row.clone());
    }
    lines.join("\n")
}

/// The number of the id a row line opens with -- `T88` → 88 -- or `None`
/// for any line that is not a row.
fn id_number(line: &str) -> Option<u64> {
    let row = spec::rows(line).into_iter().next()?;
    row.id.get(1..)?.parse().ok()
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
    mv: &Move,
    writes: &[(PathBuf, String)],
) -> Report {
    let moved = rows
        .iter()
        .filter(|r| mv.homes.get(&r.id).is_some_and(|h| h != mv.from))
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
#[path = "tests/adopt.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/subtree.rs"]
mod subtree_tests;
