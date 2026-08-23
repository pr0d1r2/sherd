//! `bbx-dev` -- tooling that maintains THIS repository.
//!
//! Separate from `bbx` because it is meaningless to anyone who installs the
//! crate: these verbs read the repository's own files and write its own
//! documentation. A second `[[bin]]` in the published package would have put
//! them on a consumer's PATH; an unpublished workspace member cannot.
//!
//! This file is a SHIM -- argument dispatch, file reads, exit codes -- for
//! the same reason `src/main.rs` is one over `cli::run`: everything that
//! decides anything lives in `badge`, where it is handed its input and can be
//! tested from a string literal (`src/cli:V6`).
//!
//! Exit codes follow `src/cli:V1` -- 0 clean, 1 violation, 2 usage -- because
//! a second convention for the same thing is the duplication §C exists to
//! end.

mod badge;
mod select;

use badge::Outcome;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
bbx-dev -- tooling for the blackbox repository itself

  bbx-dev --check [<path>...]  run every check, concurrently. Paths narrow the
                               work to what a change can have invalidated; no
                               paths means compare everything.
  bbx-dev readme [--check] [<path>...]
                               regenerate the generated blocks in README.md --
                               the badges, and the three `bbx graph`
                               renderings. --check reports and writes nothing.

exit: 0 clean · 1 violation · 2 usage
";

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = argv.iter().map(String::as_str).collect();
    let paths: Vec<String> = argv
        .iter()
        .filter(|a| !a.starts_with("--"))
        .filter(|a| a.as_str() != "readme")
        .cloned()
        .collect();
    match args.first() {
        Some(&"--check") => check_all(&paths),
        Some(&"readme") => readme(args.contains(&"--check"), &paths),
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// Every check this binary knows, run CONCURRENTLY.
///
/// One entry point for the hooks: a hook that has to name each check by hand
/// grows a second list of what the gate does, and the first time the two
/// disagree the missing one is invisible (`.:V102`). There is one check today
/// and the loop is written for N because the cost of that is a scope block,
/// while the cost of retrofitting it is another list.
///
/// `std::thread::scope` rather than a runtime: these are file reads and string
/// comparisons, and a dependency for that would be `.:§C`'s NIH rule read
/// backwards.
fn check_all(paths: &[String]) -> ExitCode {
    type Check = (&'static str, fn(&[String]) -> ExitCode);
    let checks: &[Check] = &[("readme", |p| readme(true, p))];

    let codes: Vec<(&str, ExitCode)> = std::thread::scope(|s| {
        let handles: Vec<_> = checks
            .iter()
            .map(|(name, f)| (*name, s.spawn(move || f(paths))))
            .collect();
        handles
            .into_iter()
            .map(|(name, h)| {
                (name, h.join().unwrap_or_else(|_| ExitCode::from(1)))
            })
            .collect()
    });

    for (name, code) in &codes {
        if format!("{code:?}") != format!("{:?}", ExitCode::SUCCESS) {
            eprintln!("bbx-dev: {name} failed");
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}

/// The repository root: the directory holding `.git` AND `SPEC.md`.
///
/// `src/cli:V5`'s rule, restated rather than imported, because this binary
/// may be run from anywhere and cargo's `CARGO_MANIFEST_DIR` points at `dev/`
/// rather than at the repository.
fn repo_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join(".git").exists() && dir.join("SPEC.md").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel))
        .map_err(|e| format!("bbx-dev: {rel}: {e}"))
}

fn sources(root: &Path) -> Result<badge::Sources, String> {
    Ok(badge::Sources {
        manifest: read(root, "Cargo.toml")?,
        coverage: read(root, ".coverage")?,
        debt: read(root, ".lint-debt")?,
        pkl: read(root, "hk.pkl")?,
        lock: read(root, "flake.lock")?,
        workflow: read(root, ".github/workflows/ci.yml")?,
    })
}

/// Every generated block, and the owner each one is rendered from.
///
/// The three graph renderings come from `bbx`'s own `fed` module rather than
/// by shelling out to the binary and reading its stdout: §C forbids a second
/// reading of a rule that already has an owner, and a pipe is one.
fn blocks(root: &Path, nodes: usize) -> Result<badge::Blocks, String> {
    let facts = badge::facts(&sources(root)?, nodes)?;
    Ok(vec![
        ("badges".to_string(), badge::render(&facts)),
        (
            "graph-tree".to_string(),
            format!("```\n{}```\n", bbx::fed::tree(root)),
        ),
        (
            "graph-mermaid".to_string(),
            format!("```mermaid\n{}```\n", bbx::fed::mermaid(root)),
        ),
        ("graph-table".to_string(), bbx::fed::table(root)),
    ])
}

fn readme(check_only: bool, paths: &[String]) -> ExitCode {
    // What a change can have invalidated. An empty selection from a NON-empty
    // change set means this commit touched nothing the README is rendered
    // from, which is clean rather than skipped -- the wide run on push
    // compares every block regardless.
    let wanted = select::selected(paths);
    if wanted.is_empty() {
        return ExitCode::SUCCESS;
    }
    let Some(root) = repo_root() else {
        eprintln!(
            "bbx-dev: not inside the repository -- no ancestor holds both .git and SPEC.md"
        );
        return ExitCode::from(2);
    };
    // The node count is `bbx`'s own walk, not a second one: §C forbids
    // reimplementing a rule that already has an owner, and the DAG this badge
    // reports is exactly what `fed::discover` enumerates.
    let nodes = bbx::fed::discover(&root).len();
    let outcome = blocks(&root, nodes).and_then(|b| {
        let text = read(&root, "README.md")?;
        let scoped: badge::Blocks = b
            .into_iter()
            .filter(|(name, _)| wanted.contains(&name.as_str()))
            .collect();
        Ok(badge::apply(&text, &scoped, check_only))
    });
    match outcome {
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
        Ok(Outcome::Fresh) => ExitCode::SUCCESS,
        Ok(Outcome::NoMarkers) => {
            eprintln!(
                "bbx-dev: README.md is missing a block's markers. Each generated block needs a `<!-- BEGIN <name> -->` / `<!-- END <name> -->` pair: badges, graph-tree, graph-mermaid, graph-table."
            );
            ExitCode::from(1)
        }
        Ok(Outcome::Stale(diff)) => {
            eprintln!(
                "bbx-dev: a generated README block is STALE. Run `bbx-dev readme` (or `hk fix`) to regenerate it from the files that own each number -- Cargo.toml, hk.pkl, .coverage, .lint-debt, flake.lock, ci.yml and the §F tables."
            );
            for line in diff {
                eprintln!("  {line}");
            }
            ExitCode::from(1)
        }
        Ok(Outcome::Wrote(next)) => {
            match std::fs::write(root.join("README.md"), next) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("bbx-dev: README.md: {e}");
                    ExitCode::from(1)
                }
            }
        }
    }
}
