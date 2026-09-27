//! Arg dispatch and exit codes. The binary is a shim over `run` (`src/cli:V14`).
//!
//! A node, not a loose `main.rs`, because exit codes and usage are real
//! contracts and every §T row about them was unreachable while this file had
//! no `SPEC.md` to hold them.

use crate::{code, fed, lens, plan, slice, spec, state, tokens, wave};
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
mod fixtures;

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::*;

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
            vec!["seam"],
            vec!["wave"],
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

    /// `.:V50`: the two halves are counted SEPARATELY, never as one ceiling
    /// over both. A tiny implementation with a large suite is a different
    /// thing from the reverse, and one number over the pair cannot tell them
    /// apart -- so this fixture is exactly that shape.
    #[test]
    fn the_code_and_test_halves_have_their_own_ceilings() -> Result<(), String>
    {
        let r = crate::testrepo::TestRepo::new("cli-v50")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nceilings\n")?;
        let big = "    assert_eq!(one_plus_one(), 2, \"a wordy message\");\n"
            .repeat(400);
        r.write(
            "src/small.rs",
            &format!(
                "pub fn one_plus_one() -> u32 {{ 2 }}\n\
                 #[cfg(test)]\nmod t {{\n use super::*;\n #[test]\n \
                 fn a() {{\n{big}}}\n}}\n"
            ),
        )?;
        r.commit("a small impl and a large suite")?;
        let found = file_ceilings(r.path());
        assert_eq!(found.len(), 1, "one half over, not both: {found:?}");
        let Some(one) = found.first() else {
            unreachable!("just asserted a length of one")
        };
        assert!(
            one.contains("tests"),
            "the TEST half is the one over: {one}"
        );
        assert!(
            one.contains("(judgment)"),
            "kind is judgment -- the call is the reader's: {one}"
        );
        Ok(())
    }

    /// A file inside both ceilings reports nothing at all -- the check must
    /// not fire on every file merely for existing.
    #[test]
    fn a_file_within_both_ceilings_is_silent() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-v50-quiet")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nquiet\n")?;
        r.write("src/tiny.rs", "pub fn f() -> u8 { 1 }\n")?;
        r.commit("one small file")?;
        assert!(file_ceilings(r.path()).is_empty());
        Ok(())
    }

    /// `B6`: every verb must be run against a repo that has NONE of the thing
    /// it looks for. Our own tree has all of them, so ABSENCE is the case only
    /// a stranger tests -- and both of these were found on one.
    #[test]
    fn absence_is_reported_and_is_not_a_failure() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-absence")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nnothing here\n")?;
        r.commit("a repo with no §F and no slice registry")?;

        // No `.sherd-slices` is "none required", never a bare OS error.
        assert_eq!(slice_cmd(r.path(), "--list"), ExitCode::SUCCESS);
        assert_eq!(slice_cmd(r.path(), "--check"), ExitCode::SUCCESS);

        // No `§F` table says so rather than printing nothing at all
        // (`.:V48`).
        assert_eq!(fed_cmd(r.path()), ExitCode::SUCCESS);

        // And a dir with no spec at all is still a usage error, not silence.
        assert_eq!(fed_cmd(&r.path().join("nowhere")), ExitCode::from(2));
        Ok(())
    }

    /// `B8`: a COLD START is not a breach. With no `.context-limits` at all
    /// the default is a suggestion nobody wrote, and failing a stranger
    /// against it is a claim about a rule that does not exist.
    ///
    /// An unlisted PATH inside an EXISTING file is a different thing and
    /// still takes the default -- `.:V6` requires that, or a row silently
    /// skipped gates nothing.
    #[test]
    fn a_default_ceiling_nobody_set_is_advisory() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-cold")?;
        let big = "V1: a rule long enough to blow past two thousand tokens. "
            .repeat(400);
        r.write(
            "SPEC.md",
            &format!("# SPEC\n\n## \u{a7}V INVARIANTS\n\n{big}\n"),
        )?;
        r.commit("a chain over the default, with no ceilings file")?;

        // Cold: reported, not failed.
        assert_eq!(budget(r.path(), r.path().to_path_buf()), ExitCode::SUCCESS);

        // Warm: the SAME tree with a ceilings file fails, because now the
        // number is one somebody wrote.
        r.write(".context-limits", "SPEC.md 100\n")?;
        assert_eq!(budget(r.path(), r.path().to_path_buf()), ExitCode::from(1));

        // And a path unlisted in an existing file still takes the default.
        r.write(".context-limits", "src/nowhere 999999\n")?;
        assert_eq!(budget(r.path(), r.path().to_path_buf()), ExitCode::from(1));

        // `validate` gives ONE verdict over the same rule: cold counts zero,
        // warm counts the breach.
        let nodes = crate::fed::discover(r.path());
        assert_eq!(validate_ceilings(r.path(), &nodes), 1, "warm counts it");
        std::fs::remove_file(r.path().join(".context-limits"))
            .map_err(|e| e.to_string())?;
        assert_eq!(validate_ceilings(r.path(), &nodes), 0, "cold does not");
        Ok(())
    }

    /// `sherd debt` end to end, over a scripted toolchain: the toolchain is a
    /// PARAMETER, which is what makes the verb testable at all -- `tdd::Run`
    /// carries `cargo` for the same reason. A verb the gate runs on every
    /// commit that no test can reach is a verb nobody has checked.
    #[test]
    fn the_debt_verb_checks_records_and_refuses() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-debt")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\ndebt\n")?;
        r.write("src/a.rs", &"fn f() {}\n".repeat(100))?;
        r.write(
            ".lint-debt",
            "# the reason\ndensity 20.0\nshape 25.0\ncount 0\nexcess 0\nloc 0\n",
        )?;
        r.commit("a tree with a ratchet")?;
        let fake = r.path().join("fake-cargo");
        std::fs::write(
            &fake,
            "#!/bin/sh\necho 'src/a.rs:1:1: warning: too many lines (35/15)' >&2\nexit 0\n",
        )
        .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &fake,
                std::fs::Permissions::from_mode(0o755),
            )
            .map_err(|e| e.to_string())?;
        }
        let fake = fake.display().to_string();

        // 1 warning over 100 lines is 10.0 per KLoC, and 20 excess lines is
        // 20.0% -- both under the recorded ceilings.
        assert_eq!(
            debt_cmd(r.path(), Some("--check"), &fake),
            ExitCode::SUCCESS
        );
        // `--record` brings it current and keeps the reason.
        assert_eq!(
            debt_cmd(r.path(), Some("--record"), &fake),
            ExitCode::SUCCESS
        );
        let after = std::fs::read_to_string(r.path().join(".lint-debt"))
            .unwrap_or_default();
        assert!(after.contains("density 10.0"), "{after}");
        assert!(after.contains("shape 20.0"), "{after}");
        assert!(after.contains("# the reason"), "the reason survives");
        // Recorded at 10.0, so recording again is a no-op and still passes.
        assert_eq!(
            debt_cmd(r.path(), Some("--check"), &fake),
            ExitCode::SUCCESS
        );
        Ok(())
    }

    /// A tree with no ratchet is a USAGE error, never a clean bill: absence
    /// must not read as "zero allowed" (`sherd/debt:V3`).
    #[test]
    fn a_tree_with_no_ratchet_is_a_usage_error() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-debt-none")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nnone\n")?;
        r.write("src/a.rs", "fn f() {}\n")?;
        r.commit("no ratchet here")?;
        assert_eq!(
            debt_cmd(r.path(), Some("--check"), "true"),
            ExitCode::from(2)
        );
        // A file that EXISTS but carries no ceilings is a different
        // mistake, and only one of the two is the reader's fault (`B7`).
        r.write(".lint-debt", "# a comment and nothing else\n")?;
        assert_eq!(
            debt_cmd(r.path(), Some("--check"), "true"),
            ExitCode::from(2)
        );
        Ok(())
    }

    /// `sherd coverage` end to end over a scripted toolchain, the same way
    /// `debt` is tested: the toolchain is a PARAMETER, so the verb the gate
    /// runs on every push is reachable from a test.
    #[test]
    fn the_coverage_verb_checks_records_and_refuses() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-cov")?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\ncov\n")?;
        r.write(".coverage", "# the reason\nlines 90.00\n")?;
        r.commit("a tree with a floor")?;
        let fake = r.path().join("fake-cargo");
        std::fs::write(
            &fake,
            "#!/bin/sh\necho 'TOTAL 1 2 3.00% 4 5 6.00% 7 8 92.50% 0 0 -'\nexit 0\n",
        )
        .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                &fake,
                std::fs::Permissions::from_mode(0o755),
            )
            .map_err(|e| e.to_string())?;
        }
        let fake = fake.display().to_string();

        // 92.50 is above the recorded 90.00.
        assert_eq!(
            coverage_cmd(r.path(), Some("--check"), &fake),
            ExitCode::SUCCESS
        );
        assert_eq!(
            coverage_cmd(r.path(), Some("--record"), &fake),
            ExitCode::SUCCESS
        );
        let after = std::fs::read_to_string(r.path().join(".coverage"))
            .unwrap_or_default();
        assert!(after.contains("lines 92.50"), "{after}");
        assert!(after.contains("# the reason"), "the reason survives");

        // A toolchain that cannot run is an ERROR, not a breach (V26).
        assert_eq!(
            coverage_cmd(r.path(), Some("--check"), "definitely-not-a-cargo"),
            ExitCode::from(2)
        );
        // And a tree with no floor is a usage error, not a clean bill.
        let _ = std::fs::remove_file(r.path().join(".coverage"));
        assert_eq!(
            coverage_cmd(r.path(), Some("--check"), &fake),
            ExitCode::from(2)
        );
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

    #[test]
    fn route_reports_a_hit_a_miss_and_an_ambiguity_by_exit_code() {
        let repo = routing_fixture("cli-route");
        let root = repo.path();
        assert_eq!(route_cmd(root, "sprockets"), ExitCode::SUCCESS);
        assert_eq!(route_cmd(root, "wombat"), ExitCode::from(2));
        assert_eq!(route_cmd(root, "widgets gizmos"), ExitCode::from(3));
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

    /// The read-only verbs, run against THIS repo.
    ///
    /// A fixture would be a second repo to keep honest; the gate already
    /// requires these green here, so running them on the real tree asserts
    /// the same thing the gate does and covers the dispatch that reaches
    /// them. `.:V27` -- this repo must be a valid federation -- is exactly
    /// the claim being exercised.
    const VERBS: [&[&str]; 10] = [
        &["graph"],
        &["graph", "--dot"],
        &["graph", "--table"],
        &["graph", "--tree"],
        &["fed"],
        &["check"],
        &["budget"],
        &["slice", "--list"],
        &["seam"],
        &["wave"],
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

    /// V16. Both spellings answer, and they answer with the version this
    /// binary was BUILT from -- `B10` is the state where neither did, and a
    /// CI gate recording tool versions had to parse the usage banner.
    #[test]
    fn both_version_spellings_exit_clean_and_name_the_crate_version() {
        assert_eq!(run_args(argv(&["--version"])), ExitCode::SUCCESS);
        assert_eq!(run_args(argv(&["-V"])), ExitCode::SUCCESS);

        // Read from the manifest rather than restated here: a literal in
        // this assertion is the second spelling `VERSION` exists to avoid,
        // and it would need editing at every release.
        let manifest =
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
        let declared = manifest
            .lines()
            .find_map(|l| l.strip_prefix("version = \""))
            .and_then(|v| v.split_once('"').map(|(v, _)| v));
        assert_eq!(declared, Some(VERSION), "{VERSION}");

        let parts: Vec<&str> = VERSION.split('.').collect();
        assert_eq!(parts.len(), 3, "semver, three components: {VERSION}");
    }

    #[test]
    fn the_usage_text_names_every_exit_code_it_returns() {
        // The three codes the tests above assert are the three §I documents.
        assert!(USAGE.contains("0 clean"), "{USAGE}");
        assert!(USAGE.contains("1 violation"), "{USAGE}");
        assert!(USAGE.contains("2 usage"), "{USAGE}");
    }
}
