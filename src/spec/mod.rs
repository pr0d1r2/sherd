//! `SPEC.md` structure. **Sole call site for `microlith`** (V72).
//!
//! R3: microlith owns intra-file spec ops as zero-dep pure functions. This
//! module does not reimplement parse, fmt, id or citation checking -- it
//! adapts them. What blackbox adds (`§F`, `§N`) lives in [`crate::fed`].

/// A microlith rule violation. Its `Display` is `microlith/V13: msg` -- the
/// namespace ALREADY QUALIFIED.
///
/// `bbx check` once spelled that prefix as a literal `"cavespec/"`, so renaming
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
        if cells.len() < 4 || !cells[0].starts_with('B') || cells[0] == "id" {
            continue;
        }
        let fix = cells[3..].join("|");
        let names_invariant =
            fix.split(|c: char| !c.is_ascii_alphanumeric()).any(|w| {
                w.len() > 1
                    && w.starts_with('V')
                    && w[1..].chars().all(|d| d.is_ascii_digit())
            });
        if !names_invariant {
            out.push((
                cells[0].to_string(),
                cells[2].chars().take(58).collect(),
            ));
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
        assert_eq!(s.len(), 2);
        assert!(s[0].0.contains("GOAL"));
        assert!(s[1].1.contains("V1:"));
    }

    #[test]
    fn a_bug_naming_no_invariant_is_unreflected() {
        let s =
            "## \u{a7}B BUGS\nid|date|cause|fix\nB1|d|it broke|we fixed it\n";
        let u = unreflected_bugs(s);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].0, "B1");
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
        // pass differs from its first turns every `bbx` run into a diff, and
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
