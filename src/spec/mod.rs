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
    fn check_runs_against_cavespec() {
        // Not asserting a specific verdict -- asserting the binding works and
        // returns cavespec's own Violation type.
        let _: Vec<Violation> = check(SAMPLE);
    }
}
