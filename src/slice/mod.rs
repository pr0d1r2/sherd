//! Distil a big document into the part a reader needs to ACT.
//!
//! Not a summary. A summary compresses everything proportionally and leaves
//! the reader unable to tell what is authoritative. A slice takes 100% of
//! what is load-bearing and 0% of the rest -- lossless for its purpose, empty
//! elsewhere.
//!
//! GENERATED, never hand-written. `notation.txt` was extracted from
//! `FORMAT.md` once by hand and nothing would notice if the source moved
//! (B1). Same argument as `bbx graph`: one source, many renderings, none
//! hand-maintained.

use std::path::{Path, PathBuf};

/// How to take the load-bearing part out of a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    /// First `n` paragraphs. A principle states its rule first and justifies
    /// it after; the rule is what a writer of code needs.
    Lead(usize),
    /// A markdown section by heading text, heading excluded.
    Section(String),
    /// The first fenced block after a line containing this text.
    Fence(String),
    /// Lines starting with this prefix -- for pulling rows out of a table.
    Prefix(String),
}

impl Rule {
    /// # Errors
    /// Unknown rule name, or a missing argument.
    pub fn parse(s: &str) -> Result<Self, String> {
        match s.split_once(':') {
            Some(("lead", n)) => n.parse().map(Rule::Lead)
                .map_err(|_| format!("lead: expected a number, got `{n}`")),
            Some(("section", h)) => Ok(Rule::Section(h.into())),
            Some(("fence", a)) => Ok(Rule::Fence(a.into())),
            Some(("prefix", p)) => Ok(Rule::Prefix(p.into())),
            _ => Err(format!("unknown rule `{s}` -- want lead:N, section:X, fence:X or prefix:X")),
        }
    }

    /// Apply to one document. Returns empty when the rule matches nothing,
    /// which the caller reports rather than silently accepting (V3).
    #[must_use]
    pub fn apply(&self, text: &str) -> String {
        match self {
            Rule::Lead(n) => text.split("\n\n")
                .filter(|p| !p.trim().is_empty() && !p.trim_start().starts_with('#'))
                .take(*n).map(str::trim).collect::<Vec<_>>().join("\n\n"),
            Rule::Section(h) => {
                let mut out = Vec::new();
                let mut inside = false;
                for line in text.lines() {
                    if line.starts_with('#') {
                        if inside { break }
                        inside = line.contains(h.as_str());
                        continue;
                    }
                    if inside { out.push(line) }
                }
                out.join("\n").trim().to_string()
            }
            Rule::Fence(after) => {
                let Some(pos) = text.find(after.as_str()) else { return String::new() };
                let rest = &text[pos..];
                let Some(open) = rest.find("```") else { return String::new() };
                let body = &rest[open + 3..];
                let start = body.find('\n').map_or(0, |i| i + 1);
                body[start..].find("```").map_or_else(String::new, |end| body[start..start + end].trim_end().to_string())
            }
            Rule::Prefix(p) => text.lines().filter(|l| l.trim_start().starts_with(p.as_str()))
                .collect::<Vec<_>>().join("\n"),
        }
    }
}

/// One declared slice: where it goes, what it reads, how it distils.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decl {
    pub output: PathBuf,
    pub source: String,
    pub rule: Rule,
}

/// Parse `.bbx-slices`. Line-oriented like `.context-limits` and
/// `.spec-records` -- one format family, not four.
///
/// # Errors
/// A malformed line, named and numbered. Never skipped: a skipped
/// declaration means a slice that silently stops being generated.
pub fn parse_decls(text: &str) -> Result<Vec<Decl>, String> {
    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = l.split_whitespace().collect();
        if f.len() != 3 {
            return Err(format!(".bbx-slices:{}: expected `<output> <source> <rule>`", n + 1));
        }
        out.push(Decl {
            output: PathBuf::from(f[0]),
            source: f[1].to_string(),
            rule: Rule::parse(f[2]).map_err(|e| format!(".bbx-slices:{}: {e}", n + 1))?,
        });
    }
    Ok(out)
}

/// Expand a source pattern to files. Supports one trailing `*` segment; a
/// path with no glob is itself.
#[must_use]
/// Every declared slice whose file on disk differs from what its source
/// renders to now.
///
/// One definition, called by `bbx slice --check` AND by the gate. The hook
/// and the loop must apply the same rule; two implementations of one check is
/// the defect this project exists to end (`.:V50`).
///
/// # Errors
/// A missing or malformed `.bbx-slices`, or a source that cannot be rendered.
pub fn drifted(root: &Path) -> Result<Vec<PathBuf>, String> {
    let text = std::fs::read_to_string(root.join(".bbx-slices"))
        .map_err(|e| format!(".bbx-slices: {e}"))?;
    let mut out = Vec::new();
    for d in parse_decls(&text)? {
        let rendered = render(root, &d)?;
        if std::fs::read_to_string(root.join(&d.output)).unwrap_or_default() != rendered {
            out.push(d.output.clone());
        }
    }
    Ok(out)
}

pub fn sources(root: &Path, pattern: &str) -> Vec<PathBuf> {
    let p = if Path::new(pattern).is_absolute() { PathBuf::from(pattern) } else { root.join(pattern) };
    let s = p.to_string_lossy().to_string();
    let Some((dir, pat)) = s.rsplit_once('/') else { return vec![p] };
    if !pat.contains('*') {
        return vec![p];
    }
    let suffix = pat.trim_start_matches('*');
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut v: Vec<PathBuf> = rd.flatten().map(|e| e.path())
        .filter(|q| q.is_file() && q.to_string_lossy().ends_with(suffix))
        .collect();
    v.sort();
    v
}

/// Render one declaration: the header, then each source's distilled part.
///
/// # Errors
/// A source that cannot be read, or a rule that matches nothing anywhere --
/// an empty slice is a silent failure and must be loud (V3).
pub fn render(root: &Path, d: &Decl) -> Result<String, String> {
    let files = sources(root, &d.source);
    if files.is_empty() {
        return Err(format!("{}: `{}` matched no files", d.output.display(), d.source));
    }
    let mut parts = Vec::new();
    for f in &files {
        let text = std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let got = d.rule.apply(&text);
        if !got.trim().is_empty() {
            parts.push(got);
        }
    }
    if parts.is_empty() {
        return Err(format!("{}: rule {:?} matched nothing in {} file(s) -- \
                            an empty slice is a silent failure",
                           d.output.display(), d.rule, files.len()));
    }
    Ok(format!("# GENERATED by `bbx slice` from {} -- do not edit.\n\n{}\n",
               d.source, parts.join("\n\n")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lead_takes_the_rule_and_drops_the_justification() {
        let doc = "# KISS\n\nKeep it simple.\n\nBecause complexity costs.\n\nAlso this.\n";
        assert_eq!(Rule::Lead(1).apply(doc), "Keep it simple.");
    }

    #[test]
    fn lead_skips_the_heading_rather_than_counting_it() {
        // A heading is not a paragraph; counting it would return the title.
        let doc = "# T\n\nrule here\n";
        assert_eq!(Rule::Lead(1).apply(doc), "rule here");
    }

    #[test]
    fn fence_takes_the_block_after_its_anchor() {
        let doc = "intro\n\n**Symbols**\n\n```\n! must\n⊥ never\n```\n\nafter\n";
        assert_eq!(Rule::Fence("Symbols".into()).apply(doc), "! must\n⊥ never");
    }

    #[test]
    fn section_stops_at_the_next_heading() {
        let doc = "# A\n\nalpha\n\n# B\n\nbeta\n";
        assert_eq!(Rule::Section("A".into()).apply(doc), "alpha");
    }

    #[test]
    fn prefix_pulls_table_rows() {
        let doc = "id|x\nB1|one\nnoise\nB2|two\n";
        assert_eq!(Rule::Prefix("B".into()).apply(doc), "B1|one\nB2|two");
    }

    #[test]
    fn a_malformed_declaration_is_an_error_not_a_skip() {
        let e = parse_decls("out.txt only-two-fields\n").unwrap_err();
        assert!(e.contains(":1:"), "must name the line: {e}");
    }

    #[test]
    fn comments_and_blanks_are_skipped() {
        let d = parse_decls("# c\n\na.txt b.md lead:2\n").unwrap();
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].rule, Rule::Lead(2));
    }

    #[test]
    fn an_unknown_rule_names_the_alternatives() {
        let e = Rule::parse("summarise:3").unwrap_err();
        assert!(e.contains("lead:N"), "{e}");
    }
}
