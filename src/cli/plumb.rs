//! `--format json` for `budget`, `check` and `validate` -- the PLUMBING form
//! of the three reports, as `src/plan:V25` defines it for `plan` (V19).
//!
//! One object per run, fixed keys, paths ROOT-RELATIVE with the root spelled
//! `.`. The text form is porcelain and may change; these keys may only grow.
//! Both forms render the SAME collected report, so a number cannot appear in
//! one and be missing from the other.

use super::*;
use crate::plan::{json_node, json_str};

/// One line a judging verb reports, kept in the coordinates it prints.
///
/// `rule` is `None` where the text form names no rule today (an edge that
/// skips a level, a chain over its ceiling): the plumbing says so with
/// `null` rather than inventing an id the porcelain never printed.
#[derive(Debug)]
pub(super) struct Finding {
    pub path: PathBuf,
    pub line: Option<usize>,
    pub rule: Option<String>,
    pub message: String,
    pub fatal: bool,
}

impl Finding {
    pub(super) fn new(path: &Path, rule: &str, message: String) -> Self {
        Self {
            path: path.to_path_buf(),
            line: None,
            rule: Some(rule.to_string()),
            message,
            fatal: true,
        }
    }

    pub(super) fn at(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    pub(super) fn advisory(mut self) -> Self {
        self.fatal = false;
        self
    }

    /// A finding the text form prints with NO rule label.
    pub(super) fn bare(path: &Path, message: String) -> Self {
        Self {
            rule: None,
            ..Self::new(path, "", message)
        }
    }

    /// The line the text form prints: `path[:line]: [rule: ]message`.
    pub(super) fn text(&self) -> String {
        let at = self.line.map_or(String::new(), |l| format!(":{l}"));
        let rule = self
            .rule
            .as_ref()
            .map_or(String::new(), |r| format!("{r}: "));
        format!("{}{at}: {rule}{}", self.path.display(), self.message)
    }

    pub(super) fn json(&self, root: &Path) -> String {
        format!(
            "{{\"file\":{},\"line\":{},\"rule\":{},\"message\":{},\"fatal\":{}}}",
            json_node(self.path.strip_prefix(root).unwrap_or(&self.path)),
            self.line
                .map_or_else(|| "null".to_string(), |l| l.to_string()),
            self.rule
                .as_deref()
                .map_or_else(|| "null".to_string(), json_str),
            json_str(&self.message),
            self.fatal
        )
    }
}

/// Print each finding's text line and count the FATAL ones.
pub(super) fn print_findings(fs: &[Finding]) -> usize {
    for f in fs {
        println!("{}", f.text());
    }
    fatal(fs)
}

pub(super) fn fatal(fs: &[Finding]) -> usize {
    fs.iter().filter(|f| f.fatal).count()
}

pub(super) fn findings_json(root: &Path, fs: &[Finding]) -> String {
    let all: Vec<String> = fs.iter().map(|f| f.json(root)).collect();
    format!("[{}]", all.join(","))
}

/// A node's path as the plumbing names it: root-relative, root as `.`.
pub(super) fn node_json(root: &Path, node: &Path) -> String {
    json_node(node.strip_prefix(root).unwrap_or(node))
}

/// `budget --format json`. `unmeasured` is a LIST and never absent: a node
/// whose pack or ceiling could not be read is named there, and an empty
/// `nodes` with nothing unmeasured is a dir that matched no node -- the
/// caller exits 2 on it either way (`.:V48`).
pub(super) fn budget_json(root: &Path, b: &Budget) -> String {
    let rows: Vec<String> = b
        .rows
        .iter()
        .map(|r| {
            format!(
                "{{\"node\":{},\"chain_tokens\":{},\"own_tokens\":{},\
                 \"chain_nodes\":{},\"ceiling\":{},\"over_by\":{}}}",
                node_json(root, &r.node),
                r.chain_tokens,
                r.own_tokens,
                r.chain_nodes,
                r.ceiling,
                r.over_by
                    .map_or_else(|| "null".to_string(), |n| n.to_string())
            )
        })
        .collect();
    let unmeasured: Vec<String> = b
        .unmeasured
        .iter()
        .map(|(node, e)| {
            format!(
                "{{\"node\":{},\"error\":{}}}",
                node_json(root, node),
                json_str(e)
            )
        })
        .collect();
    format!(
        "{{\"version\":{},\"ok\":{},\"window\":{WINDOW},\"entry\":{},\"working\":{},\
         \"method\":{},\"nodes_examined\":{},\"all_chains_tokens\":{},\"over\":{},\
         \"cold\":{},\"nodes\":[{}],\"unmeasured\":[{}]}}",
        json_str(VERSION),
        b.code() == 0,
        tokens::ENTRY_COST,
        tokens::working(WINDOW),
        b.method.map_or_else(|| "null".to_string(), json_str),
        b.rows.len(),
        b.total,
        b.over,
        b.cold,
        rows.join(","),
        unmeasured.join(",")
    )
}

/// `check --format json`: every finding, fatal or advisory, and the counts
/// the text form closes with.
pub(super) fn check_json(root: &Path, c: &CheckReport) -> String {
    format!(
        "{{\"version\":{},\"ok\":{},\"nodes_examined\":{},\"rs_files_measured\":{},\
         \"violations\":{},\"findings\":{},\"file_ceilings\":{}}}",
        json_str(VERSION),
        fatal(&c.findings) == 0,
        c.nodes,
        c.rs_files,
        fatal(&c.findings),
        findings_json(root, &c.findings),
        findings_json(root, &c.files)
    )
}

/// `validate --format json`: the four families it composes, each with its
/// findings, and `slices` saying whether a registry was ABSENT (legal,
/// `src/cli:V12`), READ, or UNREADABLE (a failure, never zero).
pub(super) fn validate_json(root: &Path, v: &ValidateReport) -> String {
    let (slices, slice_error) = match &v.slices {
        Slices::Absent => ("absent", "null".to_string()),
        Slices::Read(_) => ("read", "null".to_string()),
        Slices::Unreadable(e) => ("unreadable", json_str(e)),
    };
    let drifted: Vec<String> = match &v.slices {
        Slices::Read(d) => d.iter().map(|p| node_json(root, p)).collect(),
        Slices::Absent | Slices::Unreadable(_) => Vec::new(),
    };
    format!(
        "{{\"version\":{},\"ok\":{},\"nodes_examined\":{},\
         \"structural\":{},\"edges\":{},\"over_ceiling\":{},\
         \"ceilings_cold\":{},\"slices\":{},\"slice_error\":{},\
         \"drifted\":[{}]}}",
        json_str(VERSION),
        v.failures() == 0,
        v.nodes,
        findings_json(root, &v.structural),
        findings_json(root, &v.edges),
        findings_json(root, &v.over),
        v.cold,
        json_str(slices),
        slice_error,
        drifted.join(",")
    )
}

#[cfg(test)]
#[path = "tests/plumb.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/plumb_trees.rs"]
mod trees;
