//! Arg dispatch and exit codes. The binary is a shim over `run` (`.:V41`).
//!
//! A node, not a loose `main.rs`, because exit codes and usage are real
//! contracts and every §T row about them was unreachable while this file had
//! no `SPEC.md` to hold them.

use crate::{fed, lens, plan, slice, spec, state, tokens};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
bbx -- federated SPEC.md for small-context local models

  bbx budget [dir]     token cost of every node, against the working budget
  bbx lens <dir> [--depth rule|why|all]  the context pack for one node
  bbx fed [dir]        the federation edges declared by a node
  bbx check [dir]      microlith structural check of every node
  bbx review [rev]     mechanical checks on what a commit added (default HEAD)
  bbx slice [--check|--list]  regenerate distilled slices from their sources
  bbx outcome <node> <kept|reverted>  record whether a node's work survived review
  bbx graph [--tree|--table|--dot]  federation DAG, generated from §F
  bbx plan             next 3 steps, with what would invalidate each
  bbx plan --triage    unmanaged rows, with a proposed home for each
  bbx apply [--land]   execute step 1 only, commit it to a run branch, stop
  bbx land [--push]    fast-forward main to this run branch, if it earned it
  bbx ask <dir> <q>    ask the endpoint from a node's lens pack
  bbx tdd <dir> <Vn> <task>   red -> judge -> green -> gate -> repair

  -v, --verbose        dump every prompt and stream every reply

exit: 0 clean · 1 violation · 2 usage";

/// Parse argv and dispatch. The binary itself holds nothing (`.:V41`).
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
                    "\napplied as {sha}. Run `bbx plan` again before the next step -- \
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
                eprintln!("bbx: {e}");
                ExitCode::from(1)
            }
        },
        Some("land") => land_verb(&root, args.iter().any(|a| a == "--push")),
        #[cfg(feature = "ollama")]
        Some("ask") => match (args.get(1), args.get(2)) {
            (Some(_), Some(q)) => ask(&root, &arg_dir(&args, &root), q),
            _ => usage("ask needs <dir> and a question"),
        },
        #[cfg(feature = "ollama")]
        Some("oneshot") => match (args.get(1), args.get(2), args.get(3)) {
            (Some(_), Some(v), Some(t)) => {
                match crate::tdd::oneshot(&root, &arg_dir(&args, &root), v, t) {
                    Ok(_) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("bbx: {e}");
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
/// `bbx` is a shim over `cargo run`, so CWD is wherever you typed it. Using
/// CWD federated from a SUBDIRECTORY silently -- fewer nodes, a truncated
/// chain, and no error to say so. Walk up to the git root instead.
fn repo_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut d = cwd.as_path();
    loop {
        if d.join(".git").exists() && d.join("SPEC.md").is_file() {
            return d.to_path_buf();
        }
        match d.parent() {
            Some(p) => d = p,
            None => return cwd,
        }
    }
}

/// The `[dir]` argument, resolved AGAINST ROOT.
///
/// It used to be handed back verbatim, so `src/tdd` stayed relative while
/// `fed::discover` returns absolute paths -- the two never compared equal.
/// `bbx fed` survived that only because the CWD happens to be the repo root;
/// from a subdirectory it read the wrong `SPEC.md` or none.
///
/// `join` leaves an absolute argument alone, so passing a full path still
/// works.
/// `--depth rule|why|all`, defaulting to `rule` (`.:V45`).
///
/// An unknown value is a USAGE error, never a quiet fall back to the default.
/// `bbx lens x --depth rules` would otherwise look exactly like a flag that
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
            eprintln!("bbx: not landing -- {e}");
            eprintln!(
                "     the branch is untouched; it is the record of the try"
            );
            ExitCode::from(1)
        }
    }
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("bbx: {msg}\n\n{USAGE}");
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
        // §I declares `bbx budget [dir]`. The argument was parsed by
        // `arg_dir` and then dropped, so every invocation reported the whole
        // repo -- an interface promised and unread, which is `.:V104`'s own
        // shape appearing in the command that enforces it.
        if !node.starts_with(&dir) {
            continue;
        }
        let p = match lens::pack(root, node, lens::Depth::Rule) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("bbx: {}: {e}", node.display());
                return ExitCode::from(1);
            }
        };
        // Re-read per node rather than hoisting the load: the key mapping
        // lives in `ceiling_for` and copying it here to save twelve reads of
        // a one-kilobyte file would be two readings of one rule.
        let ceiling = match lens::ceiling_for(root, node) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("bbx: {e}");
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
        eprintln!("bbx: {} matched no node", dir.display());
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
            eprintln!("bbx: {}: {e}", dir.display());
            ExitCode::from(2)
        }
    }
}

fn fed_cmd(dir: &Path) -> ExitCode {
    let path = dir.join("SPEC.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("bbx: no SPEC.md at {}", dir.display());
        return ExitCode::from(2);
    };
    for e in fed::edges(&text) {
        println!("{:<16} {:<40} not: {}", e.dir, e.owns, e.not_owns);
    }
    ExitCode::SUCCESS
}

fn check(root: &Path) -> ExitCode {
    let nodes = fed::discover(root);
    let mut bad = 0;
    for node in &nodes {
        let path = node.join("SPEC.md");
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        for v in spec::check(&text) {
            // `v` prints itself already namespaced -- these are the caller's
            // coordinates prefixed to it, which is all bbx owns here.
            println!("{}:{}: {v}", path.display(), v.line);
            bad += 1;
        }
        // progress: a bug with no invariant will recur (spec V4). Advisory --
        // some bugs genuinely warrant no new rule, and forcing one would
        // manufacture invariants to silence a gate.
        for (id, cause) in spec::unreflected_bugs(&text) {
            println!(
                "{}: bbx/spec:V4: {id} names no invariant -- `{cause}` \
                      will recur (advisory)",
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
                "{}: bbx/fed:V12: `{}` named twice in §F -- descent is ambiguous",
                path.display(),
                e.dir
            );
            bad += 1;
        }
        for m in missing {
            let name = m.file_name().unwrap_or_default().to_string_lossy();
            println!(
                "{}: bbx/fed:V11: `{name}/` exists on disk with no §F row -- \
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
        eprintln!("bbx: {}: no pack", dir.display());
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
            eprintln!("bbx: {e}");
            ExitCode::from(2)
        }
    }
}

#[cfg(feature = "ollama")]
fn tdd_cmd(root: &Path, dir: &Path, invariant: &str, task: &str) -> ExitCode {
    match crate::tdd::drive(root, dir, invariant, task, 3) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bbx: {e}");
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
        "\nRun `bbx plan` again after each apply -- applying a task edits\nthe spec that plans the next one, so this list goes stale."
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
                    "{}: bbx/review:{}: {}",
                    file.display(),
                    f.rule,
                    f.detail
                );
            }
            println!("\n  {} finding(s) -- ADVISORY. Read the diff.", fs.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("bbx: {e}");
            ExitCode::from(2)
        }
    }
}

fn slice_cmd(root: &Path, mode: &str) -> ExitCode {
    let decls = match std::fs::read_to_string(root.join(".bbx-slices"))
        .map_err(|e| e.to_string())
        .and_then(|t| slice::parse_decls(&t))
    {
        Ok(d) => d,
        Err(e) => {
            eprintln!("bbx: {e}");
            return ExitCode::from(2);
        }
    };
    let mut drift = 0;
    for d in &decls {
        let rendered = match slice::render(root, d) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("bbx: {e}");
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
                    eprintln!("bbx: {}: {e}", out.display());
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
            println!("  regenerate with `bbx slice`, or fix the source");
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

    /// Exit codes are a CONTRACT (`§I`: 0 clean · 1 violation · 2 usage), and
    /// a contract nothing calls is a comment. This node sat at 0.0% coverage
    /// over 445 lines because `run` read `std::env::args` directly.
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
        // panic: `bbx` runs unattended inside the loop, and a panic there is
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
    #[test]
    fn the_read_only_verbs_succeed_on_this_repo() {
        for a in [
            vec!["graph"],
            vec!["graph", "--dot"],
            vec!["graph", "--table"],
            vec!["graph", "--tree"],
            vec!["fed"],
            vec!["check"],
            vec!["budget"],
            vec!["slice", "--list"],
        ] {
            let code = run_args(argv(&a));
            assert_eq!(
                code,
                ExitCode::SUCCESS,
                "`bbx {}` must exit 0",
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
