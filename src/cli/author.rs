//! `init`, `sync` and `slice` -- the verbs that WRITE this repo's specs or generated files.

use super::*;

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
pub(super) fn child_dirs(dir: &Path) -> Result<Vec<String>, String> {
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
pub(super) fn init_dir(root: &Path, args: &[String]) -> PathBuf {
    args.iter()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .map_or_else(|| root.to_path_buf(), |d| root.join(d))
}

/// `sherd sync [dir]` -- regenerate `§N` from the `§F` tables above it.
///
/// Exit 1 IF IT WROTE, which reads backwards until you see it from CI: a
/// generated section that had to change means the committed tree was stale,
/// and a run that silently fixed it would let the staleness ship. Exit 0 is
/// "already correct". `.:V36` makes `§F` authoritative, so this never reads
/// an existing `§N` to decide anything -- it computes what one must say.
pub(super) fn sync_cmd(
    root: &Path,
    dir: Option<&PathBuf>,
    check: bool,
) -> ExitCode {
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
pub(super) fn sync_all(
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
pub(super) fn sync_report(node: &Path, check: bool) {
    let verb = if check { "STALE" } else { "rewritten" };
    println!("{}: §N {verb}", node.display());
}

/// Where `§N` goes in a document that has none yet.
///
/// Beside `§F` where there is one. A LEAF has no `§F` at all -- that is what
/// makes it a leaf -- and `.:V34` still requires its `§N`, so the anchor
/// falls back to `§G`, the one section every spec has.
pub(super) fn nav_anchor(text: &str) -> &'static str {
    if text.contains("## \u{a7}F") {
        "F FEDERATION"
    } else {
        "G GOAL"
    }
}

/// One node's `§N`. `Ok(true)` when the file changed.
pub(super) fn sync_node(
    root: &Path,
    node: &Path,
    check: bool,
) -> Result<bool, String> {
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

/// The refusal, and there is no `--force` to bypass it.
///
/// Clobbering a spec is the one write this tool must never make: `SPEC.md` is
/// the law the rest of the binary enforces, so a scaffold that can overwrite
/// it can erase every invariant a repository has recorded. Deleting the file
/// first is a deliberate act a human takes, with git watching.
pub(super) fn init_refusal(target: &Path) -> Option<ExitCode> {
    target.exists().then(|| {
        eprintln!(
            "sherd: {} exists -- refusing to overwrite a spec. There is no --force: \
             delete it yourself if that is what you mean.",
            target.display()
        );
        ExitCode::from(1)
    })
}

pub(super) fn init_cmd(root: &Path, args: &[String]) -> ExitCode {
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

pub(super) fn init_failed(msg: &str) -> ExitCode {
    eprintln!("{msg}");
    ExitCode::from(1)
}

/// `--stdout` is the PREVIEW, so it must never write. Both paths go through
/// one function, because a preview that diverged from the write would show
/// something other than what lands.
pub(super) fn init_emit(
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
pub(super) fn init_body(
    root: &Path,
    dir: &Path,
) -> Result<(String, usize), String> {
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

pub(super) fn init_write(
    target: &Path,
    body: &str,
    children: usize,
) -> ExitCode {
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

pub(super) fn slice_cmd(root: &Path, mode: &str) -> ExitCode {
    let registry = root.join(".sherd-slices");
    // `V12`: ABSENCE is not a finding. A repo with no registry has no slices
    // to drift, and the raw `No such file or directory (os error 2)` named
    // neither the file nor the fact that it is OPTIONAL -- `validate` has
    // said "none required" for this same condition all along (`B6`).
    if !registry.is_file() {
        println!("slice: no registry (none required)");
        return ExitCode::SUCCESS;
    }
    let decls = match std::fs::read_to_string(&registry)
        .map_err(|e| format!("{}: {e}", registry.display()))
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
