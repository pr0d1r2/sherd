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
#[must_use]
pub fn rule_depth(spec: &str) -> String {
    const KEEP: [&str; 5] =
        ["\u{a7}G", "\u{a7}C", "\u{a7}I", "\u{a7}V", "\u{a7}T"];
    let mut out = String::new();
    let mut keeping = true;
    for line in spec.lines() {
        if line.starts_with("## \u{a7}") {
            keeping = KEEP.iter().any(|k| line.contains(k));
        }
        if keeping {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
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
/// statement (`§V`, `§R`).
#[must_use]
pub fn declares(spec: &str, id: &str) -> bool {
    spec.lines().any(|l| {
        l.strip_prefix(id)
            .is_some_and(|r| r.starts_with('|') || r.starts_with(':'))
    })
}

/// `§T` rows marked done, which `src/fed:V9` says do not belong there.
///
/// A `§T` row states REMAINING work. A finished one reads to a machine as work
/// to do, and it is paid by every chain that descends through the node on
/// every turn. The record of what was finished is the commit trail.
///
/// Returns id and the head of the task text, enough to find the row.
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
        if in_t && id.starts_with('T') {
            out.push(((*id).to_string(), task.chars().take(52).collect()));
        }
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# SPEC\n\n## \u{a7}G GOAL\nthing do thing\n\n## \u{a7}V INVARIANTS\nV1: a ! b\n";

    #[test]
    fn sections_split_on_headers() {
        let s = sections(SAMPLE);
        let [goal, inv] = s.as_slice() else {
            unreachable!("two sections, and the pattern says so")
        };
        assert!(goal.0.contains("GOAL"));
        assert!(inv.1.contains("V1:"));
    }

    #[test]
    fn a_bug_naming_no_invariant_is_unreflected() {
        let s =
            "## \u{a7}B BUGS\nid|date|cause|fix\nB1|d|it broke|we fixed it\n";
        let u = unreflected_bugs(s);
        assert_eq!(u.len(), 1);
        assert_eq!(u.first().map(|b| b.0.clone()), Some("B1".to_string()));
    }

    /// `B2`: three spellings for one citation and nothing rejected any.
    /// Canonical is the node PATH -- `.` for root, `src/tdd` for a node --
    /// and the bare `tdd:B12` form resolved to nothing because there is no
    /// `tdd/` beside the root.
    #[test]
    fn a_citation_carries_the_node_path_it_names() {
        assert_eq!(
            citations("V1: see `src/tdd:B12` and `.:V50`"),
            vec![
                Citation {
                    owner: "src/tdd".into(),
                    id: "B12".into(),
                    line: 1
                },
                Citation {
                    owner: ".".into(),
                    id: "V50".into(),
                    line: 1
                },
            ]
        );
    }

    /// The foreign form uses a SLASH and is not a citation into this tree:
    /// `microlith/V14` names a rule in another repository, which this tree
    /// cannot resolve and must never rewrite.
    #[test]
    fn a_foreign_rule_is_not_a_citation_here() {
        assert!(citations("V1: microlith/V14 says so").is_empty());
        assert!(citations("MEASURED: V17 held").is_empty());
        assert!(citations("no colon here at all").is_empty());
    }

    /// A row opens its line: `|` in a table, `:` in a statement.
    #[test]
    fn a_row_is_declared_by_the_line_it_opens() {
        let s = "V1: a ! b\nT3|.|do the thing|V1\nB7|2026-01-01|cause|fix\n";
        assert!(declares(s, "V1"));
        assert!(declares(s, "T3"));
        assert!(declares(s, "B7"));
        assert!(!declares(s, "V2"), "V1 must not answer for V2");
        assert!(!declares(s, "V"), "a prefix is not an id");
    }

    /// `src/fed:V9`: a `§T` row states REMAINING work. A finished one reads to
    /// a machine as work to do, and rule depth loads `§T` on every descent, so
    /// every chain pays for it every turn.
    #[test]
    fn a_finished_task_is_not_remaining_work() {
        let s = "## \u{a7}T TASKS\nid|status|task|cites\n\
                 T1|x|the thing landed|V1\nT2|.|the other thing|V2\n\
                 T3|~|half of it|V3\n";
        let done = completed_tasks(s);
        assert_eq!(done.len(), 1, "only `x` is finished");
        assert_eq!(done.first().map(|d| d.0.as_str()), Some("T1"));

        // A `§B` row that happens to start with T is not a task.
        let b = "## \u{a7}B BUGS\nid|date|cause|fix\nT9|x|not a task row|f\n";
        assert!(completed_tasks(b).is_empty(), "§T only");
    }

    use std::path::Path;

    /// The defect `B2` names, on the tree that measured it: every citation in
    /// every spec of this repository resolves to a node and a row. Four files
    /// cited `.:V41`, which was never written at root.
    #[test]
    fn every_citation_in_this_repository_resolves() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut dead = Vec::new();
        for node in crate::fed::discover(root) {
            let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
                continue;
            };
            for c in citations(&text) {
                let target = if c.owner == "." {
                    root.join("SPEC.md")
                } else {
                    root.join(&c.owner).join("SPEC.md")
                };
                match std::fs::read_to_string(&target) {
                    Ok(t) if declares(&t, &c.id) => {}
                    _ => dead.push(format!("{}:{}", c.owner, c.id)),
                }
            }
        }
        assert!(dead.is_empty(), "citations resolving to nothing: {dead:?}");
    }

    #[test]
    fn a_bug_citing_an_invariant_is_reflected() {
        for fix in ["now V9 catches it", "see `src/fed:V9`"] {
            let s = "## \u{a7}B BUGS\nid|date|cause|fix\nB1|d|broke|"
                .to_string()
                + fix
                + "\n";
            assert!(
                unreflected_bugs(&s).is_empty(),
                "should be reflected: {fix}"
            );
        }
    }

    #[test]
    fn only_the_bugs_section_is_read() {
        let s =
            "## \u{a7}T TASKS\nid|status|task|cites\nB1|.|not a bug row|-\n";
        assert!(unreflected_bugs(s).is_empty(), "a §T row is not a §B row");
    }

    /// V5. B1 imported through microlith's inner `violation` module -- which
    /// the dep later made `pub(crate)`, turning a green HEAD red with no commit
    /// here. Only the root re-exports are the contract, so the shape to forbid
    /// is any path BELOW the crate, not the one symbol that happened to move.
    /// The check reads text, so it fails on a mention in prose too: write the
    /// module name, not a path a reader could copy.
    #[test]
    fn microlith_is_reached_only_through_its_root() {
        // Built at runtime: a literal here would match itself.
        let deep = "microlith".to_string() + "::";
        let mut offenders = Vec::new();
        let mut stack =
            vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let Ok(text) = std::fs::read_to_string(&p) else {
                        continue;
                    };
                    for (n, line) in text.lines().enumerate() {
                        let Some(rest) = line.split_once(&deep).map(|(_, r)| r)
                        else {
                            continue;
                        };
                        let ident: String = rest
                            .chars()
                            .take_while(|c| c.is_alphanumeric() || *c == '_')
                            .collect();
                        if rest[ident.len()..].starts_with("::") {
                            offenders.push(format!(
                                "{}:{}: {ident}",
                                p.display(),
                                n + 1
                            ));
                        }
                    }
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "V5: reach microlith through its root re-exports, not {offenders:?}"
        );
    }

    #[test]
    fn fmt_is_lossless_and_idempotent() {
        // `fmt` is the write half of the spec binding and had no test at all.
        // Idempotence is the property that matters: a formatter whose second
        // pass differs from its first turns every `sherd` run into a diff, and
        // the gate would then fail on a tree nobody edited.
        let once = fmt(SAMPLE).unwrap_or_default();
        assert!(!once.is_empty(), "a valid spec formats to something");
        let twice = fmt(&once).unwrap_or_default();
        assert_eq!(once, twice, "formatting twice must change nothing");
    }

    #[test]
    fn check_runs_against_microlith() {
        // Not asserting a specific verdict -- asserting the binding works and
        // returns microlith's own Violation type.
        let _: Vec<Violation> = check(SAMPLE);
    }
}

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
mod scaffold_tests {
    use super::*;

    /// The output is the first thing `check` must accept, because a
    /// generator whose output its own checker rejects has shipped a second
    /// dialect (`.:V27` dogfooding, one verb over).
    #[test]
    fn a_scaffold_passes_the_checker_that_gates_every_other_spec() {
        let out = scaffold("src/lens", &["deep".to_string()]);
        assert!(
            check(&out).is_empty(),
            "our own checker rejects our own scaffold: {:?}",
            check(&out)
        );
    }

    #[test]
    fn a_childless_directory_gets_no_federation_table() {
        let out = scaffold("src/leaf", &[]);
        assert!(!out.contains("\u{a7}F"));
        assert!(check(&out).is_empty());
    }

    /// Ids are monotonic and never reused, so a placeholder id is permanent.
    /// The scaffold emits section headers and a table header, and not one
    /// `V1`, `T1` or `B1`.
    #[test]
    fn a_scaffold_carries_no_ids_at_all() {
        let out = scaffold("src/x", &["a".to_string(), "b".to_string()]);
        for line in out.lines() {
            assert!(
                !line.starts_with("V1")
                    && !line.starts_with("T1")
                    && !line.starts_with("B1"),
                "a seeded id is permanent: {line}"
            );
        }
    }

    #[test]
    fn every_child_gets_a_row_naming_what_it_does_not_own() {
        let out = scaffold("src", &["fed".to_string(), "lens".to_string()]);
        assert!(out.contains("fed|WHAT IT OWNS|WHAT IT DOES \u{22a5} OWN"));
        assert!(out.contains("lens|WHAT IT OWNS|WHAT IT DOES \u{22a5} OWN"));
    }
}

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
        kept.push(format!("{heading}\n{}", sec_body.trim_end()));
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
mod upsert_tests {
    use super::*;

    const DOC: &str = "# SPEC\n\n## \u{a7}G GOAL\n\ng\n\n## \u{a7}F FEDERATION\n\nf\n\n## \u{a7}V INVARIANTS\n\nv\n";

    #[test]
    fn an_absent_section_is_inserted_after_its_anchor() {
        let out = upsert_section(DOC, "N NAV", "## \u{a7}N NAV\n\nn\n", "F");
        assert!(out.contains("## \u{a7}N NAV"));
        let n = out.find("\u{a7}N").unwrap_or(0);
        let f = out.find("\u{a7}F").unwrap_or(0);
        let v = out.find("\u{a7}V").unwrap_or(0);
        assert!(f < n && n < v, "§N sits between §F and §V:\n{out}");
    }

    #[test]
    fn an_existing_section_is_replaced_and_nothing_else_moves() {
        let once = upsert_section(DOC, "N NAV", "## \u{a7}N NAV\n\nold\n", "F");
        let twice =
            upsert_section(&once, "N NAV", "## \u{a7}N NAV\n\nnew\n", "F");
        assert!(twice.contains("new"));
        assert!(!twice.contains("old"));
        assert_eq!(twice.matches("\u{a7}N NAV").count(), 1, "one section only");
        assert!(twice.contains("## \u{a7}G GOAL\n\ng\n"), "§G untouched");
        assert!(
            twice.contains("## \u{a7}V INVARIANTS\n\nv\n"),
            "§V untouched"
        );
    }

    /// Writing twice changes nothing the second time, which is what lets
    /// `sync` report "wrote" honestly.
    #[test]
    fn upserting_the_same_body_is_idempotent() {
        let once = upsert_section(DOC, "N NAV", "## \u{a7}N NAV\n\nn\n", "F");
        let twice =
            upsert_section(&once, "N NAV", "## \u{a7}N NAV\n\nn\n", "F");
        assert_eq!(once, twice);
    }

    #[test]
    fn a_document_without_the_anchor_is_returned_unchanged() {
        let doc = "# SPEC\n\n## \u{a7}G GOAL\n\ng\n";
        assert_eq!(upsert_section(doc, "N NAV", "x", "F"), doc);
    }
}
