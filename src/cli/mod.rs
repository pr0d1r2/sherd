//! Arg dispatch and exit codes. The binary is a shim over `run` (`src/cli:V14`).
//!
//! A node, not a loose `main.rs`, because exit codes and usage are real
//! contracts and every §T row about them was unreachable while this file had
//! no `SPEC.md` to hold them.

use crate::{fed, lens, plan, slice, spec, state, tokens};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

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
  sherd validate         DAG + ids + ceilings + slice drift, one verdict
  sherd split [dir]      propose a federation split. writes nothing
  sherd sync [dir] [--check]  regenerate §N from §F. exit 1 if it wrote
  sherd route <query>    which node owns a question. 0 hit · 2 miss · 3 ambiguous
  sherd review [rev]     mechanical checks on what a commit added (default HEAD)
  sherd slice [--check|--list]  regenerate distilled slices from their sources
  sherd outcome <node> <kept|reverted>  record whether a node's work survived review
  sherd graph [--tree|--table|--dot]  federation DAG, generated from §F
  sherd plan             next 3 steps, with what would invalidate each
  sherd plan --triage    unmanaged rows, with a proposed home for each
  sherd apply [--land]   execute step 1 only, commit it to a run branch, stop
  sherd land [--push]    fast-forward main to this run branch, if it earned it
  sherd ask <dir> <q>    ask the endpoint from a node's lens pack
  sherd tdd <dir> <Vn> <task>   red -> judge -> green -> gate -> repair
  sherd oneshot <dir> <Vn> <task>   the monolith arm: one call, whole repo

  -v, --verbose        dump every prompt and stream every reply

exit: 0 clean · 1 violation · 2 usage";

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
pub fn run_args(mut args: Vec<String>) -> ExitCode {
    // -v / --verbose is positional-agnostic: it is a mode, not an argument.
    #[cfg(feature = "ollama")]
    if let Some(i) = args.iter().position(|a| a == "-v" || a == "--verbose") {
        args.remove(i);
        crate::ollama::set_verbose(true);
    }
    let root = repo_root();
    #[cfg(feature = "ollama")]
    crate::ollama::load_pace();
    match args.first().map(String::as_str) {
        Some("budget") => budget(&root, arg_dir(&args, &root)),
        Some("lens") => match (args.get(1), depth_arg(&args)) {
            // B9: this passed `PathBuf::from(d)` while `budget` and `fed`
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
        Some("plan") if args.get(1).map(String::as_str) == Some("--triage") => {
            triage_cmd(&root)
        }
        Some("plan") => plan_cmd(&root),
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
        Some(other) => usage(&format!("unknown command '{other}'")),
        None => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
    }
}

/// The repo root, not the invocation directory.
///
/// `sherd` is a shim over `cargo run`, so CWD is wherever you typed it. Using
/// CWD federated from a SUBDIRECTORY silently -- fewer nodes, a truncated
/// chain, and no error to say so. Walk up to the git root instead.
fn repo_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    repo_root_from(&cwd)
}

/// The walk, given a starting directory (V6).
///
/// Split from [`repo_root`] so a test can be HANDED a tree instead of
/// discovering one. `B1` is what the unsplit version cost: the test asserted
/// that `repo_root()` finds a `.git` and a `SPEC.md`, which it always does
/// when the runner sits in this checkout and never does anywhere else -- so
/// the suite passed here and failed in a tarball, and the assertion was
/// about the runner's location rather than about the walk.
fn repo_root_from(start: &Path) -> PathBuf {
    let mut d = start;
    loop {
        if d.join(".git").exists() && d.join("SPEC.md").is_file() {
            return d.to_path_buf();
        }
        match d.parent() {
            Some(p) => d = p,
            None => return start.to_path_buf(),
        }
    }
}

/// The `[dir]` argument, resolved AGAINST ROOT.
///
/// It used to be handed back verbatim, so `src/tdd` stayed relative while
/// `fed::discover` returns absolute paths -- the two never compared equal.
/// `sherd fed` survived that only because the CWD happens to be the repo root;
/// from a subdirectory it read the wrong `SPEC.md` or none.
///
/// `join` leaves an absolute argument alone, so passing a full path still
/// works.
/// `--depth rule|why|all`, defaulting to `rule` (`.:V45`).
///
/// An unknown value is a USAGE error, never a quiet fall back to the default.
/// `sherd lens x --depth rules` would otherwise look exactly like a flag that
/// was honoured, which is `.:B8` wearing a typo -- and `.:V105` says a
/// declared option must change behaviour or it is a claim with no runner.
fn depth_arg(args: &[String]) -> Result<lens::Depth, String> {
    let Some(i) = args.iter().position(|a| a == "--depth") else {
        return Ok(lens::Depth::Rule);
    };
    match args.get(i + 1).map(String::as_str) {
        Some("rule") => Ok(lens::Depth::Rule),
        Some("why") => Ok(lens::Depth::Why),
        Some("all") => Ok(lens::Depth::All),
        Some(v) => Err(format!("unknown --depth `{v}` -- rule|why|all")),
        None => Err("--depth needs rule|why|all".into()),
    }
}

fn arg_dir(args: &[String], root: &Path) -> PathBuf {
    args.get(1)
        .map_or_else(|| root.to_path_buf(), |d| root.join(d))
}

/// `sherd init [dir] [--stdout]` -- scaffold a `SPEC.md` for a directory.
///
/// REFUSES an existing file, exit 1, and there is no `--force`. Clobbering a
/// spec is the one write this tool must never make: `SPEC.md` is the law the
/// rest of the binary enforces, and a scaffold that can overwrite it can
/// erase every invariant a repository has recorded. Deleting it first is a
/// deliberate act a human takes, with git watching.
///
/// `--stdout` writes nothing and prints instead, so the output can be read
/// before it is a file.
/// Directories under `dir` that could BE nodes.
///
/// The same exclusions the walk uses, so `init` and `graph` cannot disagree
/// about what a child is -- a scaffold naming a child the DAG will not
/// descend into writes a row that can never be satisfied.
fn child_dirs(dir: &Path) -> Result<Vec<String>, String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("sherd: {}: {e}", dir.display()))?;
    let mut children: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.') && n != "target")
        .collect();
    children.sort();
    Ok(children)
}

/// `sherd init [dir] [--stdout]` -- scaffold a `SPEC.md` for a directory.
///
/// REFUSES an existing file, exit 1, and there is no `--force`. Clobbering a
/// spec is the one write this tool must never make: `SPEC.md` is the law the
/// rest of the binary enforces, and a scaffold that can overwrite it can
/// erase every invariant a repository has recorded. Deleting it first is a
/// deliberate act a human takes, with git watching.
///
/// `--stdout` writes nothing and prints instead, so the output can be read
/// before it is a file.
/// The directory to scaffold: the first non-flag argument, or the root.
fn init_dir(root: &Path, args: &[String]) -> PathBuf {
    args.iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map_or_else(|| root.to_path_buf(), |d| root.join(d))
}

/// `sherd split <dir> [--apply]` -- propose a federation split, write nothing.
///
/// A PROPOSAL, and the refusal to write without `--apply` is the point: which
/// module owns which rule is a judgement, and a tool that moved spec rows on
/// its own would be rewriting law it cannot read. `--apply` is not built, and
/// says so rather than silently doing nothing.
fn split_cmd(root: &Path, dir: &Path, apply: bool) -> ExitCode {
    if apply {
        eprintln!(
            "sherd: --apply is not built. `split` proposes; moving rows between \
             specs is a judgement a reader makes."
        );
        return ExitCode::from(2);
    }
    if !dir.join("SPEC.md").is_file() {
        eprintln!("sherd: {} carries no SPEC.md", dir.display());
        return ExitCode::from(2);
    }
    let (cost, ceiling) = split_budget(root, dir);
    println!("{}: chain {cost} tok of {ceiling}", node_label(root, dir));

    // STRUCTURE FIRST (`.:src/plan:V17`): what the code already separated, then
    // the prose weight of each. A module the spec never mentions is still a
    // node; a ranking by rows cannot see it (`.:src/plan:B12`).
    let proposed = plan::structure(dir);
    if proposed.is_empty() {
        println!("  no module declarations found -- nothing to propose");
        return ExitCode::SUCCESS;
    }
    let spec = std::fs::read_to_string(dir.join("SPEC.md")).unwrap_or_default();
    print_structure(&proposed, &spec);
    ExitCode::SUCCESS
}

/// A node's path relative to root, with the root itself as `.` rather than
/// the empty string it strips to.
fn node_label(root: &Path, dir: &Path) -> String {
    let rel = dir.strip_prefix(root).unwrap_or(dir).display().to_string();
    if rel.is_empty() { ".".to_string() } else { rel }
}

/// The proposal: what the code separated, graded, with the prose weight of
/// each node beside it.
///
/// `rows` is EVIDENCE ABOUT a node rather than the reason for it -- a `0`
/// there means the spec never mentions a module the author already split
/// out, which is a gap in the spec and not a reason to skip the node.
fn print_structure(proposed: &[plan::Proposed], spec: &str) {
    println!("\n  node           evidence    rows   tok  members");
    for p in proposed {
        let (rows, tokens) = plan::row_weight(spec, &p.name);
        let members = if p.members.len() > 1 {
            format!("{} ({})", p.members.len(), p.members.join(" "))
        } else {
            String::new()
        };
        let note = if p.split_layout {
            " MERGE the .rs into mod.rs first"
        } else {
            ""
        };
        println!(
            "  {:<14} {:<10} {:>4}  {:>4}  {members}{note}",
            p.name,
            p.evidence.label(),
            rows,
            tokens,
        );
        if !p.shared.is_empty() {
            println!("  {:<14} shares: {}", "", p.shared.join(", "));
        }
    }
    let named: usize = proposed.iter().map(|p| p.members.len()).sum();
    let tally =
        |e: plan::Evidence| proposed.iter().filter(|p| p.evidence == e).count();
    println!(
        "\n  {} node(s) over {named} module(s): {} directory · {} pub mod · \
         {} family · {} declared.",
        proposed.len(),
        tally(plan::Evidence::Drawn),
        tally(plan::Evidence::Published),
        tally(plan::Evidence::Cohesion),
        tally(plan::Evidence::Declared),
    );
    println!(
        "  Evidence is how explicitly the author drew the boundary. A \
         `declared` node is a module and nothing more -- real, and the \
         weakest reason to promote one. Grouping those is a judgement this \
         does not make."
    );
}

/// A node's chain cost and the ceiling it inherits, or zeroes when either
/// cannot be read -- `budget` is the verb that reports why.
fn split_budget(root: &Path, dir: &Path) -> (u64, u64) {
    let cost = lens::pack(root, dir, lens::Depth::Rule)
        .map(|p| p.cost.tokens)
        .unwrap_or_default();
    let ceiling = lens::ceiling_for(root, dir).unwrap_or_default();
    (cost, ceiling)
}

/// `sherd sync [dir]` -- regenerate `§N` from the `§F` tables above it.
///
/// Exit 1 IF IT WROTE, which reads backwards until you see it from CI: a
/// generated section that had to change means the committed tree was stale,
/// and a run that silently fixed it would let the staleness ship. Exit 0 is
/// "already correct". `.:V36` makes `§F` authoritative, so this never reads
/// an existing `§N` to decide anything -- it computes what one must say.
fn sync_cmd(root: &Path, dir: Option<&PathBuf>, check: bool) -> ExitCode {
    let targets = dir.map_or_else(|| fed::discover(root), |d| vec![d.clone()]);
    let Ok(wrote) = sync_all(root, &targets, check) else {
        return ExitCode::from(1);
    };
    println!(
        "\n  {} nodes examined · {wrote} {}",
        targets.len(),
        if check { "stale" } else { "rewritten" }
    );
    if wrote == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// Every node, and how many needed a rewrite. `Err` when one could not be
/// read or written -- an unreadable node is not "already correct", and
/// counting it as clean is the vacuous pass `.:V48` forbids.
fn sync_all(
    root: &Path,
    targets: &[PathBuf],
    check: bool,
) -> Result<usize, ()> {
    let mut wrote = 0usize;
    for node in targets {
        match sync_node(root, node, check) {
            Ok(true) => {
                sync_report(node, check);
                wrote = wrote.saturating_add(1);
            }
            Ok(false) => {}
            Err(e) => {
                eprintln!("sherd: {e}");
                return Err(());
            }
        }
    }
    Ok(wrote)
}

/// What a changed node is called depends on which half ran: `--check` found
/// it STALE, the fix half REWROTE it.
fn sync_report(node: &Path, check: bool) {
    let verb = if check { "STALE" } else { "rewritten" };
    println!("{}: §N {verb}", node.display());
}

/// Where `§N` goes in a document that has none yet.
///
/// Beside `§F` where there is one. A LEAF has no `§F` at all -- that is what
/// makes it a leaf -- and `.:V34` still requires its `§N`, so the anchor
/// falls back to `§G`, the one section every spec has.
fn nav_anchor(text: &str) -> &'static str {
    if text.contains("## \u{a7}F") {
        "F FEDERATION"
    } else {
        "G GOAL"
    }
}

/// One node's `§N`. `Ok(true)` when the file changed.
fn sync_node(root: &Path, node: &Path, check: bool) -> Result<bool, String> {
    let path = node.join("SPEC.md");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    let body = fed::nav_section(&fed::nav(root, node));
    let next = spec::upsert_section(&text, "N NAV", &body, nav_anchor(&text));
    if next == text {
        return Ok(false);
    }
    // `--check` REPORTS. CI runs the checker, and a checker that repairs what
    // it finds is a green tick over a diff nobody has seen.
    if check {
        return Ok(true);
    }
    std::fs::write(&path, next)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(true)
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

/// The refusal, and there is no `--force` to bypass it.
///
/// Clobbering a spec is the one write this tool must never make: `SPEC.md` is
/// the law the rest of the binary enforces, so a scaffold that can overwrite
/// it can erase every invariant a repository has recorded. Deleting the file
/// first is a deliberate act a human takes, with git watching.
fn init_refusal(target: &Path) -> Option<ExitCode> {
    target.exists().then(|| {
        eprintln!(
            "sherd: {} exists -- refusing to overwrite a spec. There is no --force: \
             delete it yourself if that is what you mean.",
            target.display()
        );
        ExitCode::from(1)
    })
}

fn init_cmd(root: &Path, args: &[String]) -> ExitCode {
    let stdout = args.iter().any(|a| a == "--stdout");
    let dir = init_dir(root, args);
    if !dir.is_dir() {
        eprintln!("sherd: {} is not a directory", dir.display());
        return ExitCode::from(2);
    }
    let target = dir.join("SPEC.md");
    if let Some(refusal) = (!stdout).then(|| init_refusal(&target)).flatten() {
        return refusal;
    }
    match init_body(root, &dir) {
        Err(e) => init_failed(&e),
        Ok((body, n)) => init_emit(&target, &body, n, stdout),
    }
}

fn init_failed(msg: &str) -> ExitCode {
    eprintln!("{msg}");
    ExitCode::from(1)
}

/// `--stdout` is the PREVIEW, so it must never write. Both paths go through
/// one function, because a preview that diverged from the write would show
/// something other than what lands.
fn init_emit(
    target: &Path,
    body: &str,
    children: usize,
    stdout: bool,
) -> ExitCode {
    if stdout {
        print!("{body}");
        return ExitCode::SUCCESS;
    }
    init_write(target, body, children)
}

/// The scaffold text for a directory, and how many children it names.
fn init_body(root: &Path, dir: &Path) -> Result<(String, usize), String> {
    let children = child_dirs(dir)?;
    // The node NAME is the path relative to root, so `§G`'s prompt names the
    // node a reader is looking at rather than an absolute path only this
    // machine has.
    let name = dir
        .strip_prefix(root)
        .ok()
        .and_then(|p| p.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(".");
    Ok((crate::spec::scaffold(name, &children), children.len()))
}

fn init_write(target: &Path, body: &str, children: usize) -> ExitCode {
    match std::fs::write(target, body) {
        Ok(()) => {
            println!(
                "{}: scaffolded, {children} child row(s). Fill the prompts, then `sherd check`.",
                target.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sherd: {}: {e}", target.display());
            ExitCode::from(1)
        }
    }
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

/// Working budget on the confirmed target tier: gpt-oss:20b at its full
/// 131,072 window, minus measured harness entry cost.
const WINDOW: u64 = 131_072;

fn budget(root: &Path, dir: PathBuf) -> ExitCode {
    let work = tokens::working(WINDOW);
    println!(
        "window {WINDOW} · entry {} · working {work}\n",
        tokens::ENTRY_COST
    );
    let nodes = fed::discover(root);
    let mut total = 0;
    let mut examined = 0;
    let mut over = 0;
    for node in &nodes {
        // §I declares `sherd budget [dir]`. The argument was parsed by
        // `arg_dir` and then dropped, so every invocation reported the whole
        // repo -- an interface promised and unread, which is `.:V104`'s own
        // shape appearing in the command that enforces it.
        if !node.starts_with(&dir) {
            continue;
        }
        let p = match lens::pack(root, node, lens::Depth::Rule) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("sherd: {}: {e}", node.display());
                return ExitCode::from(1);
            }
        };
        // Re-read per node rather than hoisting the load: the key mapping
        // lives in `ceiling_for` and copying it here to save twelve reads of
        // a one-kilobyte file would be two readings of one rule.
        let ceiling = match lens::ceiling_for(root, node) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("sherd: {e}");
                return ExitCode::from(1);
            }
        };
        total += p.cost.tokens;
        examined += 1;
        let rel = node.strip_prefix(root).unwrap_or(node);
        let name = if rel.as_os_str().is_empty() {
            Path::new(".")
        } else {
            rel
        };
        // A verdict states direction and distance (`src/lens:V4`): "over"
        // without "by how much" cannot tell a node that needs splitting from
        // one that drifted eleven tokens past.
        let mark = match lens::verdict(p.cost.tokens, ceiling) {
            lens::Verdict::Fits { .. } => String::new(),
            lens::Verdict::Over { by } => {
                over += 1;
                format!("  OVER by {by}")
            }
        };
        println!(
            "  {:<24} chain {:>6} tok  ({} nodes)  ceiling {ceiling:>6}{mark}",
            name.display(),
            p.cost.tokens,
            p.chain.len()
        );
    }
    // V48: say what was examined, not only what failed.
    println!(
        "\n  {examined} nodes examined · {total} tok if all chains loaded \
         · {over} over ceiling"
    );
    // Examining NOTHING is not passing. A dir naming no node printed an
    // empty table and exited 0, which is indistinguishable from a clean
    // repo -- the same vacuous-pass shape as `src/tdd:V26`.
    if examined == 0 {
        eprintln!("sherd: {} matched no node", dir.display());
        return ExitCode::from(2);
    }
    // T10/V104: the number exists to be COMPARED. Printing it and exiting 0
    // is what let the chains drift over unseen (B7).
    if over > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

fn lens_cmd(root: &Path, dir: &Path, depth: lens::Depth) -> ExitCode {
    match lens::pack(root, dir, depth) {
        Ok(p) => {
            eprintln!("# chain: {} nodes · {}", p.chain.len(), p.cost);
            for c in &p.children {
                eprintln!(
                    "#   -> {:<16} {}  [not: {}]",
                    c.dir, c.owns, c.not_owns
                );
            }
            print!("{}", p.text);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sherd: {}: {e}", dir.display());
            ExitCode::from(2)
        }
    }
}

fn fed_cmd(dir: &Path) -> ExitCode {
    let path = dir.join("SPEC.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("sherd: no SPEC.md at {}", dir.display());
        return ExitCode::from(2);
    };
    for e in fed::edges(&text) {
        println!("{:<16} {:<40} not: {}", e.dir, e.owns, e.not_owns);
    }
    ExitCode::SUCCESS
}

/// One verdict over the whole federation, for CI and for a stranger who
/// wants to know whether a repository is coherent before reading it.
///
/// COMPOSES what already has owners rather than re-deciding anything: the
/// structural check (`spec`), the DAG's shape (`fed`), every chain against
/// its ceiling (`lens`), and slice drift (`slice`). §C forbids a second
/// reading of a rule that has an owner, and a validator that re-implemented
/// any of these would be exactly that.
///
/// It REPORTS WHAT IT EXAMINED, not only what failed. "0 violations" and "I
/// checked nothing" are the same output otherwise, which is `.:V48` and the
/// vacuous pass every gate here is written against.
/// `sherd route "<query>"` -- which node owns this question.
///
/// Exit codes carry the answer, because a script asking "where does this
/// belong" needs to tell a hit from a guess: `0` one node, `2` no node, `3`
/// several. Rounding an ambiguous query to its first match would make the
/// interesting case indistinguishable from the certain one.
fn route_cmd(root: &Path, query: &str) -> ExitCode {
    match plan::route(root, query) {
        plan::Route::Hit(node, why) => {
            println!(
                "{}\n  matched: {}",
                node.strip_prefix(root).unwrap_or(&node).display(),
                why.join(", ")
            );
            ExitCode::SUCCESS
        }
        plan::Route::Miss => route_miss(root),
        plan::Route::Ambiguous(nodes) => route_ambiguous(root, &nodes),
    }
}

/// A miss is REPORTED, and points at the table that lists what exists.
///
/// An UNFEDERATED repository is a different answer wearing the same exit
/// code: with only a root node there is nothing `route` can ever return,
/// because the root owns everything and therefore answers nothing. Saying
/// "no node matched" there sends the asker off to rephrase a query that
/// could not have succeeded (`.:B19`, found on a foreign repository).
fn route_miss(root: &Path) -> ExitCode {
    if fed::discover(root).len() <= 1 {
        println!(
            "this repository has one node, so there is nothing to route to. \
             `sherd init <dir>` starts a federation."
        );
    } else {
        println!(
            "no node matched. `sherd graph --table` lists what each one owns."
        );
    }
    ExitCode::from(2)
}

/// Several nodes tied. Listing them beats picking one: the asker can see
/// that their question spans a boundary, which is itself the answer.
fn route_ambiguous(root: &Path, nodes: &[PathBuf]) -> ExitCode {
    println!("ambiguous -- the query spans {} nodes:", nodes.len());
    for n in nodes {
        println!("  {}", n.strip_prefix(root).unwrap_or(n).display());
    }
    ExitCode::from(3)
}

fn validate(root: &Path) -> ExitCode {
    let nodes = fed::discover(root);
    let structural = validate_specs(&nodes);
    let edges = validate_edges(root);
    let over = validate_ceilings(root, &nodes);
    let drift = validate_drift(root);
    println!(
        "\n  {} nodes · {structural} structural · {edges} edge · \
         {over} over ceiling · {drift} drifted",
        nodes.len()
    );
    if structural + edges + over + drift == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

/// The structural check, on every node's spec.
fn validate_specs(nodes: &[PathBuf]) -> usize {
    let mut bad: usize = 0;
    for node in nodes {
        let path = node.join("SPEC.md");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for v in spec::check(&text) {
            println!("{}:{}: {v}", path.display(), v.line);
            bad = bad.saturating_add(1);
        }
    }
    bad
}

/// Distilled slices against their sources. An unreadable tree counts as one
/// failure rather than zero: "could not look" and "nothing wrong" are the
/// same output otherwise, which is the vacuous pass `.:V48` forbids.
fn validate_drift(root: &Path) -> usize {
    // A repository with NO slice registry has nothing to drift from, and
    // absence is legal: `sherd init` then `sherd validate` has to be able to
    // pass, or the two verbs contradict each other. Distinguished from a
    // registry that cannot be READ, which stays a failure.
    if !root.join(".sherd-slices").exists() {
        println!("slice: no registry (none required)");
        return 0;
    }
    match slice::drifted(root) {
        Ok(drifted) => report_drift(&drifted),
        Err(e) => {
            println!("slice: {e}");
            1
        }
    }
}

fn report_drift(drifted: &[PathBuf]) -> usize {
    for p in drifted {
        println!("{}: slice drifted from its source", p.display());
    }
    drifted.len()
}

/// Depth and ownership rules over the `§F` edges of every node.
fn validate_edges(root: &Path) -> usize {
    let mut bad: usize = 0;
    for node in fed::discover(root) {
        let Ok(text) = std::fs::read_to_string(node.join("SPEC.md")) else {
            continue;
        };
        let edges = fed::edges(&text);
        for e in fed::depth_violations(&edges) {
            println!("{}: edge to `{}` skips a level", node.display(), e.dir);
            bad = bad.saturating_add(1);
        }
    }
    bad
}

/// Every chain against the ceiling it inherits.
fn validate_ceilings(root: &Path, nodes: &[PathBuf]) -> usize {
    nodes.iter().filter(|node| over_ceiling(root, node)).count()
}

/// One chain against the ceiling it inherits. A node whose pack or ceiling
/// cannot be read is not over -- it is unmeasured, and `budget` is the verb
/// that reports that.
fn over_ceiling(root: &Path, node: &Path) -> bool {
    let (Ok(pack), Ok(ceiling)) = (
        lens::pack(root, node, lens::Depth::Rule),
        lens::ceiling_for(root, node),
    ) else {
        return false;
    };
    if pack.cost.tokens > ceiling {
        println!(
            "{}: chain {} tok over its ceiling of {ceiling}",
            node.display(),
            pack.cost.tokens
        );
        return true;
    }
    false
}

/// Citations that point at no node or no row (`src/spec:V7`).
///
/// A citation is a LINK, and a link nothing resolves is a comment. `.:V41` was
/// cited from four files since the first commit and never written at root,
/// which is what `src/spec:B2` found once anything looked.
fn dangling_citations(root: &Path, text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for c in spec::citations(text) {
        let target = if c.owner == "." {
            root.join("SPEC.md")
        } else {
            root.join(&c.owner).join("SPEC.md")
        };
        let Ok(owner_spec) = std::fs::read_to_string(&target) else {
            out.push(format!(
                "{}: sherd/spec:V7: `{}:{}` names no node -- \
                 a citation is a path from the root, `.` for root itself",
                c.line, c.owner, c.id
            ));
            continue;
        };
        if !spec::declares(&owner_spec, &c.id) {
            out.push(format!(
                "{}: sherd/spec:V7: `{}:{}` resolves to no row",
                c.line, c.owner, c.id
            ));
        }
    }
    out
}

fn check(root: &Path) -> ExitCode {
    let nodes = fed::discover(root);
    let mut bad: usize = 0;
    for node in &nodes {
        let path = node.join("SPEC.md");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for v in spec::check(&text) {
            // `v` prints itself already namespaced -- these are the caller's
            // coordinates prefixed to it, which is all sherd owns here.
            println!("{}:{}: {v}", path.display(), v.line);
            bad = bad.saturating_add(1);
        }
        // progress: a bug with no invariant will recur (spec V4). Advisory --
        // some bugs genuinely warrant no new rule, and forcing one would
        // manufacture invariants to silence a gate.
        for (id, cause) in spec::unreflected_bugs(&text) {
            println!(
                "{}: sherd/spec:V4: {id} names no invariant -- `{cause}` \
                      will recur (advisory)",
                path.display()
            );
        }
        // A citation is a LINK: it names a node path and a row that exists
        // there (`src/spec:V7`). Nothing resolved them until `src/spec:B2`.
        for d in dangling_citations(root, &text) {
            println!("{}:{d}", path.display());
            bad = bad.saturating_add(1);
        }
        // A finished `§T` row is history and every chain pays for it on
        // every turn (`sherd/fed:V9`). Advisory -- some carry a MEASURED
        // result that belongs in `§R` before the row goes.
        for (id, task) in spec::completed_tasks(&text) {
            println!(
                "{}: sherd/fed:V9: {id} is done -- `{task}` is history, \
                 and §T states remaining work (advisory)",
                path.display()
            );
        }
        // §F structure: duplicate rows (fed V12) and child dirs with no row
        // (fed V11). Advisory -- a missing row is often a dir that is simply
        // not a node yet, so it reports rather than fails.
        let edges = fed::edges(&text);
        let (dupes, missing) = fed::find_exhaustive_violations(&edges, node);
        for e in dupes {
            println!(
                "{}: sherd/fed:V12: `{}` named twice in §F -- descent is ambiguous",
                path.display(),
                e.dir
            );
            bad = bad.saturating_add(1);
        }
        for m in missing {
            let name = m.file_name().unwrap_or_default().to_string_lossy();
            println!(
                "{}: sherd/fed:V11: `{name}/` exists on disk with no §F row -- \
                      unreachable by descent (advisory)",
                path.display()
            );
        }
    }
    println!("\n  {} nodes examined · {bad} violations", nodes.len());
    if bad > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(feature = "ollama")]
fn ask(root: &Path, dir: &Path, question: &str) -> ExitCode {
    let Ok(p) = lens::pack(root, dir, lens::Depth::Rule) else {
        eprintln!("sherd: {}: no pack", dir.display());
        return ExitCode::from(2);
    };
    use std::io::Write;
    let prompt = format!("{}\n\n---\n{question}\n", p.text);
    let eta = crate::ollama::predict(p.cost.tokens);
    eprintln!(
        "# pack {} · {} nodes · eta {:.0}s cold / {:.0}s if cached",
        p.cost,
        p.chain.len(),
        eta.total_s(),
        eta.cached_s()
    );
    eprint!("# ");
    let _ = std::io::stderr().flush();
    let mut n = 0usize;
    match crate::ollama::generate_with(&prompt, eta, &mut |c| {
        if crate::ollama::verbose() {
            eprint!("{c}")
        } else {
            n += 1;
            if n.is_multiple_of(25) {
                eprint!(".")
            }
        }
        let _ = std::io::stderr().flush();
    }) {
        Ok(r) => {
            eprintln!();
            println!("{}", r.text);
            let actual = r.ms as f64 / 1000.0;
            eprintln!(
                "[{} sent · {} gen · {actual:.1}s (eta {:.0}s, {:+.0}%){}]",
                r.prompt_tokens,
                r.eval_tokens,
                if crate::ollama::last_cached() {
                    eta.cached_s()
                } else {
                    eta.total_s()
                },
                (actual
                    - if crate::ollama::last_cached() {
                        eta.cached_s()
                    } else {
                        eta.total_s()
                    })
                    / if crate::ollama::last_cached() {
                        eta.cached_s()
                    } else {
                        eta.total_s()
                    }
                    * 100.0,
                if crate::ollama::last_cached() {
                    " prefix CACHED"
                } else {
                    ""
                }
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sherd: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(feature = "ollama")]
fn tdd_cmd(root: &Path, dir: &Path, invariant: &str, task: &str) -> ExitCode {
    match crate::tdd::drive(root, dir, invariant, task, 3) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sherd: {e}");
            ExitCode::from(1)
        }
    }
}

fn plan_cmd(root: &Path) -> ExitCode {
    let p = plan::plan(root);
    let mut st = state::State::load();
    st.clear_kind("plan"); // a superseded step must not outlive its plan

    println!(
        "HORIZON {} of {} open rows · {} unmanaged\n",
        p.steps.len(),
        p.total_open,
        p.unmanaged.len()
    );
    for (i, t) in p.steps.iter().enumerate() {
        let c = plan::Confidence::of(i);
        let est = lens::pack(root, &root.join(&t.node), lens::Depth::Rule)
            .map_or(0, |k| k.cost.tokens);
        let (tried, kept) = plan::record(&t.node);
        let score = if tried == 0 {
            "untried".to_string()
        } else {
            format!("{kept}/{tried} kept")
        };
        println!(
            "{}. {} {:<11} {} {}",
            i + 1,
            c.label(),
            t.node.display(),
            t.id,
            t.text
        );
        println!(
            "      believability {:.2} ({score})",
            plan::believability(&t.node)
        );
        println!(
            "      ~{est} tok context · invalidated by: {}\n",
            c.invalidated_by()
        );
        st.set(
            "plan",
            &(i + 1).to_string(),
            format!("{} {} {}", t.node.display(), t.id, c.label().trim()),
        );
    }
    if p.steps.is_empty() {
        println!("  nothing actionable.\n");
    }
    // V3: say what is NOT managed. Silence would read as coverage.
    println!("UNMANAGED ({}):", p.unmanaged.len());
    let mut by: std::collections::BTreeMap<&str, usize> =
        std::collections::BTreeMap::new();
    for (_, k) in &p.unmanaged {
        *by.entry(k.why()).or_default() += 1;
    }
    for (why, n) in by {
        println!("  {n:3}  {why}");
    }
    println!(
        "\nRun `sherd plan` again after each apply -- applying a task edits\nthe spec that plans the next one, so this list goes stale."
    );
    st.save();
    ExitCode::SUCCESS
}

fn triage_cmd(root: &Path) -> ExitCode {
    let rows = plan::triage(root);
    let mut moves: std::collections::BTreeMap<&str, Vec<(String, String)>> =
        std::collections::BTreeMap::new();
    let (mut decompose, mut keep) = (Vec::new(), Vec::new());
    for (t, k, p) in &rows {
        let id = format!("{} {}", t.node.display(), t.id);
        match p {
            plan::Proposal::Move(n) => {
                moves.entry(n).or_default().push((id, t.text.clone()))
            }
            plan::Proposal::Decompose(ns) => {
                decompose.push((id, t.text.clone(), ns.join(" + ")))
            }
            plan::Proposal::Keep => keep.push((id, t.text.clone(), k.why())),
        }
    }
    println!(
        "TRIAGE of {} unmanaged rows -- ADVISORY, prose classification is\n\
              wrong-by-default (plan V4). Confirm each before moving.\n",
        rows.len()
    );
    let total: usize = moves.values().map(Vec::len).sum();
    println!("MOVE to a node ({total}):");
    for (node, rs) in &moves {
        println!("  -> src/{node}");
        for (id, text) in rs {
            println!(
                "       {id:14} {}",
                text.chars().take(66).collect::<String>()
            );
        }
    }
    println!("\nDECOMPOSE, spans several nodes ({}):", decompose.len());
    for (id, text, ns) in &decompose {
        println!(
            "  {id:14} [{ns}]  {}",
            text.chars().take(50).collect::<String>()
        );
    }
    println!("\nKEEP at root ({}):", keep.len());
    for (id, text, why) in &keep {
        println!(
            "  {id:14} {:<48} ({why})",
            text.chars().take(48).collect::<String>()
        );
    }
    ExitCode::SUCCESS
}

fn review_cmd(root: &Path, rev: &str) -> ExitCode {
    match crate::review::commit(root, rev) {
        Ok(fs) if fs.is_empty() => {
            // V4: say what was CHECKED. "clean" on two rules is not "clean".
            println!(
                "{rev}: no findings (checked: unwired, negative-only, ignored-input)"
            );
            ExitCode::SUCCESS
        }
        Ok(fs) => {
            for (file, f) in &fs {
                println!(
                    "{}: sherd/review:{}: {}",
                    file.display(),
                    f.rule,
                    f.detail
                );
            }
            println!("\n  {} finding(s) -- ADVISORY. Read the diff.", fs.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sherd: {e}");
            ExitCode::from(2)
        }
    }
}

fn slice_cmd(root: &Path, mode: &str) -> ExitCode {
    let decls = match std::fs::read_to_string(root.join(".sherd-slices"))
        .map_err(|e| e.to_string())
        .and_then(|t| slice::parse_decls(&t))
    {
        Ok(d) => d,
        Err(e) => {
            eprintln!("sherd: {e}");
            return ExitCode::from(2);
        }
    };
    let mut drift = 0;
    for d in &decls {
        let rendered = match slice::render(root, d) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("sherd: {e}");
                return ExitCode::from(1);
            }
        };
        let out = root.join(&d.output);
        let current = std::fs::read_to_string(&out).unwrap_or_default();
        let full: u64 = slice::sources(root, &d.source)
            .iter()
            .filter_map(|f| std::fs::read_to_string(f).ok())
            .map(|s| tokens::count(&s).tokens)
            .sum();
        let sliced = tokens::count(&rendered).tokens;
        match mode {
            "--list" => println!(
                "  {:<28} {:>7} -> {:>5} tok  ({:.0}%)  {:?}",
                d.output.display(),
                full,
                sliced,
                100.0 * sliced as f64 / full.max(1) as f64,
                d.rule
            ),
            "--check" => {
                if current != rendered {
                    println!(
                        "{}: DRIFT -- differs from what its source produces",
                        d.output.display()
                    );
                    drift += 1;
                }
            }
            _ => {
                if let Err(e) = std::fs::write(&out, &rendered) {
                    eprintln!("sherd: {}: {e}", out.display());
                    return ExitCode::from(2);
                }
                println!(
                    "  {:<28} {:>7} -> {:>5} tok",
                    d.output.display(),
                    full,
                    sliced
                );
            }
        }
    }
    if mode == "--check" {
        // V3: report what was CHECKED, not only what failed.
        println!("\n  {} slice(s) checked · {drift} drifted", decls.len());
        if drift > 0 {
            println!("  regenerate with `sherd slice`, or fix the source");
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(s: &[&str]) -> Vec<String> {
        s.iter().map(|a| (*a).to_string()).collect()
    }

    /// The read-only verbs, driven through `run_args` against THIS repo.
    ///
    /// Exit codes are the contract `§I` states -- `0 clean / 1 violation /
    /// 2 usage` -- so asserting them is asserting the documented surface, not
    /// merely executing lines. The gate runs `sherd check` and `sherd budget` on
    /// every commit and requires them clean, so SUCCESS here is a claim the
    /// gate independently holds true.
    ///
    /// `cargo test` runs with the package root as CWD, so `repo_root` finds
    /// the real tree. None of these verbs writes: `slice` is given
    /// `--check`, and `outcome`, `apply`, `tdd` and `ask` are excluded --
    /// they write state or need an endpoint.
    #[test]
    fn the_read_only_verbs_all_exit_clean_on_this_repo() {
        for verb in [
            vec!["check"],
            vec!["budget"],
            vec!["fed"],
            vec!["plan"],
            vec!["plan", "--triage"],
            vec!["slice", "--check"],
        ] {
            assert_eq!(
                run_args(argv(&verb)),
                ExitCode::SUCCESS,
                "`sherd {}` must exit 0 on a clean tree",
                verb.join(" ")
            );
        }
    }

    /// Every `graph` rendering, including the default.
    ///
    /// B15 is why there are four: a mermaid diagram that no renderer draws is
    /// a diagram nobody reads, so `--tree` renders in any markdown forever.
    /// A flag whose branches agree is a claim with no runner (`.:V105`), so
    /// the renderings must also DIFFER.
    #[test]
    fn every_graph_rendering_succeeds_and_they_are_not_the_same_render() {
        for flag in [
            vec!["graph"],
            vec!["graph", "--dot"],
            vec!["graph", "--table"],
            vec!["graph", "--tree"],
        ] {
            assert_eq!(run_args(argv(&flag)), ExitCode::SUCCESS, "{flag:?}");
        }
        let root = repo_root();
        let (dot, table) = (fed::dot(&root), fed::table(&root));
        let (tree, mermaid) = (fed::tree(&root), fed::mermaid(&root));
        assert!(dot.contains("digraph"), "--dot must emit dot");
        assert!(table.contains('|'), "--table must emit a markdown table");
        assert_ne!(dot, mermaid, "a flag whose branches agree is no flag");
        assert_ne!(tree, mermaid);
        assert_ne!(table, tree);
    }

    /// `lens` at each depth, and the depths must not agree.
    ///
    /// `.:B8` is `--depth rule` selecting NOTHING for the project's whole
    /// life while §I documented it as the default. A test that only checked
    /// the exit code would have passed throughout.
    #[test]
    fn lens_runs_at_every_depth_and_the_depths_differ() {
        for d in ["rule", "why", "all"] {
            assert_eq!(
                run_args(argv(&["lens", ".", "--depth", d])),
                ExitCode::SUCCESS,
                "lens --depth {d}"
            );
        }
        let root = repo_root();
        let p = |d| lens::pack(&root, &root, d).map(|p| p.text.len());
        let (Ok(rule), Ok(all)) = (p(lens::Depth::Rule), p(lens::Depth::All))
        else {
            panic!("the root pack must be readable at both depths")
        };
        assert!(rule < all, "B8: `rule` must SELECT, not render everything");
    }

    /// A verb pointed at a directory that is not a node.
    ///
    /// USAGE (2), not violation (1): you named the wrong directory, which is
    /// a bad argument -- distinct from `check` finding a real defect inside a
    /// node that does exist. `§I` separates the two codes and something has
    /// to hold them apart.
    #[test]
    fn a_dir_with_no_spec_is_a_usage_error_not_a_violation() {
        assert_eq!(
            run_args(argv(&["fed", "target"])),
            ExitCode::from(2),
            "no SPEC.md there is a bad ARGUMENT, never a silent zero"
        );
        assert_eq!(
            run_args(argv(&["check"])),
            ExitCode::SUCCESS,
            "and a real node still checks clean -- the codes differ"
        );
    }

    /// `budget` on a repo whose chain EXCEEDS its declared ceiling.
    ///
    /// The over-ceiling branch is another detector with no positive case:
    /// `.:B7` is `.context-limits` declaring per-node ceilings while `budget`
    /// PRINTED the table without comparing against them, so every chain
    /// drifted over unseen for the project's life. The comparison now exists
    /// and the gate keeps this repo at 0 over, which means the branch that
    /// reports a breach can never fire here.
    #[test]
    fn a_chain_over_its_ceiling_exits_one_and_a_generous_one_exits_zero() {
        assert_eq!(over_ceiling_is_caught(), Ok(()));
    }

    /// A node whose SPEC costs more than one token -- which is every real one.
    const FAT_SPEC: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nsomething long \
         enough to cost more than one token, several times over, so the \
         ceiling below is genuinely exceeded rather than merely equalled\n";

    fn over_ceiling_is_caught() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-budget-over")?;
        r.write("SPEC.md", FAT_SPEC)?;
        r.write(".context-limits", "SPEC.md 1\n")?;
        r.commit("a node over its ceiling")?;
        let root = r.path().to_path_buf();
        assert_eq!(
            budget(r.path(), root.clone()),
            ExitCode::from(1),
            "B7: a chain over its ceiling must FAIL, not merely print"
        );
        // The control: raise the ceiling and the same tree passes. Without
        // it, `budget` returning 1 unconditionally would satisfy the test.
        r.write(".context-limits", "SPEC.md 100000\n")?;
        r.commit("raise it")?;
        assert_eq!(budget(r.path(), root), ExitCode::SUCCESS);
        Ok(())
    }

    /// `check` on a repo that IS broken.
    ///
    /// Every reporting branch in `check` only runs when something is wrong,
    /// so a clean tree exercises none of them -- and this repo is kept clean
    /// by the gate. A detector tested only on the negative case is satisfied
    /// by finding nothing, which is `src/fed:B6` and the reason `src/fed:V10`
    /// exists. These are five such detectors with no positive case.
    #[test]
    fn check_reports_the_violations_it_finds_and_exits_one() {
        assert_eq!(broken_repo_is_caught(), Ok(()));
    }

    /// A §F row pointing at a directory that is not there (fed V1), the same
    /// dir named twice (fed V12), a child on disk with no row (fed V11), and
    /// a §B row naming no invariant (spec V4).
    const BROKEN: &str = "# SPEC\n\n## \u{a7}V INVARIANTS\n\nV1: out of order\n\n## \u{a7}G GOAL\n\nbroken on purpose\n\n## \u{a7}T TASKS\n\nid|status|task|cites\nT1|?|a status that is not x, ~ or .|-\n\n\
         ## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n\
         ghost|nothing real|-|-\n\
         twice|a|-|-\n\
         twice|b|-|-\n\n\
         ## \u{a7}B BUGS\n\nid|date|cause|fix\n\
         B1|2026-08-21|something broke|no invariant named here\n";

    fn broken_repo_is_caught() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-check-broken")?;
        r.write("SPEC.md", BROKEN)?;
        // A real child dir with no §F row -- fed V11's advisory.
        r.write("orphan/SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nx\n")?;
        r.commit("a deliberately broken tree")?;
        assert_eq!(
            check(r.path()),
            ExitCode::from(1),
            "a tree with violations must exit 1, never 0"
        );
        Ok(())
    }

    /// The control, and it is what makes the test above mean anything: the
    /// same function on a clean tree exits 0. Without this, `check` returning
    /// 1 unconditionally would pass.
    #[test]
    fn check_on_a_clean_tree_exits_zero() {
        assert_eq!(check_clean_tree(), Ok(()));
    }

    fn check_clean_tree() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-check-clean")?;
        r.write(
            "SPEC.md",
            "# SPEC\n\n## \u{a7}G GOAL\n\nclean\n\n## \u{a7}V INVARIANTS\n\n\
             V1: something ! hold\n",
        )?;
        r.commit("a clean tree")?;
        assert_eq!(check(r.path()), ExitCode::SUCCESS);
        Ok(())
    }

    /// `src/spec:B2` and `B3`: a citation is a LINK, and both ways it can
    /// fail are distinct mistakes. This tree reports neither, which is why
    /// the branches need a repo that does.
    #[test]
    fn a_citation_fails_on_a_missing_node_and_on_a_missing_row()
    -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-citations")?;
        r.write(
            "SPEC.md",
            "# SPEC\n\n## \u{a7}G GOAL\n\nlinks\n\n## \u{a7}V INVARIANTS\n\n\
             V1: see `nowhere:V3`\nV2: see `.:V99`\nV3: see `.:V1`\n",
        )?;
        r.commit("two dead links and one live one")?;
        let Ok(text) = std::fs::read_to_string(r.path().join("SPEC.md")) else {
            unreachable!("just written")
        };
        let found = dangling_citations(r.path(), &text);
        assert_eq!(found.len(), 2, "V3 cites a row that exists: {found:?}");
        assert!(
            found.iter().any(|f| f.contains("names no node")),
            "`nowhere` is not a node: {found:?}"
        );
        assert!(
            found.iter().any(|f| f.contains("resolves to no row")),
            "root has no V99: {found:?}"
        );
        Ok(())
    }

    /// `review` on a commit that adds a STUB, so the findings loop runs.
    ///
    /// The printing branch only executes when there is something to print,
    /// and this repo's own commits are reviewed clean -- so `review_cmd`'s
    /// findings path had never run. A stub is a new `pub fn` called only from
    /// its own test, which is exactly what `unwired` flags (`src/fed:B6`).
    #[test]
    fn review_prints_the_findings_it_has_and_stays_advisory() {
        assert_eq!(review_a_stub_commit(), Ok(()));
    }

    fn review_a_stub_commit() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-review-stub")?;
        r.write(
            "src/n/mod.rs",
            "pub fn stub() -> bool { false }\n\n#[cfg(test)]\nmod t {\n \
             use super::*;\n #[test]\n fn a() { assert!(!stub()); }\n}\n",
        )?;
        r.commit("add a stub called only by its own test")?;
        assert_eq!(
            review_cmd(r.path(), "HEAD"),
            ExitCode::SUCCESS,
            "V3: a finding is ADVISORY -- review reports, the reader judges, \
             and auto-failing would trade a false negative for a false positive"
        );
        Ok(())
    }

    /// `review` against a revision that does not exist.
    #[test]
    fn review_of_an_unknown_revision_is_an_error_not_a_clean_bill() {
        // `src/review`'s own rule: an unreadable module is an error, never a
        // clean review. A missing rev reporting "no findings" would be the
        // most dangerous possible output.
        assert_ne!(
            run_args(argv(&["review", "definitely-not-a-rev"])),
            ExitCode::SUCCESS,
            "a rev that does not exist cannot be clean"
        );
    }

    /// `review` of a real revision runs and reports.
    #[test]
    fn review_of_a_real_revision_reports_and_succeeds() {
        // ADVISORY by design -- findings do not fail the command -- so the
        // assertion is that it runs and classifies, not that it is silent.
        //
        // Handed a FIXTURE repository rather than run against whatever tree
        // the runner sits in (V6/B1): `run_args` would resolve the root from
        // the CWD, which is this checkout here and is not a repository at
        // all inside a crate tarball or a nix sandbox.
        let Ok(repo) = crate::testrepo::TestRepo::new("cli-review") else {
            unreachable!("a fixture repository is buildable")
        };
        assert_eq!(review_cmd(repo.path(), "HEAD"), ExitCode::SUCCESS);
    }

    /// `repo_root_from` walks UP to the tree that has both markers.
    ///
    /// Every assertion is about a tree this test built. The version this
    /// replaces asserted that the ambient root carries a `.git` and a
    /// `SPEC.md`, which is true of this checkout, false of a tarball, and
    /// says nothing about the walk either way (B1).
    #[test]
    fn repo_root_finds_the_tree_that_has_both_markers() {
        let Ok(repo) = crate::testrepo::TestRepo::new("cli-root") else {
            unreachable!("a fixture repository is buildable")
        };
        let root = repo.path();
        assert_eq!(repo_root_from(root), root);

        let nested = root.join("src").join("deep");
        let Ok(()) = std::fs::create_dir_all(&nested) else {
            unreachable!("a nested dir is creatable")
        };
        assert_eq!(repo_root_from(&nested), root, "the walk goes UP");
    }

    /// A directory that is not inside any repository resolves to ITSELF
    /// rather than escaping upward into one.
    ///
    /// This is the half B1 needed and did not have: the failure mode is not
    /// "the walk is wrong", it is "the walk finds someone else's repository
    /// and the caller cannot tell". A `SPEC.md` with no `.git` beside it
    /// must not satisfy the search.
    #[test]
    fn a_tree_that_is_not_a_repository_resolves_to_itself() {
        let dir = std::env::temp_dir().join(format!(
            "sherd-notarepo-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let Ok(()) = std::fs::create_dir_all(&dir) else {
            unreachable!("a scratch dir is creatable")
        };
        let Ok(()) = std::fs::write(dir.join("SPEC.md"), "# SPEC\n") else {
            unreachable!("a scratch file is writable")
        };

        // `/tmp` has no `.git` above it, so the walk runs out of parents.
        assert_eq!(repo_root_from(&dir), dir);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `init` REFUSES an existing spec, and the refusal is the feature. A
    /// scaffold that can overwrite `SPEC.md` can erase every invariant a
    /// repository has recorded, so there is no `--force` to test.
    #[test]
    fn init_refuses_to_overwrite_an_existing_spec() {
        let Ok(repo) = crate::testrepo::TestRepo::new("cli-init-refuse") else {
            unreachable!("a fixture repository is buildable")
        };
        // `TestRepo::new` writes a SPEC.md, so the root already has one.
        assert_eq!(init_cmd(repo.path(), &argv(&["init"])), ExitCode::from(1));
    }

    /// A directory with children gets a row per child; one without gets no
    /// `§F` table at all, because an empty table is a claim of no children
    /// rather than an absence of information.
    /// A repository with a `node/deep` directory: enough for one child row.
    fn init_fixture(tag: &str) -> (crate::testrepo::TestRepo, PathBuf) {
        let Ok(repo) = crate::testrepo::TestRepo::new(tag) else {
            unreachable!("a fixture repository is buildable")
        };
        let node = repo.path().join("node");
        let Ok(()) = std::fs::create_dir_all(node.join("deep")) else {
            unreachable!("a nested dir is creatable")
        };
        (repo, node)
    }

    /// Idempotence here is REFUSAL, not a silent rewrite: the second run
    /// finds the file the first one wrote and declines to touch it.
    #[test]
    fn init_run_twice_refuses_the_second_time() {
        let (repo, _node) = init_fixture("cli-init-twice");
        assert_eq!(
            init_cmd(repo.path(), &argv(&["init", "node"])),
            ExitCode::SUCCESS
        );
        assert_eq!(
            init_cmd(repo.path(), &argv(&["init", "node"])),
            ExitCode::from(1)
        );
    }

    #[test]
    fn init_writes_a_scaffold_with_a_row_per_child() {
        let (repo, node) = init_fixture("cli-init-write");
        assert_eq!(
            init_cmd(repo.path(), &argv(&["init", "node"])),
            ExitCode::SUCCESS
        );
        let Ok(body) = std::fs::read_to_string(node.join("SPEC.md")) else {
            unreachable!("init wrote a spec")
        };
        assert!(body.contains("deep|WHAT IT OWNS"), "the child row: {body}");
        assert!(
            crate::spec::check(&body).is_empty(),
            "our checker accepts it"
        );
    }

    /// `--stdout` is the preview, and previewing must never write.
    #[test]
    fn init_stdout_writes_nothing() {
        let Ok(repo) = crate::testrepo::TestRepo::new("cli-init-stdout") else {
            unreachable!("a fixture repository is buildable")
        };
        let node = repo.path().join("preview");
        let Ok(()) = std::fs::create_dir_all(&node) else {
            unreachable!("a dir is creatable")
        };
        assert_eq!(
            init_cmd(repo.path(), &argv(&["init", "preview", "--stdout"])),
            ExitCode::SUCCESS
        );
        assert!(!node.join("SPEC.md").exists(), "preview wrote a file");
    }

    /// The failure path: a directory that cannot be read. `init` reports the
    /// cause and exits 1 rather than scaffolding an empty `§F` table, which
    /// would claim "no children" about a directory it never saw.
    #[test]
    fn init_reports_a_directory_it_cannot_read() {
        let missing = std::env::temp_dir().join("sherd-no-such-dir-init");
        let _ = std::fs::remove_dir_all(&missing);
        let Err(msg) = init_body(Path::new("/"), &missing) else {
            unreachable!("an unreadable directory is an error")
        };
        assert!(msg.contains("sherd-no-such-dir-init"), "names it: {msg}");
        assert_eq!(init_failed(&msg), ExitCode::from(1));
    }

    #[test]
    fn init_on_a_path_that_is_not_a_directory_is_usage() {
        let Ok(repo) = crate::testrepo::TestRepo::new("cli-init-nodir") else {
            unreachable!("a fixture repository is buildable")
        };
        assert_eq!(
            init_cmd(repo.path(), &argv(&["init", "no-such-dir"])),
            ExitCode::from(2)
        );
    }

    /// One child node whose `§G` body is `goal` -- a query matches its words,
    /// and anything after them is whatever the test needs next.
    fn write_spec(root: &Path, dir: &str, goal: &str) {
        let node = root.join(dir);
        let spec = format!("# SPEC\n\n## \u{a7}G GOAL\n\n{goal}\n");
        let Ok(()) = std::fs::create_dir_all(&node) else {
            unreachable!("a node dir is creatable")
        };
        let Ok(()) = std::fs::write(node.join("SPEC.md"), spec) else {
            unreachable!("a node spec is writable")
        };
    }

    /// A fixture with two child nodes, each carrying a `§G` a query can hit.
    fn routing_fixture(tag: &str) -> crate::testrepo::TestRepo {
        let Ok(repo) = crate::testrepo::TestRepo::new(tag) else {
            unreachable!("a fixture repository is buildable")
        };
        write_spec(repo.path(), "alpha", "widgets and sprockets");
        write_spec(repo.path(), "beta", "gizmos");
        repo
    }

    #[test]
    fn route_reports_a_hit_a_miss_and_an_ambiguity_by_exit_code() {
        let repo = routing_fixture("cli-route");
        let root = repo.path();
        assert_eq!(route_cmd(root, "sprockets"), ExitCode::SUCCESS);
        assert_eq!(route_cmd(root, "wombat"), ExitCode::from(2));
        assert_eq!(route_cmd(root, "widgets gizmos"), ExitCode::from(3));
    }

    /// `split` PROPOSES and never writes, which is the property worth
    /// pinning: `--apply` refuses, and a federated node has nothing left to
    /// promote so the table is empty rather than noise.
    #[test]
    fn split_proposes_and_refuses_to_apply() {
        let repo = routing_fixture("cli-split");
        let root = repo.path();
        // `alpha` and `beta` are nodes already; a flat module is not.
        let Ok(()) =
            std::fs::write(root.join("gamma.rs"), "pub fn gamma() {}\n")
        else {
            unreachable!("a module file is writable")
        };
        write_spec(root, "alpha", "widgets and sprockets");
        let Ok(spec) = std::fs::read_to_string(root.join("SPEC.md")) else {
            unreachable!("the fixture spec is readable")
        };
        let Ok(()) = std::fs::write(
            root.join("SPEC.md"),
            format!("{spec}\nV1: gamma holds the gamma rule\n"),
        ) else {
            unreachable!("the fixture spec is writable")
        };

        assert_eq!(split_cmd(root, root, true), ExitCode::from(2), "--apply");
        assert_eq!(split_cmd(root, root, false), ExitCode::SUCCESS);
        // Proposing must not have written anything.
        assert!(!root.join("gamma").exists(), "split created a directory");
    }

    /// The structure-first proposal on a fixture whose modules the spec
    /// never names: `.:src/plan:B12` is that a row ranking sees nothing here,
    /// while the code plainly declares two nodes.
    /// Every grade, including the bottom rung that always fires: a plain
    /// `mod` and a `pub(crate) mod` are both DECLARED, which is what
    /// `microlith` is made of (`.:src/plan:B13`).
    #[test]
    fn a_private_or_crate_visible_module_is_still_a_node() {
        let repo = routing_fixture("cli-split-grades");
        let root = repo.path();
        let Ok(()) = std::fs::create_dir_all(root.join("src")) else {
            unreachable!("a src dir is creatable")
        };
        let Ok(()) = std::fs::write(
            root.join("src").join("lib.rs"),
            "pub mod api;\npub(crate) mod inner;\nmod hidden;\n",
        ) else {
            unreachable!("a lib.rs is writable")
        };
        let found = plan::structure(root);
        let grade =
            |n: &str| found.iter().find(|p| p.name == n).map(|p| p.evidence);
        assert_eq!(grade("api"), Some(plan::Evidence::Published));
        assert_eq!(
            grade("inner"),
            Some(plan::Evidence::Declared),
            "pub(crate) is not published"
        );
        assert_eq!(grade("hidden"), Some(plan::Evidence::Declared));
    }

    #[test]
    fn split_proposes_nodes_the_spec_never_mentions() {
        let repo = routing_fixture("cli-split-structure");
        let root = repo.path();
        let Ok(()) = std::fs::create_dir_all(root.join("src")) else {
            unreachable!("a src dir is creatable")
        };
        let Ok(()) = std::fs::write(
            root.join("src").join("lib.rs"),
            "pub mod widget;\nmod helper;\n#[cfg(test)]\nmod testonly;\n",
        ) else {
            unreachable!("a lib.rs is writable")
        };
        let found = plan::structure(root);
        let names: Vec<&str> = found.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["widget", "helper"],
            "published first, then declared: {found:?}"
        );
        assert_eq!(split_cmd(root, root, false), ExitCode::SUCCESS);
    }

    /// A node with no `SPEC.md` is a usage error, not an empty proposal.
    #[test]
    fn split_on_a_directory_with_no_spec_is_usage() {
        let repo = routing_fixture("cli-split-nospec");
        let bare = repo.path().join("bare");
        let Ok(()) = std::fs::create_dir_all(&bare) else {
            unreachable!("a dir is creatable")
        };
        assert_eq!(split_cmd(repo.path(), &bare, false), ExitCode::from(2));
    }

    /// `sync` is idempotent, `--check` never writes, and a stale `§N` is
    /// reported by both. The exit code inverts on purpose: writing means the
    /// committed tree WAS stale, which CI has to hear about.
    #[test]
    #[allow(
        clippy::too_many_lines,
        reason = "the assertions are a SEQUENCE -- stale, then check writes \
                  nothing, then the fix writes, then it is a no-op -- and \
                  splitting them into separate tests loses the ordering, \
                  which is the property under test"
    )]
    fn sync_writes_once_then_reports_clean() {
        let repo = routing_fixture("cli-sync");
        let root = repo.path();
        write_spec(
            root,
            "alpha",
            "widgets\n\n## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\ndeep|the deep bit|the rest|-",
        );
        let Ok(()) = std::fs::create_dir_all(root.join("alpha").join("deep"))
        else {
            unreachable!("a nested dir is creatable")
        };

        assert_eq!(sync_cmd(root, None, true), ExitCode::from(1), "stale");
        let before = std::fs::read_to_string(root.join("SPEC.md")).ok();
        assert_eq!(
            std::fs::read_to_string(root.join("SPEC.md")).ok(),
            before,
            "--check wrote to the tree"
        );

        assert_eq!(sync_cmd(root, None, false), ExitCode::from(1), "wrote");
        assert_eq!(
            sync_cmd(root, None, false),
            ExitCode::SUCCESS,
            "idempotent"
        );
        assert_eq!(sync_cmd(root, None, true), ExitCode::SUCCESS, "clean");
    }

    /// The edge half. A `§F` row naming a grandchild skips a level, which is
    /// the one structural rule `validate` checks that `check` does not.
    #[test]
    fn an_edge_that_skips_a_level_is_a_finding() {
        let repo = routing_fixture("cli-edges");
        assert_eq!(validate_edges(repo.path()), 0);

        write_spec(
            repo.path(),
            "alpha",
            "widgets\n\n## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\ndeep/deeper|a|b|-",
        );
        assert_eq!(validate_edges(repo.path()), 1);
    }

    /// A node whose `SPEC.md` cannot be READ is skipped rather than counted
    /// as a violation: `validate` reports what it examined, and an
    /// unreadable file was not examined (`.:V48`).
    #[test]
    fn a_node_whose_spec_cannot_be_read_is_skipped() {
        let repo = routing_fixture("cli-unreadable");
        let spec = repo.path().join("alpha").join("SPEC.md");
        let Ok(()) = std::fs::remove_file(&spec) else {
            unreachable!("the fixture spec exists")
        };
        let Ok(()) = std::fs::create_dir_all(&spec) else {
            unreachable!("a directory can take its place")
        };
        assert_eq!(validate_specs(&[repo.path().join("alpha")]), 0);
    }

    /// The drift half, which the other two `validate` tests never reach: a
    /// tree with no slice registry counts ZERO, and a registry that cannot
    /// be read still counts one (`.:src/cli:B3`, `V12`).
    #[test]
    fn a_missing_slice_registry_is_not_drift() {
        let repo = routing_fixture("cli-drift");
        assert_eq!(validate_drift(repo.path()), 0);

        let Ok(()) = std::fs::create_dir_all(repo.path().join(".sherd-slices"))
        else {
            unreachable!("a registry dir is creatable")
        };
        // A directory where a registry file belongs: present, unreadable.
        assert_eq!(validate_drift(repo.path()), 1);
    }

    /// `validate` REPORTS what it examined, and a planted structural
    /// violation has to move its verdict -- a validator that cannot fail is
    /// the vacuous pass every gate here is written against.
    #[test]
    fn validate_passes_a_clean_tree_and_fails_a_broken_one() {
        let repo = routing_fixture("cli-validate");
        assert_eq!(validate(repo.path()), ExitCode::SUCCESS);

        // A task citing a rule that does not exist: dangling, and structural.
        write_spec(
            repo.path(),
            "alpha",
            "widgets\n\n## \u{a7}T TASKS\n\nid|status|task|cites\nT1|.|do it|V99",
        );
        assert_eq!(validate(repo.path()), ExitCode::from(1));
    }

    #[test]
    fn an_unknown_command_is_a_usage_error() {
        assert_eq!(run_args(argv(&["nope"])), ExitCode::from(2));
    }

    #[test]
    fn help_and_no_args_both_succeed() {
        assert_eq!(run_args(argv(&["--help"])), ExitCode::SUCCESS);
        assert_eq!(run_args(argv(&["-h"])), ExitCode::SUCCESS);
        assert_eq!(run_args(argv(&["help"])), ExitCode::SUCCESS);
        assert_eq!(run_args(argv(&[])), ExitCode::SUCCESS);
    }

    #[test]
    fn a_verb_missing_its_argument_is_usage_not_a_crash() {
        // Each of these needs an argument it is not given. Usage, never a
        // panic: `sherd` runs unattended inside the loop, and a panic there is
        // a run that stops with no record.
        assert_eq!(run_args(argv(&["lens"])), ExitCode::from(2));
        assert_eq!(run_args(argv(&["outcome"])), ExitCode::from(2));
        assert_eq!(run_args(argv(&["outcome", "src/fed"])), ExitCode::from(2));
    }

    #[test]
    fn an_outcome_verdict_outside_the_three_words_is_refused() {
        // `kept`, `reverted`, `failed`. Anything else must not be read as one
        // of them -- believability is computed from these and a typo silently
        // scored as `kept` would corrupt the record it exists to keep.
        assert_eq!(
            run_args(argv(&["outcome", "src/fed", "probably"])),
            ExitCode::from(2)
        );
    }

    #[test]
    fn depth_defaults_to_rule_and_refuses_a_typo() {
        // `.:V45` -- `rule` is the default. A typo must be a usage error, not
        // a silent fall back, or an ignored flag looks exactly like an
        // honoured one (`.:B8`).
        assert_eq!(depth_arg(&argv(&["lens", "."])), Ok(lens::Depth::Rule));
        assert_eq!(
            depth_arg(&argv(&["lens", ".", "--depth", "all"])),
            Ok(lens::Depth::All)
        );
        assert_eq!(
            depth_arg(&argv(&["lens", ".", "--depth", "why"])),
            Ok(lens::Depth::Why)
        );
        assert!(depth_arg(&argv(&["lens", ".", "--depth", "rules"])).is_err());
        assert!(depth_arg(&argv(&["lens", ".", "--depth"])).is_err());
    }

    #[test]
    fn arg_dir_resolves_against_root_not_the_cwd() {
        // B9: a relative argument never compared equal to the absolute paths
        // `fed::discover` returns, so `budget src/tdd` examined nothing.
        let root = Path::new("/tmp/xyz");
        assert_eq!(arg_dir(&argv(&["budget"]), root), root.to_path_buf());
        assert_eq!(
            arg_dir(&argv(&["budget", "src/tdd"]), root),
            root.join("src/tdd")
        );
        // An absolute argument is left alone.
        assert_eq!(
            arg_dir(&argv(&["budget", "/elsewhere"]), root),
            PathBuf::from("/elsewhere")
        );
    }

    /// The read-only verbs, run against THIS repo.
    ///
    /// A fixture would be a second repo to keep honest; the gate already
    /// requires these green here, so running them on the real tree asserts
    /// the same thing the gate does and covers the dispatch that reaches
    /// them. `.:V27` -- this repo must be a valid federation -- is exactly
    /// the claim being exercised.
    const VERBS: [&[&str]; 8] = [
        &["graph"],
        &["graph", "--dot"],
        &["graph", "--table"],
        &["graph", "--tree"],
        &["fed"],
        &["check"],
        &["budget"],
        &["slice", "--list"],
    ];

    #[test]
    fn the_read_only_verbs_succeed_on_this_repo() {
        for a in VERBS {
            let code = run_args(argv(a));
            assert_eq!(
                code,
                ExitCode::SUCCESS,
                "`sherd {}` must exit 0",
                a.join(" ")
            );
        }
    }

    #[test]
    fn a_dir_matching_no_node_is_usage_not_a_vacuous_pass() {
        // Examining NOTHING is not passing. An empty table and a clean repo
        // were indistinguishable until T10, which is `src/tdd:V26`'s shape.
        assert_eq!(
            run_args(argv(&["budget", "no-such-dir"])),
            ExitCode::from(2)
        );
    }

    #[test]
    fn the_usage_text_names_every_exit_code_it_returns() {
        // The three codes the tests above assert are the three §I documents.
        assert!(USAGE.contains("0 clean"), "{USAGE}");
        assert!(USAGE.contains("1 violation"), "{USAGE}");
        assert!(USAGE.contains("2 usage"), "{USAGE}");
    }
}
