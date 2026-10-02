//! V7: a shell codebase on V2's ladder. Each test carries the case that
//! must be found beside the one that must not, so a reader that finds
//! nothing fails here too (`src/fed:V10`).

use super::*;
use crate::testrepo::TestRepo;
use std::path::PathBuf;

/// A committed repo: `spec` as its root `SPEC.md`, and each script.
fn repo(tag: &str, spec: &str, scripts: &[&str]) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    r.write("SPEC.md", spec)?;
    for s in scripts {
        r.write(s, "#!/bin/sh\n")?;
    }
    r.commit("fixture")?;
    Ok(r)
}

fn law(rows: &str) -> String {
    format!("# SPEC\n\n## \u{a7}V INVARIANTS\n\n{rows}")
}

fn named<'a>(p: &'a [Proposed], name: &str) -> Option<&'a Proposed> {
    p.iter().find(|c| c.name == name)
}

/// Rows weighing each candidate, by name, through `rank_in`.
fn rows(dir: &Path, p: &[Proposed], spec: &str) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = rank_in(dir, p, spec)
        .iter()
        .map(|r| (r.node.name.clone(), r.rows))
        .collect();
    out.sort();
    out
}

fn owned(
    dir: &Path,
    p: &[Proposed],
    name: &str,
) -> Result<Vec<PathBuf>, String> {
    let c = named(p, name).ok_or(format!("no candidate {name}"))?;
    Ok(scripts_of(dir, c, &crate::fed::script_files(dir)))
}

/// The fixture #81 asks for: rows citing `a/one.sh` and `two.sh` land on
/// `a` and `b`, one by its path and one by its basename alone.
#[test]
fn each_script_dir_is_a_candidate_weighted_by_the_rows_citing_it()
-> Result<(), String> {
    let spec = law("V1: `a/one.sh` checks it\nV2: `two.sh` runs it\n");
    let r = repo("split-sh", &spec, &["a/one.sh", "a/other.sh", "b/two.sh"])?;
    let p = structure(r.path());
    let got: Vec<(&str, Evidence)> =
        p.iter().map(|c| (c.name.as_str(), c.evidence)).collect();
    assert_eq!(got, vec![("a", Evidence::Drawn), ("b", Evidence::Drawn)]);
    assert_eq!(
        rows(r.path(), &p, &spec),
        vec![("a".into(), 1), ("b".into(), 1)]
    );
    Ok(())
}

/// An edge is parent + 1 (`src/fed:V2`): a deep script dir is proposed
/// through its top dir, which owns the script.
#[test]
fn a_deep_script_dir_is_proposed_through_its_top_dir() -> Result<(), String> {
    let r = repo("split-sh-deep", &law("V1: x\n"), &["c/d/e/x.sh"])?;
    let p = structure(r.path());
    let names: Vec<&str> = p.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["c"]);
    assert_eq!(owned(r.path(), &p, "c")?, vec![r.path().join("c/d/e/x.sh")]);
    Ok(())
}

/// A basename two files share counts for neither and is named, while the
/// same script cited by its path still counts.
#[test]
fn a_basename_two_files_share_counts_nowhere_and_is_named() -> Result<(), String>
{
    let spec = law("V1: `run.sh` starts it\nV2: `a/run.sh` stops it\n");
    let r = repo("split-sh-amb", &spec, &["a/run.sh", "b/run.sh"])?;
    let p = structure(r.path());
    assert_eq!(
        rows(r.path(), &p, &spec),
        vec![("a".into(), 1), ("b".into(), 0)]
    );
    let all = crate::fed::script_files(r.path());
    assert_eq!(ambiguous_scripts(&spec, &all), vec!["run.sh"]);
    Ok(())
}

/// `src/split:V4`: a dir named like an English word is not cited by the
/// word. Only the script is a citation.
#[test]
fn a_bare_dir_word_is_not_a_citation() -> Result<(), String> {
    let spec = law("V1: trips are cheap\nV2: `x.sh` is one\n");
    let r = repo("split-sh-word", &spec, &["trips/x.sh"])?;
    let p = structure(r.path());
    assert_eq!(rows(r.path(), &p, &spec), vec![("trips".into(), 1)]);
    Ok(())
}

/// Three flat scripts sharing a prefix are a family; two are a pair.
#[test]
fn three_scripts_sharing_a_prefix_are_a_family() -> Result<(), String> {
    let flat = ["pr-a.sh", "pr-b.sh", "pr-c.sh", "gate-x.sh", "gate-y.sh"];
    let r = repo("split-sh-family", &law("V1: x\n"), &flat)?;
    let p = structure(r.path());
    let pr = named(&p, "pr").ok_or("pr")?;
    assert_eq!(pr.evidence, Evidence::Cohesion);
    assert_eq!(pr.members, vec!["pr-a", "pr-b", "pr-c"]);
    assert_eq!(owned(r.path(), &p, "pr")?.len(), 3);
    assert!(named(&p, "gate").is_none(), "{p:?}");
    Ok(())
}

/// A dir the Rust reading and the shell reading both find is one node,
/// keeping the Rust grade and gaining the scripts.
#[test]
fn a_dir_both_readings_find_is_proposed_once() -> Result<(), String> {
    let r = repo(
        "split-sh-mixed",
        &law("V1: x\n"),
        &["tools/build.sh", "ops/deploy.sh"],
    )?;
    r.write("lib.rs", "pub mod tools;\n")?;
    r.write("tools/mod.rs", "\n")?;
    let p = structure(r.path());
    assert_eq!(p.iter().filter(|c| c.name == "tools").count(), 1, "{p:?}");
    let tools = named(&p, "tools").ok_or("tools")?;
    assert_eq!(tools.evidence, Evidence::Drawn);
    assert_eq!(
        owned(r.path(), &p, "tools")?,
        vec![r.path().join("tools/build.sh")]
    );
    assert!(
        named(&p, "ops").is_some(),
        "the shell-only dir is still proposed"
    );
    Ok(())
}

/// No Rust and no script: nothing to propose, as before.
#[test]
fn a_tree_with_neither_proposes_nothing() -> Result<(), String> {
    let r = repo("split-sh-none", &law("V1: x\n"), &[])?;
    r.write("docs/README.md", "x\n")?;
    assert!(structure(r.path()).is_empty());
    let empty: Vec<PathBuf> = crate::fed::script_files(r.path());
    assert!(empty.is_empty());
    Ok(())
}
