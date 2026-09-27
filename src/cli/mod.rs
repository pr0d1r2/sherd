//! Arg dispatch and exit codes. The binary is a shim over `run` (`src/cli:V14`).
//!
//! A node, not a loose `main.rs`, because exit codes and usage are real
//! contracts and every §T row about them was unreachable while this file had
//! no `SPEC.md` to hold them.

use crate::{code, fed, lens, plan, slice, spec, split, state, tokens, wave};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod args;
use args::*;

mod propose;
use propose::*;

mod author;
use author::*;

mod adopt;
use adopt::*;

mod steps;
use steps::*;

mod measure;
use measure::*;

mod check;
use check::*;

/// The verb list, and the SOURCE the README's Commands section is generated
/// from (`dev:T2`). Public so `sherd-dev` reads the text this binary actually
/// prints rather than a second copy of it: two lists of one command set is
/// the founding defect §C names, and `B2` is what it costs -- `oneshot`
/// dispatched for weeks while appearing in no usage.
pub const USAGE: &str = "\
sherd -- federated SPEC.md for small-context local models

  sherd init [dir] [--stdout]  scaffold a SPEC.md, §F rows from child dirs
  sherd budget [dir]     token cost of every node, against the working budget
  sherd lens <dir> [--depth rule|why|all]  the context pack for one node
  sherd fed [dir]        the federation edges declared by a node
  sherd check [dir]      microlith structural check of every node
  sherd debt [--check|--record]  lint ratchet: density & shape vs .lint-debt
  sherd coverage [--check|--record]  coverage floor vs .coverage
  sherd validate         DAG + ids + ceilings + slice drift, one verdict
  sherd split [dir]      propose a federation split. writes nothing
  sherd seam [dir]       the public types each node declares. writes nothing
  sherd wave [dir]       the rounds a parallel build would run. writes nothing
  sherd adopt <dir> [--map FILE] [--check]  migrate a single-file SPEC.md onto a federation
  sherd sync [dir] [--check]  regenerate §N from §F. exit 1 if it wrote
  sherd route <query>    which node owns a question. 0 hit · 2 miss · 3 ambiguous
  sherd review [rev]     mechanical checks on what a commit added (default HEAD)
  sherd slice [--check|--list]  regenerate distilled slices from their sources
  sherd outcome <node> <kept|reverted>  record whether a node's work survived review
  sherd graph [--tree|--table|--dot]  federation DAG, generated from §F
  sherd plan [--format text|json]  next 3 steps, with what would invalidate each
  sherd plan --triage    unmanaged rows, with a proposed home for each
  sherd plan --milestone <M> [--format text|json]  next 3 steps among the rows milestone M claims
  sherd apply [--land]   execute step 1 only, commit it to a run branch, stop
  sherd land [--push]    fast-forward main to this run branch, if it earned it
  sherd ask <dir> <q>    ask the endpoint from a node's lens pack
  sherd tdd <dir> <Vn> <task>   red -> judge -> green -> gate -> repair
  sherd oneshot <dir> <Vn> <task>   the monolith arm: one call, whole repo

  -v, --verbose        dump every prompt and stream every reply
  -V, --version        print `sherd <semver>` and exit 0

exit: 0 clean · 1 violation · 2 usage";

/// The answer to `--version`: the crate version this binary was built from.
///
/// `env!`, so it cannot drift from `Cargo.toml` -- a second spelling of a
/// version number is the founding defect §C names, one field over.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Parse argv and dispatch. The binary itself holds nothing (`src/cli:V14`).
#[must_use]
pub fn run() -> ExitCode {
    run_args(std::env::args().skip(1).collect())
}

/// As [`run`], from an explicit argv.
///
/// `run` read `std::env::args` directly, so dispatch -- usage, exit codes,
/// every verb's routing -- could not be exercised at all. This node measured
/// 0.0% coverage over 445 lines with no test module (`.:R50`), and an
/// untestable entry point is why. Exit codes and usage are real contracts
/// per this module's own header; a contract nothing can call is a comment.
#[must_use]
pub fn run_args(args: Vec<String>) -> ExitCode {
    // `mut` ONLY under `ollama`: the sole mutation is the `-v` removal below,
    // so a `mut` on the parameter is an unused-mut warning in the DEFAULT
    // build -- which is the build that ships (`.:B26`).
    #[cfg(feature = "ollama")]
    let mut args = args;
    // -v / --verbose is positional-agnostic: it is a mode, not an argument.
    #[cfg(feature = "ollama")]
    if let Some(i) = args.iter().position(|a| a == "-v" || a == "--verbose") {
        args.remove(i);
        crate::ollama::set_verbose(true);
    }
    let root = root_for(&args);
    #[cfg(feature = "ollama")]
    crate::ollama::load_pace();
    match args.first().map(String::as_str) {
        Some("budget") => budget(&root, arg_dir(&args, &root)),
        Some("debt") => debt_cmd(
            &root,
            args.get(1).map(String::as_str),
            &crate::land::cargo_bin(),
        ),
        Some("coverage") => coverage_cmd(
            &root,
            args.get(1).map(String::as_str),
            &crate::land::cargo_bin(),
        ),
        Some("lens") => match (args.get(1), depth_arg(&args)) {
            // `.:B9` (the ROOT's, not this node's): this passed
            // `PathBuf::from(d)` while `budget` and `fed`
            // went through `arg_dir`, so it never got T10's root resolution
            // and `lens .` reported a different chain than `budget` for the
            // same node. Fixing a shared helper has to be followed by finding
            // who does not use it.
            (Some(_), Ok(dep)) => lens_cmd(&root, &arg_dir(&args, &root), dep),
            (Some(_), Err(m)) => usage(&m),
            (None, _) => usage("lens needs a dir"),
        },
        Some("fed") => fed_cmd(&arg_dir(&args, &root)),
        Some("init") => init_cmd(&root, &args),
        Some("graph") => {
            match args.get(1).map(String::as_str) {
                Some("--dot") => print!("{}", fed::dot(&root)),
                Some("--table") => print!("{}", fed::table(&root)),
                Some("--tree") => print!("{}", fed::tree(&root)),
                _ => print!("{}", fed::mermaid(&root)),
            }
            ExitCode::SUCCESS
        }
        Some("check") => check(&root),
        Some("validate") => validate(&root),
        Some("split") => {
            let apply = args.iter().any(|a| a == "--apply");
            let dir = args.get(1).filter(|a| !a.starts_with("--"));
            split_cmd(
                &root,
                &dir.map_or_else(|| root.clone(), |d| root.join(d)),
                apply,
            )
        }
        Some("seam") => seam_cmd(&root, &arg_dir(&args, &root)),
        Some("wave") => wave_cmd(&root, &arg_dir(&args, &root)),
        Some("adopt") => match args.get(1).filter(|a| !a.starts_with("--")) {
            Some(_) => adopt_cmd(&root, &args),
            None => usage("adopt needs a dir"),
        },
        Some("sync") => {
            let check = args.iter().any(|a| a == "--check");
            let dir = args.get(1).filter(|a| !a.starts_with("--"));
            sync_cmd(&root, dir.map(|d| root.join(d)).as_ref(), check)
        }
        Some("route") => match args.get(1) {
            Some(q) => route_cmd(&root, q),
            None => usage("route needs a query"),
        },
        Some("outcome") => match (args.get(1), args.get(2)) {
            (Some(node), Some(verdict)) => {
                let kept = match verdict.as_str() {
                    "kept" => true,
                    "reverted" | "failed" => false,
                    _ => {
                        return usage(
                            "outcome verdict is `kept`, `reverted` or `failed`",
                        );
                    }
                };
                plan::record_outcome(Path::new(node), kept);
                let (tried, k) = plan::record(Path::new(node));
                eprintln!(
                    "{node}: {k}/{tried} kept · believability {:.2}",
                    plan::believability(Path::new(node))
                );
                ExitCode::SUCCESS
            }
            _ => usage("outcome needs <node> <kept|reverted|failed>"),
        },
        Some("slice") => {
            slice_cmd(&root, args.get(1).map_or("", String::as_str))
        }
        Some("review") => {
            review_cmd(&root, args.get(1).map_or("HEAD", String::as_str))
        }
        Some("plan") => match take_format(args.get(1..).unwrap_or_default()) {
            Err(e) => usage(&e),
            Ok((json, rest)) => match rest.first().map(String::as_str) {
                Some("--triage") if json => {
                    usage("plan --triage has no --format json yet")
                }
                Some("--triage") => triage_cmd(&root),
                Some("--milestone") => match rest.get(1) {
                    Some(m) if plan::milestone_declared(&root, m) => {
                        plan_cmd(&root, Some(m), json)
                    }
                    Some(m) => usage(&format!(
                        "no node declares milestone `{m}` -- a `| {m} |` row in some node's \u{a7}T"
                    )),
                    None => {
                        usage("plan --milestone needs a milestone id, e.g. M1")
                    }
                },
                _ => plan_cmd(&root, None, json),
            },
        },
        #[cfg(feature = "ollama")]
        Some("apply") => match plan::apply(&root, 3) {
            Ok(sha) => {
                eprintln!(
                    "\napplied as {sha}. Run `sherd plan` again before the next step -- \
                           this commit changed the specs that plan it."
                );
                // --land asks to land it now; the evidence still decides.
                if args.iter().any(|a| a == "--land") {
                    land_verb(&root, args.iter().any(|a| a == "--push"))
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                eprintln!("sherd: {e}");
                ExitCode::from(1)
            }
        },
        Some("land") => land_verb(&root, args.iter().any(|a| a == "--push")),
        // The model verbs exist in `USAGE` whatever this build carries, so a
        // consumer reading `--help` sees the whole tool. Without the feature
        // they are not "unknown" -- they are NOT COMPILED IN, and saying so
        // is the difference between a typo and a build choice.
        #[cfg(not(feature = "ollama"))]
        Some(verb @ ("apply" | "ask" | "oneshot" | "tdd")) => {
            needs_ollama(verb)
        }
        #[cfg(feature = "ollama")]
        Some("ask") => match (args.get(1), args.get(2)) {
            (Some(_), Some(q)) => ask(&root, &arg_dir(&args, &root), q),
            _ => usage("ask needs <dir> and a question"),
        },
        #[cfg(feature = "ollama")]
        Some("oneshot") => match (args.get(1), args.get(2), args.get(3)) {
            (Some(_), Some(v), Some(t)) => {
                let dir = arg_dir(&args, &root);
                let run = crate::tdd::Run::new(&root, &dir, v, t);
                match crate::tdd::oneshot(&run) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("sherd: {e}");
                        ExitCode::from(1)
                    }
                }
            }
            _ => usage("oneshot needs <dir> <invariant> <task>"),
        },
        #[cfg(feature = "ollama")]
        Some("tdd") => match (args.get(1), args.get(2), args.get(3)) {
            (Some(_), Some(v), Some(task)) => {
                tdd_cmd(&root, &arg_dir(&args, &root), v, task)
            }
            _ => usage("tdd needs <dir> <invariant> <task>"),
        },
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        // STDOUT and exit 0, like every sibling in the toolchain. A version
        // on stderr behind exit 2 is one a CI gate has to parse out of a
        // usage banner, which is what `B10` records.
        Some("-V" | "--version") => {
            println!("sherd {VERSION}");
            ExitCode::SUCCESS
        }
        Some(other) => usage(&format!("unknown command '{other}'")),
        None => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
    }
}

/// A verb this build does not carry, named as such.
///
/// Exit 2, the USAGE code: nothing is wrong with the repository or the
/// request, the binary simply was not built with it. `.:V117` freezes the
/// model half until `0.7`, and `default = []` means the published crate is
/// the deterministic core -- so this is the common path, not an edge.
#[cfg(not(feature = "ollama"))]
fn needs_ollama(verb: &str) -> ExitCode {
    eprintln!(
        "sherd: `{verb}` needs the `ollama` feature, which this build does not \
         carry. Install it with `cargo install sherd --features ollama`."
    );
    ExitCode::from(2)
}

/// `land`, shared by the verb and by `apply --land`.
///
/// Local by default. Pushing is the outward-facing act, and an unattended run
/// that pushes at 3am publishes unreviewed generated code; `--push` opts in.
/// The local branches are the record of every try -- git as the memory.
fn land_verb(root: &Path, push: bool) -> ExitCode {
    match crate::land::land(root, push) {
        Ok(msg) => {
            eprintln!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sherd: not landing -- {e}");
            eprintln!(
                "     the branch is untouched; it is the record of the try"
            );
            ExitCode::from(1)
        }
    }
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("sherd: {msg}\n\n{USAGE}");
    ExitCode::from(2)
}

#[cfg(test)]
#[path = "tests/fixtures.rs"]
mod fixtures;

#[cfg(test)]
#[path = "tests/dispatch.rs"]
mod tests;
