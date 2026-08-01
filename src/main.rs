//! `bbx` -- arg dispatch only. V41: the binary holds no logic.

use bbx::{fed, lens, spec, tokens};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "\
bbx -- federated SPEC.md for small-context local models

  bbx budget [dir]     token cost of every node, against the working budget
  bbx lens <dir>       the context pack for one node
  bbx fed [dir]        the federation edges declared by a node
  bbx check [dir]      cavespec structural check of every node
  bbx graph [--tree|--table|--dot]  federation DAG, generated from §F
  bbx ask <dir> <q>    ask the endpoint from a node's lens pack
  bbx tdd <dir> <Vn> <task>   red -> judge -> green -> gate -> repair

exit: 0 clean · 1 violation · 2 usage";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = repo_root();
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

fn repo_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
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
    eprintln!("# pack {} · {} nodes", p.cost, p.chain.len());
    match bbx::ollama::generate(&format!("{}\n\n---\n{question}\n", p.text)) {
        Ok(r) => {
            println!("{}", r.text);
            eprintln!("[sent {} tok · gen {} · {}ms]", r.prompt_tokens, r.eval_tokens, r.ms);
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
