//! `budget`, `lens`, `fed`, `route`, `debt` and `coverage` -- the verbs that MEASURE and report a number.

use super::*;

/// Working budget on the confirmed target tier: gpt-oss:20b at its full
/// 131,072 window, minus measured harness entry cost.
pub(super) const WINDOW: u64 = 131_072;

/// `--record`: bring `.lint-debt`'s numbers current, refusing a raise.
pub(super) fn record_debt(root: &Path, now: crate::debt::Measured) -> ExitCode {
    match crate::debt::record(root, now) {
        Ok(msg) => {
            println!("{msg}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("sherd: {e}");
            ExitCode::from(1)
        }
    }
}

/// `sherd coverage [--check|--record]` -- the floor, as `hk` runs it.
///
/// The MIRROR of `debt`: this ratchet may only RISE. Same reason for
/// existing -- the rule was stated twice in `hk.pkl` and the recording path
/// was a hand edit, which is how a floor got lowered to match a drop today
/// (`src/debt:B7`).
pub(super) fn coverage_cmd(
    root: &Path,
    mode: Option<&str>,
    cargo: &str,
) -> ExitCode {
    let Some(now) = crate::debt::coverage(root, cargo) else {
        eprintln!(
            "sherd: could not read a coverage total -- that is an ERROR, \
             not a floor breach (sherd/tdd:V26)"
        );
        return ExitCode::from(2);
    };
    let Some(was) = crate::debt::recorded_floor(root) else {
        let path = root.join(".coverage");
        eprintln!("sherd: {}: no `lines` row to read", path.display());
        return ExitCode::from(2);
    };
    if mode == Some("--record") {
        return match crate::debt::record_coverage(root, now) {
            Ok(msg) => {
                println!("{msg}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("sherd: {e}");
                ExitCode::from(1)
            }
        };
    }
    let (ok, report) = crate::debt::coverage_verdict(now, was);
    if ok {
        println!("{report}");
        return ExitCode::SUCCESS;
    }
    eprintln!("{report}");
    ExitCode::from(1)
}

/// `sherd debt [--check|--record]` -- the ratchet, as `hk` runs it.
///
/// The gate CALLS this instead of re-deriving the formula in awk. It was
/// stated three times before -- twice in `hk.pkl`, once in `src/land` -- and
/// the Rust copy diverged on all three of its inputs at once
/// (`src/debt:B6`). One caller cannot disagree with itself.
pub(super) fn debt_cmd(
    root: &Path,
    mode: Option<&str>,
    cargo: &str,
) -> ExitCode {
    let Some(now) = crate::debt::measure(root, cargo) else {
        eprintln!(
            "sherd: clippy did not COMPILE, so its count is not a \
             measurement -- a target that fails to build emits no warnings \
             at all (sherd/debt:V3)."
        );
        return ExitCode::from(2);
    };
    let Some(was) = crate::debt::recorded_ceilings(root) else {
        // ABSENT and MALFORMED are different mistakes and only one of them
        // is the reader's fault (`V12`, `B6` again one verb over).
        let registry = root.join(".lint-debt");
        if registry.is_file() {
            eprintln!(
                "sherd: {}: no `density` and `shape` rows to read",
                registry.display()
            );
        } else {
            eprintln!(
                "sherd: {}: no ratchet here. `sherd debt` needs a `.lint-debt` \
                 carrying `density` and `shape`.",
                registry.display()
            );
        }
        return ExitCode::from(2);
    };
    if mode == Some("--record") {
        return record_debt(root, now);
    }
    let (ok, report) = crate::debt::verdict(now, was);
    if ok {
        println!("{report}");
    } else {
        eprintln!("{report}");
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

pub(super) fn budget(root: &Path, dir: PathBuf) -> ExitCode {
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
        eprintln!("{}", no_node(root, &dir));
        return ExitCode::from(2);
    }
    // A COLD START is not a breach. With no `.context-limits` at all the
    // default is a suggestion nobody wrote, and failing a stranger against
    // it is a claim about a rule that does not exist (`B8`). An unlisted
    // PATH inside an existing file still takes the default -- `.:V6`.
    if tokens::Ceilings::load(root).is_ok_and(|c| c.is_cold()) {
        println!(
            "  no .context-limits: ceilings are the {} tok default, and \
             over is advisory until you set them",
            tokens::DEFAULT_NODE
        );
        return ExitCode::SUCCESS;
    }
    // T10/V104: the number exists to be COMPARED. Printing it and exiting 0
    // is what let the chains drift over unseen (B7).
    if over > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

pub(super) fn lens_cmd(
    root: &Path,
    dir: &Path,
    depth: lens::Depth,
) -> ExitCode {
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

pub(super) fn fed_cmd(dir: &Path) -> ExitCode {
    let path = dir.join("SPEC.md");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("sherd: no SPEC.md at {}", dir.display());
        return ExitCode::from(2);
    };
    let edges = fed::edges(&text);
    for e in &edges {
        println!("{:<16} {:<40} not: {}", e.dir, e.owns, e.not_owns);
    }
    // `.:V48`: state what was EXAMINED. An unfederated spec printed NOTHING
    // and exited 0, which reads as "no problems" rather than "no §F table"
    // -- the shape `src/fed:B6` records, met on a stranger (`B6` here).
    if edges.is_empty() {
        println!(
            "{}: no §F table -- this node federates nothing. \
             `sherd split` proposes where the boundaries are.",
            dir.display()
        );
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
pub(super) fn route_cmd(root: &Path, query: &str) -> ExitCode {
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
pub(super) fn route_miss(root: &Path) -> ExitCode {
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
pub(super) fn route_ambiguous(root: &Path, nodes: &[PathBuf]) -> ExitCode {
    println!("ambiguous -- the query spans {} nodes:", nodes.len());
    for n in nodes {
        println!("  {}", n.strip_prefix(root).unwrap_or(n).display());
    }
    ExitCode::from(3)
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::*;

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

    #[test]
    fn route_reports_a_hit_a_miss_and_an_ambiguity_by_exit_code() {
        let repo = routing_fixture("cli-route");
        let root = repo.path();
        assert_eq!(route_cmd(root, "sprockets"), ExitCode::SUCCESS);
        assert_eq!(route_cmd(root, "wombat"), ExitCode::from(2));
        assert_eq!(route_cmd(root, "widgets gizmos"), ExitCode::from(3));
    }
}
