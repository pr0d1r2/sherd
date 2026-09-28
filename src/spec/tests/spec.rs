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
    let s = "## \u{a7}B BUGS\nid|date|cause|fix\nB1|d|it broke|we fixed it\n";
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

/// V8. `§C` and `§I` are bullet lists in FORMAT.md, so an id there sits
/// behind `- `. `citation_at` already accepts `C` and `I`; without this a
/// `node:C1` citation could be written and never resolve (`B4`).
#[test]
fn a_bullet_declares_the_id_it_opens_with() {
    let s = "## \u{a7}C CONSTRAINTS\n- C1: Rust only\n- plain bullet\n\
                 ## \u{a7}I INTERFACES\n- I2: `sherd check` exits 1\n";
    assert!(declares(s, "C1"));
    assert!(declares(s, "I2"));
    assert!(!declares(s, "C2"), "C1 must not answer for C2");
    assert!(!declares("- see C1: for why\n", "C3"), "prose is not a row");
    assert!(!declares("text - C1: mid-line\n", "C1"), "opens the line");
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
        let s = "## \u{a7}B BUGS\nid|date|cause|fix\nB1|d|broke|".to_string()
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
    let s = "## \u{a7}T TASKS\nid|status|task|cites\nB1|.|not a bug row|-\n";
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
