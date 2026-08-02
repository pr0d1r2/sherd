//! `SPEC.md` structure. **Sole call site for `cavespec`** (V72).
//!
//! R3: cavespec owns intra-file spec ops as zero-dep pure functions. This
//! module does not reimplement parse, fmt, id or citation checking -- it
//! adapts them. What blackbox adds (`§F`, `§N`) lives in [`crate::fed`].

pub use cavespec::violation::Violation;

/// Structural check of one spec file: sections ordered, ids unique,
/// citations resolve, rows sorted, statuses valid.
#[must_use]
pub fn check(text: &str) -> Vec<Violation> {
    cavespec::check_spec(text, &[])
}

/// Lossless, idempotent reformat -- one line per statement.
///
/// # Errors
/// Returns the cavespec diagnostic when the text cannot be formatted
/// losslessly (e.g. a line over the cap).
pub fn fmt(text: &str) -> Result<String, String> {
    cavespec::format_spec(text)
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
        let names_invariant = fix.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|w| w.len() > 1 && w.starts_with('V') && w[1..].chars().all(|d| d.is_ascii_digit()));
        if !names_invariant {
            out.push((cells[0].to_string(), cells[2].chars().take(58).collect()));
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
        let s = "## \u{a7}B BUGS\nid|date|cause|fix\nB1|d|it broke|we fixed it\n";
        let u = unreflected_bugs(s);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].0, "B1");
    }

    #[test]
    fn a_bug_citing_an_invariant_is_reflected() {
        for fix in ["now V9 catches it", "see `src/fed:V9`"] {
            let s = "## \u{a7}B BUGS\nid|date|cause|fix\nB1|d|broke|".to_string() + fix + "\n";
            assert!(unreflected_bugs(&s).is_empty(), "should be reflected: {fix}");
        }
    }

    #[test]
    fn only_the_bugs_section_is_read() {
        let s = "## \u{a7}T TASKS\nid|status|task|cites\nB1|.|not a bug row|-\n";
        assert!(unreflected_bugs(s).is_empty(), "a §T row is not a §B row");
    }

    #[test]
    fn check_runs_against_cavespec() {
        // Not asserting a specific verdict -- asserting the binding works and
        // returns cavespec's own Violation type.
        let _: Vec<Violation> = check(SAMPLE);
    }
}
