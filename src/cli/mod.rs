//! Arg dispatch and exit codes. The binary is a shim over `run` (`src/cli:V14`).
//!
//! A node, not a loose `main.rs`, because exit codes and usage are real
//! contracts and every §T row about them was unreachable while this file had
//! no `SPEC.md` to hold them.

use crate::{code, fed, lens, plan, slice, spec, split, state, tokens, wave};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod parse;
use clap::Parser;
use parse::{Cmd, Verdict};

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

mod swift;
use swift::*;

mod check;
use check::*;

mod validate;
use validate::*;

mod plumb;
use plumb::*;

/// The verb list, and the SOURCE the README's Commands section is generated
/// from (`dev:T2`). Public so `sherd-dev` reads the text this binary actually
/// prints rather than a second copy of it: two lists of one command set is
/// the founding defect §C names, and `B2` is what it costs -- `oneshot`
/// dispatched for weeks while appearing in no usage.
pub const USAGE: &str = "\
sherd -- federated SPEC.md for small-context local models

  sherd init [dir] [--stdout]  scaffold a SPEC.md, §F rows from child dirs
  sherd budget [dir] [--format text|json]  token cost of every node, against the working budget
  sherd lens <dir> [--depth rule|why|all]  the context pack for one node
  sherd fed [dir]        the federation edges declared by a node
  sherd check [dir] [--format text|json]  microlith structural check of every node
  sherd debt [--check|--record]  lint ratchet: density & shape vs .lint-debt
  sherd coverage [--check|--record]  coverage floor vs .coverage
  sherd validate [--format text|json]  DAG + ids + ceilings + slice drift, one verdict
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
  sherd completions <shell>  shell completion script (bash zsh fish elvish powershell)
  sherd man              the man page, roff, on stdout

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

/// The clap command tree, for completions and the man page.
#[must_use]
pub fn command() -> clap::Command {
    parse::command()
}

/// As [`run`], from an explicit argv.
///
/// `run` read `std::env::args` directly, so dispatch -- usage, exit codes,
/// every verb's routing -- could not be exercised at all. This node measured
/// 0.0% coverage over 445 lines with no test module (`.:R50`), and an
/// untestable entry point is why. Exit codes and usage are real contracts
/// per this module's own header; a contract nothing can call is a comment.
///
/// The grammar is `parse::Cli` (`src/cli:V21`); this function only routes a
/// parsed value. A parse failure is clap's usage error -- exit 2 -- and
/// `--help`/`--version` are its success paths, exit 0.
#[must_use]
pub fn run_args(args: Vec<String>) -> ExitCode {
    let cli = match parse::Cli::try_parse_from(
        std::iter::once("sherd".to_string()).chain(args.iter().cloned()),
    ) {
        Ok(c) => c,
        Err(e) => {
            // Not `e.exit()`: that ends the process, and a test could never
            // see the code it chose.
            let code = if e.use_stderr() { 2 } else { 0 };
            let _ = e.print();
            return ExitCode::from(code);
        }
    };
    #[cfg(feature = "ollama")]
    if cli.verbose {
        crate::ollama::set_verbose(true);
    }
    let Some(cmd) = cli.cmd else {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    };
    // A verb with an `At` selector takes its root from the PARSED selector.
    // Scanning raw argv instead let any directory named like a flag value or
    // a query win over the trailing path.
    let root = match at_of(&cmd) {
        Some(at) => match root_at(at.as_deref()) {
            Ok(r) => r,
            Err(e) => return usage(&e),
        },
        // The rest take a `dir` field, or nothing: the parsed field, never a
        // raw argument, so `--format json` cannot be read as a directory.
        None => {
            let named: Vec<String> = std::iter::once("sherd".to_string())
                .chain(dir_of(&cmd).map(String::from))
                .collect();
            root_for(&named)
        }
    };
    #[cfg(feature = "ollama")]
    crate::ollama::load_pace();
    dispatch(&root, cmd)
}

/// The `At` selector of a verb that carries one; `None` for verbs that don't.
fn at_of(cmd: &Cmd) -> Option<&Option<String>> {
    match cmd {
        Cmd::Debt { at, .. }
        | Cmd::Coverage { at, .. }
        | Cmd::Validate { at, .. }
        | Cmd::Graph { at, .. }
        | Cmd::Route { at, .. }
        | Cmd::Slice { at, .. }
        | Cmd::Plan { at, .. }
        | Cmd::Apply { at, .. }
        | Cmd::Land { at, .. } => Some(&at.at),
        _ => None,
    }
}

/// The `dir` a verb without an `At` selector was given, if any.
fn dir_of(cmd: &Cmd) -> Option<&str> {
    match cmd {
        Cmd::Init { dir, .. }
        | Cmd::Budget { dir, .. }
        | Cmd::Fed { dir }
        | Cmd::Check { dir, .. }
        | Cmd::Split { dir, .. }
        | Cmd::Seam { dir }
        | Cmd::Wave { dir }
        | Cmd::Sync { dir, .. } => dir.as_deref(),
        Cmd::Lens { dir, .. }
        | Cmd::Adopt { dir, .. }
        | Cmd::Ask { dir, .. }
        | Cmd::Tdd { dir, .. }
        | Cmd::Oneshot { dir, .. } => Some(dir),
        _ => None,
    }
}

/// The repository root for an `At` selector: walk up from the named
/// directory, or from the CWD when none was given.
fn root_at(at: Option<&str>) -> Result<PathBuf, String> {
    let here = cwd();
    let Some(a) = at else {
        return Ok(repo_root_from(&here));
    };
    let p = here.join(a);
    if p.is_dir() {
        Ok(repo_root_from(&p))
    } else {
        Err(format!("`{a}` is not a directory"))
    }
}

/// Route one parsed verb. Split from [`run_args`] so the parse and the
/// routing each fit a screen.
fn dispatch(root: &Path, cmd: Cmd) -> ExitCode {
    match cmd {
        Cmd::Budget { dir, format } => budget_as(
            root,
            &arg_dir(dir.as_deref(), root),
            format.format.is_json(),
        ),
        Cmd::Debt { check, record, .. } => {
            debt_cmd(root, mode_flag(check, record), &crate::land::cargo_bin())
        }
        Cmd::Coverage { check, record, .. } => coverage_in(
            root,
            mode_flag(check, record),
            &crate::land::cargo_bin(),
            &crate::debt::swift_bin(),
        ),
        // `.:B9` (the ROOT's, not this node's): this passed
        // `PathBuf::from(d)` while `budget` and `fed` went through
        // `arg_dir`, so it never got T10's root resolution and `lens .`
        // reported a different chain than `budget` for the same node.
        // Fixing a shared helper has to be followed by finding who does not
        // use it.
        Cmd::Lens { dir, depth } => {
            lens_cmd(root, &arg_dir(Some(&dir), root), depth.into())
        }
        Cmd::Fed { dir } => fed_cmd(&arg_dir(dir.as_deref(), root)),
        Cmd::Init { dir, stdout } => init_cmd(root, dir.as_deref(), stdout),
        Cmd::Graph {
            tree, table, dot, ..
        } => {
            print!("{}", graph_render(root, tree, table, dot));
            ExitCode::SUCCESS
        }
        Cmd::Check { format, .. } => check_as(root, format.format.is_json()),
        Cmd::Validate { format, .. } if format.format.is_json() => {
            validate_json_cmd(root)
        }
        Cmd::Validate { .. } => validate(root),
        Cmd::Split { dir, apply } => split_cmd(
            root,
            &dir.map_or_else(|| root.to_path_buf(), |d| root.join(d)),
            apply,
        ),
        Cmd::Seam { dir } => seam_cmd(root, &arg_dir(dir.as_deref(), root)),
        Cmd::Wave { dir } => wave_cmd(root, &arg_dir(dir.as_deref(), root)),
        Cmd::Adopt { dir, map, check } => {
            adopt_cmd(root, &dir, map.as_deref(), check)
        }
        Cmd::Sync { dir, check } => {
            sync_cmd(root, dir.map(|d| root.join(d)).as_ref(), check)
        }
        Cmd::Route { query, .. } => route_cmd(root, &query),
        Cmd::Outcome { node, verdict, .. } => outcome_cmd(&node, verdict),
        Cmd::Slice { check, list, .. } => slice_cmd(
            root,
            match (check, list) {
                (true, _) => "--check",
                (_, true) => "--list",
                _ => "",
            },
        ),
        Cmd::Review { rev } => {
            review_cmd(root, rev.as_deref().unwrap_or("HEAD"))
        }
        Cmd::Plan {
            format,
            triage,
            milestone,
            ..
        } => plan_verb(root, format.format.is_json(), triage, milestone),
        Cmd::Land { push, .. } => land_verb(root, push),
        Cmd::Completions { shell } => {
            clap_complete::generate(
                clap_complete::Shell::from(shell),
                &mut command(),
                "sherd",
                &mut std::io::stdout(),
            );
            ExitCode::SUCCESS
        }
        Cmd::Man => match clap_mangen::Man::new(command())
            .render(&mut std::io::stdout())
        {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("sherd: man: {e}");
                ExitCode::from(1)
            }
        },
        model => model_verb(root, model),
    }
}

/// `--check` / `--record` as the one mode string the ratchet verbs read.
fn mode_flag(check: bool, record: bool) -> Option<&'static str> {
    match (check, record) {
        (true, _) => Some("--check"),
        (_, true) => Some("--record"),
        _ => None,
    }
}

/// The `graph` rendering a flag picks; mermaid when none does.
fn graph_render(root: &Path, tree: bool, table: bool, dot: bool) -> String {
    match (dot, table, tree) {
        (true, _, _) => fed::dot(root),
        (_, true, _) => fed::table(root),
        (_, _, true) => fed::tree(root),
        _ => fed::mermaid(root),
    }
}

/// `outcome <node> <kept|reverted|failed>`.
fn outcome_cmd(node: &str, verdict: Verdict) -> ExitCode {
    plan::record_outcome(Path::new(node), verdict.is_kept());
    let (tried, k) = plan::record(Path::new(node));
    eprintln!(
        "{node}: {k}/{tried} kept · believability {:.2}",
        plan::believability(Path::new(node))
    );
    ExitCode::SUCCESS
}

/// `plan`, its `--triage` and `--milestone` forms.
fn plan_verb(
    root: &Path,
    json: bool,
    triage: bool,
    milestone: Option<String>,
) -> ExitCode {
    match milestone {
        _ if triage && json => usage("plan --triage has no --format json yet"),
        _ if triage => triage_cmd(root),
        Some(m) if plan::milestone_declared(root, &m) => {
            plan_cmd(root, Some(&m), json)
        }
        Some(m) => usage(&format!(
            "no node declares milestone `{m}` -- a `| {m} |` row in some node's \u{a7}T"
        )),
        None => plan_cmd(root, None, json),
    }
}

/// The verbs that need the `ollama` feature, or say so.
///
/// The model verbs exist in `USAGE` whatever this build carries, so a
/// consumer reading `--help` sees the whole tool. Without the feature they
/// are not "unknown" -- they are NOT COMPILED IN, and saying so is the
/// difference between a typo and a build choice.
#[cfg(not(feature = "ollama"))]
fn model_verb(_root: &Path, cmd: Cmd) -> ExitCode {
    let verb = match cmd {
        Cmd::Apply { .. } => "apply",
        Cmd::Ask { .. } => "ask",
        Cmd::Oneshot { .. } => "oneshot",
        _ => "tdd",
    };
    needs_ollama(verb)
}

#[cfg(feature = "ollama")]
fn model_verb(root: &Path, cmd: Cmd) -> ExitCode {
    match cmd {
        Cmd::Apply { land, push, .. } => match plan::apply(root, 3) {
            Ok(sha) => {
                eprintln!(
                    "\napplied as {sha}. Run `sherd plan` again before the next step -- \
                           this commit changed the specs that plan it."
                );
                // --land asks to land it now; the evidence still decides.
                if land {
                    land_verb(root, push)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                eprintln!("sherd: {e}");
                ExitCode::from(1)
            }
        },
        Cmd::Ask { dir, question } => {
            ask(root, &arg_dir(Some(&dir), root), &question)
        }
        Cmd::Oneshot {
            dir,
            invariant,
            task,
        } => {
            let dir = arg_dir(Some(&dir), root);
            let run = crate::tdd::Run::new(root, &dir, &invariant, &task);
            match crate::tdd::oneshot(&run) {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprintln!("sherd: {e}");
                    ExitCode::from(1)
                }
            }
        }
        Cmd::Tdd {
            dir,
            invariant,
            task,
        } => tdd_cmd(root, &arg_dir(Some(&dir), root), &invariant, &task),
        _ => ExitCode::from(2),
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

#[cfg(test)]
#[path = "tests/routes.rs"]
mod routes;

#[cfg(test)]
#[path = "tests/unread.rs"]
pub(crate) mod unread;

#[cfg(test)]
#[path = "tests/verdicts.rs"]
mod verdicts;
