//! `plan`, `triage`, `ask` and `tdd` -- the verbs that choose or take the next step.

use super::*;

#[cfg(feature = "ollama")]
pub(super) fn ask(root: &Path, dir: &Path, question: &str) -> ExitCode {
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
pub(super) fn tdd_cmd(
    root: &Path,
    dir: &Path,
    invariant: &str,
    task: &str,
) -> ExitCode {
    match crate::tdd::drive(root, dir, invariant, task, 3) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("sherd: {e}");
            ExitCode::from(1)
        }
    }
}

/// Split `--format <text|json>` out of a verb's arguments, wherever it sits
/// (V4's rule for a mode). `Ok((true, rest))` for json; a missing or unknown
/// value is usage, ⊥ a silent fall back to text a parser then chokes on.
pub(super) fn take_format(
    args: &[String],
) -> Result<(bool, Vec<String>), String> {
    let Some(at) = args.iter().position(|a| a == "--format") else {
        return Ok((false, args.to_vec()));
    };
    let json = match args.get(at.saturating_add(1)).map(String::as_str) {
        Some("json") => true,
        Some("text") => false,
        Some(v) => return Err(format!("unknown --format `{v}` -- text|json")),
        None => return Err("--format needs a value -- text|json".into()),
    };
    let rest = (0usize..)
        .zip(args)
        .filter(|(i, _)| *i != at && *i != at.saturating_add(1))
        .map(|(_, a)| a.clone())
        .collect();
    Ok((json, rest))
}

/// [`take_format`] over a whole argv: the verb stays at `[0]`, so the
/// positional readers (`arg_dir`) see the same shape with or without the flag.
pub(super) fn with_format(
    args: &[String],
) -> Result<(bool, Vec<String>), String> {
    let (json, rest) = take_format(args.get(1..).unwrap_or_default())?;
    Ok((json, args.iter().take(1).cloned().chain(rest).collect()))
}

pub(super) fn plan_cmd(
    root: &Path,
    milestone: Option<&str>,
    json: bool,
) -> ExitCode {
    plan_with(root, milestone, json, &state::State::load())
}

/// `plan` over a store it READS -- believability and the kept/tried record --
/// and never writes (`src/plan:V7`, `B12`).
///
/// It used to clear every `plan` key, record one per step and save, on both
/// output forms. Nothing read those keys -- `apply` re-derives the plan -- so
/// a read-only verb rewrote a store every worktree shares. Taking the store
/// as a value is also what lets a test hand it a file of its own.
pub(super) fn plan_with(
    root: &Path,
    milestone: Option<&str>,
    json: bool,
    st: &state::State,
) -> ExitCode {
    let (p, outside) = plan::plan_in(root, milestone);
    let est: Vec<u64> = p
        .steps
        .iter()
        .map(|t| {
            lens::pack(root, &root.join(&t.node), lens::Depth::Rule)
                .map_or(0, |k| k.cost.tokens)
        })
        .collect();
    if json {
        println!("{}", plan::to_json(st, &p, (milestone, outside), &est));
        return ExitCode::SUCCESS;
    }

    let scope = milestone.map_or(String::new(), |m| format!(" in {m}"));
    println!(
        "HORIZON {} of {} open rows{scope} · {} unmanaged\n",
        p.steps.len(),
        p.total_open,
        p.unmanaged.len()
    );
    // V24: rows the filter set aside are named, never silently dropped.
    if outside > 0 {
        println!(
            "  {outside} open rows sit in nodes that declare no milestones -- not in any\n  milestone, so not in this plan.\n"
        );
    }
    for ((i, t), est) in p.steps.iter().enumerate().zip(&est) {
        let c = plan::Confidence::of(i);
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
    ExitCode::SUCCESS
}

pub(super) fn triage_cmd(root: &Path) -> ExitCode {
    let rows = plan::triage(root);
    let mut moves: std::collections::BTreeMap<&str, Vec<(String, String)>> =
        std::collections::BTreeMap::new();
    let (mut decompose, mut keep) = (Vec::new(), Vec::new());
    for (t, k, p) in &rows {
        let id = format!("{} {}", t.node.display(), t.id);
        match p {
            split::Proposal::Move(n) => {
                moves.entry(n).or_default().push((id, t.text.clone()))
            }
            split::Proposal::Decompose(ns) => {
                decompose.push((id, t.text.clone(), ns.join(" + ")))
            }
            split::Proposal::Keep => keep.push((id, t.text.clone(), k.why())),
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

#[cfg(test)]
#[path = "tests/steps.rs"]
mod tests;
