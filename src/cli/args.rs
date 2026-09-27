//! Argv RESOLUTION -- which repo, which dir, which depth a command acts on (V5, V15, V17).

use super::*;

/// The repo root, not the invocation directory.
///
/// `sherd` is a shim over `cargo run`, so CWD is wherever you typed it. Using
/// CWD federated from a SUBDIRECTORY silently -- fewer nodes, a truncated
/// chain, and no error to say so. Walk up to the git root instead.
/// The repo root a run is ABOUT, which is not always the one it was launched
/// from.
///
/// Every `[dir]` verb takes a path that may live in ANOTHER repository, and
/// resolving the root from the CWD then silently answers about the wrong
/// tree: `sherd check ../their-project` reported this crate's own nodes and
/// exited 0 (`src/cli:B5`). When the first argument names an existing directory, the
/// root is the one ABOVE IT.
///
/// Any argument may be the directory, not just the first: `plan --triage
/// <dir>` put a FLAG at position one, so a scan of `args[1]` alone found no
/// dir and answered about the CWD -- the same bug one flag to the left, and
/// the fix for it was incomplete until a stranger showed the flag-first form
/// (`B7`).
///
/// A run with no directory argument at all -- `HEAD` for `review`, `--check`
/// alone for `sync` -- leaves the CWD walk alone, and an in-repo path
/// resolves to the same root it always did.
///
/// And the directory it names is made ABSOLUTE before the walk. A relative
/// one was handed over as typed, and `parent()` climbs a relative path to
/// `""`, which holds no `.git`: the walk ran out of parents and returned its
/// own START -- the argument -- as the "root", after which `arg_dir` joined
/// that argument on a second time. `sherd seam code` from `src/` reported
/// `code/code matched no node`, and the doubling is the proof (`src/cli:B9`,
/// which is NOT the root's `B9`).
pub(super) fn root_for(args: &[String]) -> PathBuf {
    root_for_from(args, &cwd())
}

/// As [`root_for`], from an explicit invocation directory (`V6`).
///
/// Split for the reason [`repo_root_from`] is: a test can be HANDED the
/// directory a command was typed in, rather than mutating the process CWD --
/// which is global, and races every other test in the binary.
///
/// The candidate is resolved against `cwd` BEFORE anything reads it, so the
/// scan and the walk agree about what the path means. `join` is LEXICAL and
/// total: an absolute argument is left alone, there is no I/O to fail, and
/// the spelling the caller passed survives. `canonicalize` would also
/// collapse `..`, but it rewrites symlinks -- macOS hands out
/// `/var/folders/...` and resolves it to `/private/var/...` -- so the root
/// would stop comparing equal to the path it was given.
pub(super) fn root_for_from(args: &[String], cwd: &Path) -> PathBuf {
    args.iter()
        .skip(1)
        .map(|a| cwd.join(a))
        .find(|p| p.is_dir())
        .map_or_else(|| repo_root_from(cwd), |p| repo_root_from(&p))
}

/// The directory the command was typed in, or `.` when it cannot be read.
pub(super) fn cwd() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// The root from the CWD, for tests that assert about THIS checkout.
///
/// Dispatch reaches the walk through [`root_for_from`], which is HANDED its
/// directory, so this is a test convenience and is compiled as one -- left in
/// the shipping build it is dead code, which the gate denies.
#[cfg(test)]
pub(super) fn repo_root() -> PathBuf {
    repo_root_from(&cwd())
}

/// The walk, given a starting directory (V6).
///
/// Split from `repo_root` so a test can be HANDED a tree instead of
/// discovering one. `B1` is what the unsplit version cost: the test asserted
/// that `repo_root()` finds a `.git` and a `SPEC.md`, which it always does
/// when the runner sits in this checkout and never does anywhere else -- so
/// the suite passed here and failed in a tarball, and the assertion was
/// about the runner's location rather than about the walk.
pub(super) fn repo_root_from(start: &Path) -> PathBuf {
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
pub(super) fn depth_arg(args: &[String]) -> Result<lens::Depth, String> {
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

pub(super) fn arg_dir(args: &[String], root: &Path) -> PathBuf {
    args.get(1)
        .map_or_else(|| root.to_path_buf(), |d| root.join(d))
}

/// A `[dir]` that matched no node, said in full (V2, V18).
///
/// The path is named absolute, because that is what was looked for. What the
/// old message left out is WHY it was looked for there: `arg_dir` resolves
/// `[dir]` against the repo root, so `seam code` typed in `src/` is a miss
/// even though `src/code` is right there. The contract stays -- an argument
/// that means a different node depending on where the caller stands is the
/// ambiguity `B5` and `B7` were about -- and the miss now teaches the
/// spelling that works instead of ending the conversation.
pub(super) fn no_node(root: &Path, dir: &Path) -> String {
    let mut msg = format!("sherd: {} matched no node", dir.display());
    let rel = dir.strip_prefix(root).unwrap_or(dir);
    let near = fed::spelled(root, rel);
    let Some((first, rest)) = near.split_first() else {
        return msg;
    };
    msg.push_str(
        "\n  [dir] is resolved against the repo ROOT, not the directory you \
         are standing in (V15).",
    );
    if rest.is_empty() {
        msg.push_str(&format!(
            "\n  did you mean `{}`?",
            fed::node_label(root, first)
        ));
        return msg;
    }
    // Several nodes end in that name, so there is no single answer and
    // guessing one would be `route`'s ambiguous case answered as a hit
    // (`.:V20`). All of them are named, and the reader picks.
    let all: Vec<String> =
        near.iter().map(|n| fed::node_label(root, n)).collect();
    msg.push_str(&format!(
        "\n  nodes with that name: {}",
        all.iter()
            .map(|n| format!("`{n}`"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    msg
}

#[cfg(test)]
mod tests {
    use super::super::fixtures::*;
    use super::*;

    /// `B5`: a `[dir]` in ANOTHER repository resolved its root from the CWD,
    /// so `sherd check ../their-project` reported THIS crate's nodes and
    /// exited 0 -- confidently, about the wrong tree. Found by pointing the
    /// tool at a fourth foreign repo, not by any test.
    #[test]
    fn a_dir_in_another_repo_resolves_that_repo_s_root() -> Result<(), String> {
        let theirs = crate::testrepo::TestRepo::new("cli-foreign")?;
        theirs.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\ntheirs\n")?;
        theirs.commit("a repo that is not ours")?;
        let arg = theirs.path().display().to_string();
        assert_eq!(
            root_for(&["check".into(), arg]),
            theirs.path().to_path_buf(),
            "the root is the one ABOVE the given dir"
        );
        // A non-directory first argument leaves the CWD walk alone: `HEAD`
        // for `review`, `--check` for `sync`.
        let here = root_for(&["review".into(), "HEAD".into()]);
        assert_eq!(here, repo_root(), "a rev is not a dir");
        assert_eq!(root_for(&["check".into()]), repo_root(), "no arg at all");
        Ok(())
    }

    /// `B7`: any argument may be the directory, not just the first. `plan
    /// --triage <dir>` puts a FLAG at position one, so scanning `args[1]`
    /// alone found no dir and answered about the CWD -- `B5` one flag to the
    /// left, and its fix was incomplete until a stranger showed this form.
    #[test]
    fn a_flag_before_the_dir_does_not_hide_it() -> Result<(), String> {
        let theirs = crate::testrepo::TestRepo::new("cli-flag-first")?;
        theirs.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\ntheirs\n")?;
        theirs.commit("a repo behind a flag")?;
        let arg = theirs.path().display().to_string();
        let want = theirs.path().to_path_buf();
        for args in [
            vec!["plan".to_string(), "--triage".into(), arg.clone()],
            vec!["sync".to_string(), "--check".into(), arg.clone()],
            vec!["split".to_string(), "--apply".into(), arg.clone()],
            vec!["check".to_string(), arg.clone()],
        ] {
            assert_eq!(root_for(&args), want, "{args:?}");
        }
        // Still no dir anywhere means the CWD walk, unchanged.
        assert_eq!(
            root_for(&["sync".to_string(), "--check".into()]),
            repo_root()
        );
        Ok(())
    }

    /// `src/cli:B9` -- NOT the root's `B9`: a RELATIVE `[dir]` never reached
    /// the walk as a location.
    ///
    /// `parent()` climbs a relative path to `""`, which holds no `.git`, so
    /// the walk ran out of parents and returned its own start -- the argument
    /// itself -- as the root. `arg_dir` then joined that argument on a second
    /// time, and `sherd seam code` from `src/` answered `code/code matched no
    /// node`. The DOUBLING is the proof.
    ///
    /// Every spelling of one directory has to resolve to one root: bare,
    /// `./`-prefixed, from a subdirectory, and absolute. `./x` appeared to
    /// work from the repo root alone, which is why "use `./`" was never a
    /// workaround -- it was the one cwd where the two readings agreed.
    #[test]
    fn every_spelling_of_a_dir_resolves_to_one_root() -> Result<(), String> {
        let r = crate::testrepo::TestRepo::new("cli-relative")?;
        r.write("src/code/SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\na node\n")?;
        r.commit("a node two levels down")?;
        let root = r.path().to_path_buf();
        let abs = root.join("src").join("code").display().to_string();
        for (typed_in, arg) in [
            (root.clone(), "src/code"),
            (root.clone(), "./src/code"),
            (root.join("src"), "code"),
            (root.join("src"), "./code"),
            (root.clone(), abs.as_str()),
        ] {
            assert_eq!(
                root_for_from(&argv(&["seam", arg]), &typed_in),
                root,
                "`sherd seam {arg}` typed in {}",
                typed_in.display()
            );
        }
        // The control: resolving against the CWD must not turn a NON-path
        // into one. A rev is not a dir, so the walk starts where it always
        // did (`B5`).
        assert_eq!(
            root_for_from(&argv(&["review", "HEAD"]), &root),
            root,
            "a rev is not a dir"
        );
        Ok(())
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
        // `.:B9` (the ROOT's, not this node's): a relative argument never
        // compared equal to the absolute paths `fed::discover` returns, so
        // `budget src/tdd` examined nothing.
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

    /// V18, and the residue `B9` left: the contract above is right and the
    /// miss it produces was a dead end. `seam code` typed in `src/` looks for
    /// `<root>/code` -- honestly named and still no help to a reader with
    /// `src/code` in front of them.
    #[test]
    fn a_miss_names_the_spelling_that_would_have_worked() {
        let r = crate::testrepo::TestRepo::new("cli-miss").expect("fixture");
        r.write("src/code/SPEC.md", "# SPEC\n").expect("write");
        let root = r.path();

        let msg = no_node(root, &root.join("code"));
        assert!(msg.contains("matched no node"), "{msg}");
        assert!(msg.contains("resolved against the repo ROOT"), "{msg}");
        assert!(msg.contains("did you mean `src/code`?"), "{msg}");

        // A name no node carries gets no invented suggestion: the honest
        // answer is the miss alone (`V2`).
        let none = no_node(root, &root.join("nowhere"));
        assert!(none.contains("matched no node"), "{none}");
        assert_eq!(none.lines().count(), 1, "{none}");
    }
}
