//! The parts of an llvm-cov JSON export a FUNCTION exclusion reads
//! (`V11`): `files[]` summaries, `functions[]` regions, and one function's
//! line count as llvm-cov computes it. Hand-read, as `swift.rs` reads the
//! files, because `serde_json` is optional under `ollama` (root §C).

use super::swift::{after_key, is_source, number_of, string_of};
use std::path::{Path, PathBuf};

/// One llvm-cov region: `[line, col, end line, end col, count, file id,
/// expanded file id, kind]`.
#[derive(Clone, Copy, Debug)]
pub(super) struct Region {
    pub(super) ls: usize,
    pub(super) cs: usize,
    pub(super) le: usize,
    pub(super) ce: usize,
    pub(super) count: usize,
    pub(super) file: usize,
    pub(super) kind: usize,
}

/// `CodeRegion` and `SkippedRegion` in llvm's `CounterMappingRegion`.
pub(super) const CODE: usize = 0;
pub(super) const SKIPPED: usize = 2;

/// One `functions[]` record: its mangled name, file and regions.
pub(super) struct Function {
    pub(super) name: String,
    pub(super) file: String,
    pub(super) regions: Vec<Region>,
}

/// One function's mapped and covered lines, as llvm's `LineCoverageIterator`
/// counts them over the function's own segments: a line is MAPPED when a
/// counted code region starts on it or a counted region wraps into it, and
/// not when its first segment opens a skipped region (an inactive `#if`).
/// Its count is the max of the wrapping count and every region starting
/// on it. Only regions in the function's own file (id 0) count.
pub(super) fn function_lines(all: &[Region]) -> (usize, usize) {
    let regions: Vec<&Region> = all.iter().filter(|r| r.file == 0).collect();
    let lo = regions.iter().map(|r| r.ls).min().unwrap_or(1);
    let hi = regions.iter().map(|r| r.le).max().unwrap_or(0);
    let (mut lines, mut covered) = (0_usize, 0_usize);
    for line in lo..=hi {
        let first = regions
            .iter()
            .filter(|r| r.ls == line)
            .map(|r| (r.cs, r.kind != SKIPPED))
            .chain(
                regions
                    .iter()
                    .filter(|r| r.le == line)
                    .map(|r| (r.ce, true)),
            )
            .min();
        if first.is_some_and(|(_, counted)| !counted) {
            continue;
        }
        let wrapped = regions
            .iter()
            .filter(|r| r.ls < line && line <= r.le)
            .max_by_key(|r| (r.ls, r.cs, std::cmp::Reverse((r.le, r.ce))))
            .filter(|r| r.kind != SKIPPED)
            .map(|r| r.count);
        let starts = regions
            .iter()
            .filter(|r| r.ls == line && r.kind == CODE)
            .map(|r| r.count)
            .max();
        if wrapped.is_none() && starts.is_none() {
            continue;
        }
        lines = lines.saturating_add(1);
        if wrapped.unwrap_or(0).max(starts.unwrap_or(0)) > 0 {
            covered = covered.saturating_add(1);
        }
    }
    (lines, covered)
}

/// `file` relative to the root it is under, or as written when relative.
pub(super) fn relative(
    file: &Path,
    roots: &[Option<PathBuf>],
) -> Option<String> {
    let rel = if file.is_relative() {
        file
    } else {
        roots
            .iter()
            .flatten()
            .find_map(|r| file.strip_prefix(r).ok())?
    };
    Some(rel.to_string_lossy().into_owned())
}

/// `(path, lines, covered)` of every `files[]` entry under `Sources/`.
pub(super) fn source_files(
    json: &str,
    roots: &[Option<PathBuf>],
) -> Option<Vec<(String, usize, usize)>> {
    let mut out = Vec::new();
    for entry in elements(array_of(json, "files")?)? {
        let (name, _) = string_of(after_key(entry, "filename")?)?;
        let file = Path::new(&name);
        if !is_source(file, roots) {
            continue;
        }
        let lines = after_key(after_key(entry, "summary")?, "lines")?;
        let lines = lines.get(..lines.find('}')?)?;
        out.push((
            relative(file, roots)?,
            number_of(lines, "count")?,
            number_of(lines, "covered")?,
        ));
    }
    Some(out)
}

/// Every `functions[]` record whose first file is under a root and
/// `wanted`; only those have their regions read.
pub(super) fn functions(
    json: &str,
    roots: &[Option<PathBuf>],
    wanted: &dyn Fn(&str) -> bool,
) -> Option<Vec<Function>> {
    let mut out = Vec::new();
    for entry in elements(array_of(json, "functions")?)? {
        let first = *elements(after_key(entry, "filenames")?)?.first()?;
        let (file, _) = string_of(first)?;
        let Some(file) = relative(Path::new(&file), roots) else {
            continue;
        };
        if !wanted(&file) {
            continue;
        }
        let (name, _) = string_of(after_key(entry, "name")?)?;
        let mut regions = Vec::new();
        for r in elements(after_key(entry, "regions")?)? {
            let n: Vec<usize> = elements(r)?
                .iter()
                .map(|v| v.trim().parse().ok())
                .collect::<Option<_>>()?;
            regions.push(Region {
                ls: *n.first()?,
                cs: *n.get(1)?,
                le: *n.get(2)?,
                ce: *n.get(3)?,
                count: *n.get(4)?,
                file: *n.get(5)?,
                kind: *n.get(7)?,
            });
        }
        out.push(Function {
            name,
            file,
            regions,
        });
    }
    Some(out)
}

/// The text from the `[` of the first `"key": [` on -- `"functions"` is
/// also a `summary` object's key, which is `{`, so that one is skipped.
pub(super) fn array_of<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let quoted = format!("\"{key}\"");
    let mut rest = json;
    loop {
        let at = rest.find(&quoted)?.checked_add(quoted.len())?;
        rest = rest.get(at..)?;
        let v = rest.trim_start().strip_prefix(':')?.trim_start();
        if v.starts_with('[') {
            return Some(v);
        }
    }
}

/// The top-level elements of the JSON array `v` starts with, as text.
pub(super) fn elements(v: &str) -> Option<Vec<&str>> {
    let body = v.trim_start().strip_prefix('[')?;
    let (mut out, mut depth, mut start) = (Vec::new(), 0_usize, 0_usize);
    let (mut in_str, mut escaped) = (false, false);
    for (i, ch) in body.char_indices() {
        if in_str {
            match (escaped, ch) {
                (true, _) => escaped = false,
                (false, '\\') => escaped = true,
                (false, '"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '[' | '{' => depth = depth.checked_add(1)?,
            ']' | '}' if depth > 0 => depth = depth.checked_sub(1)?,
            ',' | ']' if depth == 0 => {
                let item = body.get(start..i)?.trim();
                if !item.is_empty() {
                    out.push(item);
                }
                if ch == ']' {
                    return Some(out);
                }
                start = i.checked_add(1)?;
            }
            _ => {}
        }
    }
    None
}
