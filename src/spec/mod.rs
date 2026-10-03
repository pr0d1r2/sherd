//! `SPEC.md` structure. **Sole call site for `microlith`** (V72).
//!
//! R3: microlith owns intra-file spec ops as zero-dep pure functions. This
//! module does not reimplement parse, fmt, id or citation checking -- it
//! adapts them. What sherd adds (`§F`, `§N`) lives in [`crate::fed`].

/// A microlith rule violation. Its `Display` is `microlith/V13: msg` -- the
/// namespace ALREADY QUALIFIED.
///
/// `sherd check` once spelled that prefix as a literal `"cavespec/"`, so renaming
/// the crate left every violation line naming a crate that no longer exists.
/// The name is not re-exported to fix that: the published crate keeps
/// `violation` private, and printing the `Violation` itself is the reading that
/// cannot drift -- one owner of the qualified id, not two.
pub use microlith::Violation;

/// Structural check of one spec file: sections ordered, ids unique,
/// citations resolve, rows sorted, statuses valid.
#[must_use]
pub fn check(text: &str) -> Vec<Violation> {
    microlith::check_spec(text, &[])
}

/// Lossless, idempotent reformat -- one line per statement.
///
/// # Errors
/// Returns the microlith diagnostic when the text cannot be formatted
/// losslessly (e.g. a line over the cap).
pub fn fmt(text: &str) -> Result<String, String> {
    microlith::format_spec(text)
}

/// Each `| M<n> |` row's id and the `§T` numbers it claims, ranges expanded,
/// in file order -- microlith's own reading of the milestone grammar, handed
/// over rather than re-read here (V1; this module is the sole call site, V72).
#[must_use]
pub fn milestones(text: &str) -> Vec<(String, Vec<u32>)> {
    microlith::milestones(text)
}

/// Each `| M<n> |` row's id and its task cell AS WRITTEN -- `T1-T3` stays a
/// range here, where [`milestones`] would have expanded it (V10).
///
/// The pair is the point. What a row CLAIMS is microlith's answer and is
/// never re-derived; what it SAYS is what a message has to quote and what a
/// rewrite has to survive, and the two differ exactly when a range is in
/// play. `src/adopt:B5` is what that difference cost while nothing could see
/// it.
#[must_use]
pub fn milestone_cells(text: &str) -> Vec<(String, String)> {
    text.lines()
        // microlith's own predicate for a milestone row, deliberately the
        // same one: a line this recognised and `milestones` did not would
        // pair an id with a cell from another row.
        .filter(|l| l.starts_with("| M"))
        .map(|l| {
            let cells = microlith::cells(l);
            let at =
                |n: usize| cells.get(n).map(|c| microlith::unescape(c.trim()));
            (at(1).unwrap_or_default(), at(3).unwrap_or_default())
        })
        .collect()
}

/// The RULE sections: what a worker needs to act, without the archive.
///
/// `§G §C §I §V §T` survive; `§R` and `§B` are history and stay out. `§T` is
/// the PLAN rather than the archive -- the row being implemented names the
/// work, and dropping it cost a run (`src/tdd:B9`).
///
/// This lives here because `src/spec` owns `SPEC.md` structure. It spent the
/// project's life in `src/tdd`, reachable only from the worker path, while
/// `lens::pack` -- the thing that builds every context pack and every budget
/// -- shipped whole files including both archive sections (`.:B8`).
///
/// MEASURED share dropped: 33% of root, 59% of `src/tdd`, 58% of `src/plan`.
///
/// Within `§T`, only REMAINING work survives (V11): a row marked `x` is done,
/// and an `ARCHIVED to SPEC-ARCHIVE.md` stub is done with its text already
/// gone. Both are history by `src/fed:V9`'s own reading -- "§T states
/// remaining work" -- and a pack that ships them bills every worker for the
/// project's past. The file keeps them: `check` reads the file, not the pack,
/// so citations to a finished row still resolve.
#[must_use]
pub fn rule_depth(spec: &str) -> String {
    const KEEP: [&str; 5] =
        ["\u{a7}G", "\u{a7}C", "\u{a7}I", "\u{a7}V", "\u{a7}T"];
    let mut out = String::new();
    let (mut keeping, mut tasks) = (true, false);
    for line in spec.lines() {
        if line.starts_with("## \u{a7}") {
            keeping = KEEP.iter().any(|k| line.contains(k));
            tasks = line.contains("\u{a7}T");
        }
        if keeping && !(tasks && finished_row(line)) {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// A `§T` row whose status cell is `x`: `T12|x|...`, suffixed ids included.
fn finished_row(line: &str) -> bool {
    let mut cells = line.split('|');
    let id = cells.next().unwrap_or_default();
    id.starts_with('T') && cells.next() == Some("x")
}

/// Split a spec into `(header, body)` pairs, one per `## §X` section.
#[must_use]
pub fn sections(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cur: Option<(String, String)> = None;
    for line in text.lines() {
        if line.starts_with("## \u{a7}") {
            if let Some(prev) = cur.take() {
                out.push(prev);
            }
            cur = Some((line.trim().to_string(), String::new()));
        } else if let Some((_, body)) = cur.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    out.extend(cur);
    out
}

/// A `§B` row whose fix names no invariant.
///
/// Pain plus reflection is progress; pain alone is just pain. A bug recorded
/// without a rule that would catch its recurrence will recur, and the §B log
/// becomes a list of things that happened rather than a set of guards.
///
/// Cites are recognised in either form: bare `V9` or namespaced
/// `` `src/fed:V9` `` -- the second is what a moved row uses.
#[must_use]
pub fn unreflected_bugs(spec: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut in_b = false;
    for line in spec.lines() {
        if line.starts_with("## \u{a7}") {
            in_b = line.starts_with("## \u{a7}B");
            continue;
        }
        if !in_b {
            continue;
        }
        let cells: Vec<&str> = line.split('|').collect();
        let [id, _date, cause, fix @ ..] = cells.as_slice() else {
            continue;
        };
        if !id.starts_with('B') || *id == "id" {
            continue;
        }
        let fix = fix.join("|");
        let names_invariant =
            fix.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
                w.len() > 1
                    && w.starts_with('V')
                    && w[1..].chars().all(|d| d.is_ascii_digit())
            });
        if !names_invariant {
            out.push(((*id).to_string(), cause.chars().take(58).collect()));
        }
    }
    out
}

/// One `owner:ID` citation and where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// The node path as written -- canonically `.` or a path like `src/tdd`.
    pub owner: String,
    /// The row id, e.g. `V18`.
    pub id: String,
    /// 1-indexed line the citation appears on.
    pub line: usize,
}

/// Every namespaced citation in a spec.
///
/// `.:V10` fixes the form: ids are namespaced by DIR PATH, so `src/plan:V3`
/// and `src/spec:V3` are different rows. Nothing resolved them until `B2`, and
/// four files cited a `.:V41` that was never written.
///
/// The foreign form `microlith/V14` uses a slash and is deliberately not a
/// citation here -- it names a rule in another repository, which this tree
/// cannot resolve and must not rewrite.
#[must_use]
pub fn citations(text: &str) -> Vec<Citation> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        for (at, _) in line.match_indices(':') {
            if let Some((owner, id)) = citation_at(line, at) {
                out.push(Citation {
                    owner,
                    id,
                    line: n.saturating_add(1),
                });
            }
        }
    }
    out
}

/// Expand around one colon: a row id to the right, a node path to the left.
fn citation_at(line: &str, colon: usize) -> Option<(String, String)> {
    let (left, right) = line.split_at(colon);
    let id: String = right
        .get(1..)?
        .chars()
        .take_while(char::is_ascii_alphanumeric)
        .collect();
    let mut rest = id.chars();
    if !matches!(rest.next(), Some(k) if "VBTRIC".contains(k))
        || id.len() < 2
        || !rest.all(|d| d.is_ascii_digit())
    {
        return None;
    }
    let owner: String = left
        .chars()
        .rev()
        .take_while(|c| {
            c.is_ascii_lowercase()
                || c.is_ascii_digit()
                || *c == '_'
                || *c == '/'
                || *c == '.'
        })
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    (!owner.is_empty()).then_some((owner, id))
}

/// Does this spec declare that row id?
///
/// A row opens its line, followed by `|` in a table (`§T`, `§B`) or `:` in a
/// statement (`§V`, `§R`). In a bullet list (`§C`, `§I`) the id opens the
/// bullet instead, behind `- ` (V8).
#[must_use]
pub fn declares(spec: &str, id: &str) -> bool {
    spec.lines().any(|l| {
        l.strip_prefix("- ")
            .unwrap_or(l)
            .strip_prefix(id)
            .is_some_and(|r| r.starts_with('|') || r.starts_with(':'))
    })
}

/// The task text `mth archive` leaves when it moves a finished row to
/// `SPEC-ARCHIVE.md`. microlith keeps its copy private, and the release sherd
/// pins predates it, so the bytes are restated here.
const ARCHIVE_STUB: &str = "ARCHIVED to SPEC-ARCHIVE.md";

/// `§T` rows marked done, which `src/fed:V9` says do not belong there.
///
/// A `§T` row states REMAINING work. A finished one reads to a machine as work
/// to do, and it is paid by every chain that descends through the node on
/// every turn. The record of what was finished is the commit trail.
///
/// Returns id and the head of the task text, enough to find the row.
///
/// An archive stub is skipped: `mth archive` moved its text and left the row
/// so milestones and citations still resolve (`microlith/V48`, `src/fed:B14`).
#[must_use]
pub fn completed_tasks(spec: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut in_t = false;
    for line in spec.lines() {
        if line.starts_with("## \u{a7}") {
            in_t = line.starts_with("## \u{a7}T");
            continue;
        }
        let cells: Vec<&str> = line.split('|').collect();
        let [id, "x", task, ..] = cells.as_slice() else {
            continue;
        };
        if in_t && id.starts_with('T') && task.trim() != ARCHIVE_STUB {
            out.push(((*id).to_string(), task.chars().take(52).collect()));
        }
    }
    out
}
#[cfg(test)]
#[path = "tests/spec.rs"]
mod tests;

/// A `SPEC.md` skeleton for a directory, with `§F` rows for its children.
///
/// EMPTY WITH PROMPTS, and zero ids. A seeded `T1|.|replace me` is `T1`
/// forever -- ids are monotonic and never reused (`.:V74`'s neighbour in
/// FORMAT) -- and a `T1` citing a placeholder `V1` is a spec that lies from
/// its first commit. Prompts are prose a human replaces; an id is a promise
/// nothing can take back.
///
/// NO INFERENCE. `§G` is never guessed from the directory name, and an `§F`
/// row's `owns`/`⊥owns` cells are prompts rather than a model's guess at what
/// a directory is for -- §C keeps the deterministic core away from the model,
/// and a scaffold that invents ownership is exactly the drift `sherd check`
/// exists to catch.
///
/// FOUR SECTIONS, not seven. Absence is legal in FORMAT, an empty `§B`
/// asserts nothing, and `§R` needs research nobody has done at scaffold time.
/// What is here is what a node cannot be a node without: a goal, its
/// children, its rules, and its remaining work.
/// The `§F` table, or nothing.
///
/// NOTHING when there are no children, because an empty table is a CLAIM of
/// no children rather than an absence of information, and a node that grows
/// one later would have to notice the difference.
fn federation_table(children: &[String]) -> String {
    if children.is_empty() {
        return String::new();
    }
    let mut s =
        String::from("## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n");
    for c in children {
        s.push_str(&format!(
            "{c}|WHAT IT OWNS|WHAT IT DOES \u{22a5} OWN, & WHERE THAT LIVES|-\n"
        ));
    }
    s.push('\n');
    s
}

#[must_use]
pub fn scaffold(dir_name: &str, children: &[String]) -> String {
    let mut s = String::from("# SPEC\n\n## \u{a7}G GOAL\n\n");
    s.push_str(&format!(
        "WHAT `{dir_name}` OWNS, in one sentence. Delete this line.\n\n"
    ));
    s.push_str(&federation_table(children));
    s.push_str(
        "## \u{a7}V INVARIANTS\n\n\
         WHAT MUST STAY TRUE HERE, one line each, numbered from the first id. Delete this line.\n\n\
         ## \u{a7}T TASKS\n\n\
         id|status|task|cites\n",
    );
    s
}

#[cfg(test)]
#[path = "tests/scaffold.rs"]
mod scaffold_tests;

/// Replace a `§`-section's body, or insert the section after `after`.
///
/// GENERATED sections need one writer, and this is it: `sync` must be able to
/// rewrite `§N` without touching a byte of anything else, including on a file
/// that has no `§N` yet. Insertion goes after the named section rather than
/// at the end, because FORMAT fixes the order and a section appended below
/// `§B` is in the wrong place the moment it is written.
///
/// Rebuilt from the SECTION LIST rather than by splicing strings, and the
/// reason is `.:src/cli:B4`: the splice version appended one newline per run, so
/// `sync` was never idempotent and every commit grew the file. Rendering
/// every section with exactly one blank line between them makes a second run
/// a no-op BY CONSTRUCTION rather than by careful string handling.
///
/// Returns the text unchanged when `after` is absent, since a document
/// without the anchor has not opted in and guessing a position would be an
/// edit nobody asked for.
#[must_use]
pub fn upsert_section(
    text: &str,
    name: &str,
    body: &str,
    after: &str,
) -> String {
    let head = format!("## \u{a7}{name}");
    let anchor = format!("## \u{a7}{after}");
    let preamble: String = text
        .lines()
        .take_while(|l| !l.starts_with("## \u{a7}"))
        .map(|l| format!("{l}\n"))
        .collect();

    // REPLACE wins over insert, and the order matters: the anchor sits
    // BEFORE the section in a well-formed document, so an insert-first loop
    // adds a second copy every run rather than rewriting the first.
    let exists = sections(text).iter().any(|(h, _)| h.starts_with(&head));
    let mut kept: Vec<String> = Vec::new();
    let mut placed = false;
    for (heading, sec_body) in sections(text) {
        if exists && heading.starts_with(&head) {
            kept.push(body.trim_end().to_string());
            placed = true;
            continue;
        }
        // Heading and body trimmed AS ONE (V12): an empty body trims to
        // nothing, and a heading keeping its own newline would put a second
        // blank line under it once the join adds the separator.
        let section = format!("{heading}\n{sec_body}");
        kept.push(section.trim_end().to_string());
        if !exists && heading.starts_with(&anchor) {
            kept.push(body.trim_end().to_string());
            placed = true;
        }
    }
    if !placed {
        return text.to_string();
    }
    format!("{}{}\n", preamble.trim_end_matches('\n'), {
        let joined = kept.join("\n\n");
        format!("\n\n{joined}")
    })
}

#[cfg(test)]
#[path = "tests/upsert.rs"]
mod upsert_tests;

/// One addressable row of a spec: the section it sits in, its id, the whole
/// line, and where that line is.
///
/// Adoption has to NAME a row before it can move one, and the text is carried
/// verbatim because a row is MOVED, never retyped (`src/adopt:V1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The section letter: `V`, `T`, `B` or `R`.
    pub section: char,
    /// The id as written, e.g. `V9`.
    pub id: String,
    /// The whole line, byte for byte.
    pub text: String,
    /// 1-indexed line in the source document.
    pub line: usize,
}

/// A line that LOOKS like a row and is not one this grammar reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unreadable {
    /// 1-indexed line in the source document.
    pub line: usize,
    /// The line, byte for byte.
    pub text: String,
}

/// Lines carrying an id in a form [`rows`] does not accept (V9).
///
/// The bracketed markdown table -- `| T1 | . | first task | - |` -- is the
/// one a stranger writes, and to [`rows`] it is prose: the id does not open
/// the line, so the file reads as having no rows at all. "I read this and
/// there is nothing" and "I could not read this" then produce the same
/// output and the same exit code, and on a repository being weighed for
/// adoption the first is the opposite of the truth (`src/adopt:B4`).
///
/// The id shape is decided by the same private parser `rows` uses and not by
/// a second reading of it:
/// the first cell is handed to the same parser with the terminator it would
/// have had. So `M1` in a bracketed milestone table is NOT reported -- this
/// grammar does not own `M`, `microlith::milestones` reads that table, and
/// flagging it would refuse every spec that keeps one.
#[must_use]
pub fn unreadable_rows(spec: &str) -> Vec<Unreadable> {
    spec.lines()
        .enumerate()
        .filter(|(_, line)| row_id(line).is_none())
        .filter(|(_, line)| bracketed_id(line))
        .map(|(n, line)| Unreadable {
            line: n.saturating_add(1),
            text: line.to_string(),
        })
        .collect()
}

/// `| T1 | ... |` -- an id in the first cell of a leading-pipe table row.
///
/// Three cells at least, so a two-column table of prose whose first cell
/// happens to read `T1` is not mistaken for a row, and the separator line
/// (`| --- | --- |`) fails the id test anyway.
fn bracketed_id(line: &str) -> bool {
    let trimmed = line.trim();
    let Some(body) = trimmed.strip_prefix('|') else {
        return false;
    };
    // The CLOSING pipe is punctuation of this dialect, not a column: left on,
    // it yields an empty trailing cell and a two-column table of prose counts
    // as three.
    let body = body.strip_suffix('|').unwrap_or(body);
    let cells: Vec<&str> = microlith::cells(body);
    let Some(first) = cells.first().map(|c| c.trim()) else {
        return false;
    };
    cells.len() >= 3 && row_id(&format!("{first}|")).is_some()
}

/// Every addressable row of a spec, in document order.
///
/// One parser (V1): `declares` already fixed what a row LOOKS like -- an id
/// opening the line, followed by `|` in a table or `:` in a statement -- and
/// a second reading of that shape is the drift this node exists to prevent.
#[must_use]
pub fn rows(spec: &str) -> Vec<Row> {
    let mut out = Vec::new();
    let mut section = ' ';
    for (n, line) in spec.lines().enumerate() {
        if let Some(s) = section_letter(line) {
            section = s;
        } else if let Some(id) = row_id(line) {
            out.push(Row {
                section,
                id,
                text: line.to_string(),
                line: n.saturating_add(1),
            });
        }
    }
    out
}

/// `.:V44`'s two failures for a node that keeps a `SPEC.why.md`, as
/// `(missing, orphan)`: `§V` ids with no why row, and why rows naming no
/// `§V` id. A row of `-` is an answer. Both files are read by [`rows`], so
/// what counts as a row is the one definition this node has (V1).
#[must_use]
pub fn why_gaps(spec: &str, why: &str) -> (Vec<String>, Vec<String>) {
    let rules: Vec<String> = rows(spec)
        .into_iter()
        .filter(|r| r.section == 'V')
        .map(|r| r.id)
        .collect();
    let answered: Vec<String> = rows(why)
        .into_iter()
        .map(|r| r.id)
        .filter(|id| id.starts_with('V'))
        .collect();
    let missing = rules
        .iter()
        .filter(|id| !answered.contains(id))
        .cloned()
        .collect();
    let orphan = answered
        .iter()
        .filter(|id| !rules.contains(id))
        .cloned()
        .collect();
    (missing, orphan)
}

/// The letter of a `## §X NAME` heading, or `None` for any other line.
fn section_letter(line: &str) -> Option<char> {
    line.strip_prefix("## \u{a7}")?.chars().next()
}

/// The id a line OPENS with, by `declares`' rule.
fn row_id(line: &str) -> Option<String> {
    let kind = line.chars().next().filter(|k| "VBTR".contains(*k))?;
    let digits: String = line
        .chars()
        .skip(1)
        .take_while(char::is_ascii_digit)
        .collect();
    let id = format!("{kind}{digits}");
    let rest = line.get(id.len()..).filter(|_| !digits.is_empty())?;
    (rest.starts_with('|') || rest.starts_with(':')).then_some(id)
}

/// Rewrite the bare ids in one row so they still resolve after it MOVES.
///
/// A bare `V9` means "this file". Split the rows across nodes and that stops
/// being true, so a citation whose target landed elsewhere gains that node's
/// path (V7). The NUMBER never changes: every citation already names it, and
/// renumbering would break each one (`src/adopt:V4`).
///
/// Three things are deliberately left alone, and each is a way to get this
/// wrong:
///
/// - the row's OWN id, which opens the line and is a declaration, not a link
/// - an already-namespaced `owner:V9`, which names its node already
/// - the foreign slash form `microlith/V14`, which names another repository
///   this tree cannot resolve and must never rewrite
#[must_use]
pub fn requalify(
    line: &str,
    here: &str,
    homes: &std::collections::BTreeMap<String, String>,
) -> String {
    let mut out = String::new();
    let mut token = String::new();
    let mut prev = '\0';
    for c in line.chars() {
        if c.is_ascii_alphanumeric() {
            token.push(c);
            continue;
        }
        out.push_str(&qualified(&token, prev, here, homes));
        token.clear();
        prev = c;
        out.push(c);
    }
    out.push_str(&qualified(&token, prev, here, homes));
    out
}

/// One token, rewritten if it is a citation that has to travel.
///
/// `prev` is the character immediately before the token, and it carries the
/// whole decision about whether this is a citation at all: `\0` is the start
/// of the line (the row's own id), `:` means already namespaced, `/` means
/// foreign. A backtick means the caller already wrote the quotes, so the
/// replacement goes inside them rather than growing a second pair.
fn qualified(
    token: &str,
    prev: char,
    here: &str,
    homes: &std::collections::BTreeMap<String, String>,
) -> String {
    if matches!(prev, '\0' | ':' | '/')
        || row_id(&format!("{token}:")).is_none()
    {
        return token.to_string();
    }
    match homes.get(token) {
        Some(home) if home != here && prev == '`' => format!("{home}:{token}"),
        Some(home) if home != here => format!("`{home}:{token}`"),
        _ => token.to_string(),
    }
}

#[cfg(test)]
#[path = "tests/row.rs"]
mod row_tests;
