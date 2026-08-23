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

use badge::Outcome;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
bbx-dev -- tooling for the blackbox repository itself

  bbx-dev readme [--check]   regenerate every generated block in README.md --
                             the badges, and the three `bbx graph` renderings.
                             --check reports staleness and writes nothing.

exit: 0 clean · 1 violation · 2 usage
";

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = argv.iter().map(String::as_str).collect();
    match args.split_first() {
        Some((&"readme", rest)) => readme(rest.contains(&"--check")),
        _ => {
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
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

fn readme(check_only: bool) -> ExitCode {
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
        Ok(badge::apply(&text, &b, check_only))
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
