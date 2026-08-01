//! `bbx` -- arg dispatch only. V41: the binary holds no logic.

use bbx::{fed, lens, plan, spec, state, tokens};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
bbx -- federated SPEC.md for small-context local models

  bbx budget [dir]     token cost of every node, against the working budget
  bbx lens <dir>       the context pack for one node
  bbx fed [dir]        the federation edges declared by a node
  bbx check [dir]      cavespec structural check of every node
  bbx graph [--tree|--table|--dot]  federation DAG, generated from §F
  bbx plan             next 3 steps, with what would invalidate each
  bbx apply            execute step 1 only, commit it, then stop
  bbx ask <dir> <q>    ask the endpoint from a node's lens pack
  bbx tdd <dir> <Vn> <task>   red -> judge -> green -> gate -> repair

  -v, --verbose        dump every prompt and stream every reply

exit: 0 clean · 1 violation · 2 usage";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // -v / --verbose is positional-agnostic: it is a mode, not an argument.
    #[cfg(feature = "ollama")]
    if let Some(i) = args.iter().position(|a| a == "-v" || a == "--verbose") {
        args.remove(i);
        bbx::ollama::set_verbose(true);
    }
    let root = repo_root();
    #[cfg(feature = "ollama")]
    bbx::ollama::load_pace();
    match args.first().map(String::as_str) {
        Some("budget") => budget(&root, arg_dir(&args, &root)),
        Some("lens") => match args.get(1) {
            Some(d) => lens_cmd(&root, &PathBuf::from(d)),
            None => usage("lens needs a dir"),
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
        Some("plan") => plan_cmd(&root),
        #[cfg(feature = "ollama")]
        Some("apply") => match plan::apply(&root, 3) {
            Ok(sha) => {
                eprintln!("\napplied as {sha}. REPLAN before the next step -- \
                           this commit changed the specs that plan it.");
                ExitCode::SUCCESS
            }
            Err(e) => { eprintln!("bbx: {e}"); ExitCode::from(1) }
        },
        #[cfg(feature = "ollama")]
        Some("ask") => match (args.get(1), args.get(2)) {
            (Some(d), Some(q)) => ask(&root, &PathBuf::from(d), q),
            _ => usage("ask needs <dir> and a question"),
        },
        #[cfg(feature = "ollama")]
        Some("oneshot") => match (args.get(1), args.get(2), args.get(3)) {
            (Some(d), Some(v), Some(t)) => match bbx::tdd::oneshot(&root, &PathBuf::from(d), v, t) {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => { eprintln!("bbx: {e}"); ExitCode::from(1) }
            },
            _ => usage("oneshot needs <dir> <invariant> <task>"),
        },
        #[cfg(feature = "ollama")]
        Some("tdd") => match (args.get(1), args.get(2), args.get(3)) {
            (Some(d), Some(v), Some(task)) => tdd_cmd(&root, &PathBuf::from(d), v, task),
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

fn arg_dir(args: &[String], root: &Path) -> PathBuf {
    args.get(1).map_or_else(|| root.to_path_buf(), PathBuf::from)
}

fn usage(msg: &str) -> ExitCode {
    eprintln!("bbx: {msg}\n\n{USAGE}");
    ExitCode::from(2)
}

/// Working budget on the confirmed target tier: gpt-oss:20b at its full
/// 131,072 window, minus measured harness entry cost.
const WINDOW: u64 = 131_072;

fn budget(root: &Path, _dir: PathBuf) -> ExitCode {
    let work = tokens::working(WINDOW);
    println!("window {WINDOW} · entry {} · working {work}\n", tokens::ENTRY_COST);
    let nodes = fed::discover(root);
    let mut total = 0;
    for node in &nodes {
        match lens::pack(root, node, lens::Depth::Rule) {
            Ok(p) => {
                total += p.cost.tokens;
                let rel = node.strip_prefix(root).unwrap_or(node);
                let name = if rel.as_os_str().is_empty() { Path::new(".") } else { rel };
                println!("  {:<24} chain {:>6} tok  ({} nodes)", name.display(), p.cost.tokens, p.chain.len());
            }
            Err(e) => {
                eprintln!("bbx: {}: {e}", node.display());
                return ExitCode::from(1);
            }
        }
    }
    // V48: say what was examined, not only what failed.
    println!("\n  {} nodes examined · {total} tok if all chains loaded", nodes.len());
    ExitCode::SUCCESS
}

fn lens_cmd(root: &Path, dir: &Path) -> ExitCode {
    match lens::pack(root, dir, lens::Depth::Rule) {
        Ok(p) => {
            eprintln!("# chain: {} nodes · {}", p.chain.len(), p.cost);
            for c in &p.children {
                eprintln!("#   -> {:<16} {}  [not: {}]", c.dir, c.owns, c.not_owns);
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
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        for v in spec::check(&text) {
            println!("{}:{}: cavespec/{}: {}", path.display(), v.line, v.rule, v.msg);
            bad += 1;
        }
    }
    println!("\n  {} nodes examined · {bad} violations", nodes.len());
    if bad > 0 { ExitCode::from(1) } else { ExitCode::SUCCESS }
}

#[cfg(feature = "ollama")]
fn ask(root: &Path, dir: &Path, question: &str) -> ExitCode {
    let Ok(p) = lens::pack(root, dir, lens::Depth::Rule) else {
        eprintln!("bbx: {}: no pack", dir.display());
        return ExitCode::from(2);
    };
    use std::io::Write;
    let prompt = format!("{}\n\n---\n{question}\n", p.text);
    let eta = bbx::ollama::predict(p.cost.tokens);
    eprintln!("# pack {} · {} nodes · eta {:.0}s cold / {:.0}s if cached",
              p.cost, p.chain.len(), eta.total_s(), eta.cached_s());
    eprint!("# ");
    let _ = std::io::stderr().flush();
    let mut n = 0usize;
    match bbx::ollama::generate_with(&prompt, eta, &mut |c| {
        if bbx::ollama::verbose() { eprint!("{c}") } else {
            n += 1;
            if n % 25 == 0 { eprint!(".") }
        }
        let _ = std::io::stderr().flush();
    }) {
        Ok(r) => {
            eprintln!();
            println!("{}", r.text);
            let actual = r.ms as f64 / 1000.0;
            eprintln!("[{} sent · {} gen · {actual:.1}s (eta {:.0}s, {:+.0}%){}]",
                      r.prompt_tokens, r.eval_tokens,
                      if bbx::ollama::last_cached() { eta.cached_s() } else { eta.total_s() },
                      (actual - if bbx::ollama::last_cached() { eta.cached_s() } else { eta.total_s() })
                          / if bbx::ollama::last_cached() { eta.cached_s() } else { eta.total_s() } * 100.0,
                      if bbx::ollama::last_cached() { " prefix CACHED" } else { "" });
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
    match bbx::tdd::drive(root, dir, invariant, task, 3) {
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

    println!("HORIZON {} of {} open rows · {} unmanaged\n",
             p.steps.len(), p.total_open, p.unmanaged.len());
    for (i, t) in p.steps.iter().enumerate() {
        let c = plan::Confidence::of(i);
        let est = lens::pack(root, &root.join(&t.node), lens::Depth::Rule)
            .map_or(0, |k| k.cost.tokens);
        println!("{}. {} {:<11} {} {}", i + 1, c.label(), t.node.display(), t.id, t.text);
        println!("      ~{est} tok context · invalidated by: {}\n", c.invalidated_by());
        st.set("plan", &(i + 1).to_string(),
               format!("{} {} {}", t.node.display(), t.id, c.label().trim()));
    }
    if p.steps.is_empty() {
        println!("  nothing actionable.\n");
    }
    // V3: say what is NOT managed. Silence would read as coverage.
    println!("UNMANAGED ({}):", p.unmanaged.len());
    let mut by: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for (_, k) in &p.unmanaged {
        *by.entry(k.why()).or_default() += 1;
    }
    for (why, n) in by {
        println!("  {n:3}  {why}");
    }
    println!("\nREPLAN after each apply -- applying a task edits the spec that plans the next.");
    st.save();
    ExitCode::SUCCESS
}
