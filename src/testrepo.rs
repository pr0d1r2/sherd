//! A scratch git repo, for tests only.
//!
//! `plan::preflight`/`apply`, `review::node`/`added_in_commit`/`commit` and
//! `land::evidence`/`current_branch`/`remote` all shell out to git, so none
//! of them could be exercised at all -- which is most of what `.:R50` measured
//! as uncovered in those three nodes. The gap was never missing effort; it was
//! a missing fixture, the same shape as `src/tdd:T17`'s transport and the
//! `run_args` seam.
//!
//! Compiled only under `cfg(test)`, so it ships in no binary.

/// Run `body` only when the suite is running against THIS repository.
///
/// Some tests assert facts about our own tree -- the ceilings in
/// `.context-limits`, the rows in `.sherd-slices` -- and those files are
/// `exclude`d from the published crate on purpose. On registry source they
/// are absent, so the assertion is unanswerable rather than false, and a
/// test that cannot be answered must not fail: it must not run.
///
/// Set in the dev shell and by the gate, so it is on wherever the repo is.
/// It is an env var and not a `cfg` because the same compiled test binary
/// runs in both places (`itok` reached the same shape for the same reason).
///
/// It TAKES the body rather than returning a bool, and that is a coverage
/// decision: `if !dogfood() { return; }` puts a `return` in every caller
/// that never executes while the variable is set -- six tests, six lines
/// the gate can never cover, and `sherd/debt:V9` is a floor that may only
/// rise. Here the skip lives on one line, in one place, and that line runs.
pub fn dogfood(body: impl FnOnce()) {
    if std::env::var_os("SHERD_DOGFOOD").is_some() {
        body();
    }
}

use std::path::{Path, PathBuf};
use std::process::Command;

/// A temp git repo that deletes itself.
pub struct TestRepo {
    /// Repo root.
    pub root: PathBuf,
}

impl Drop for TestRepo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

impl TestRepo {
    /// Create an initialised repo with one commit on `main`.
    ///
    /// # Errors
    /// Any git or filesystem failure, verbatim. Setup that cannot run is an
    /// ERROR, never a silent empty repo (`src/tdd:V26`).
    pub fn new(tag: &str) -> Result<Self, String> {
        // Unique per INSTANCE, not per process. Keying on the pid alone
        // gave every test in the binary the same directory, so two tests
        // sharing a tag raced and one's `Drop` deleted the other's repo --
        // green alone, red in the suite (`B4`).
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let n = N.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir()
            .join(format!("sherd-repo-{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).map_err(|e| format!("mkdir: {e}"))?;
        let r = Self { root };
        r.git(&["init", "-q", "-b", "main"])?;
        r.git(&["config", "user.email", "t@example.invalid"])?;
        r.git(&["config", "user.name", "test"])?;
        r.write("SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\nfixture\n")?;
        r.commit("root")?;
        Ok(r)
    }

    /// A git command aimed at THIS repo and no other.
    ///
    /// Delegates to `crate::git`, which owns the rule. A fixture carrying its
    /// own copy would be a second reading of it (`.:B25`).
    fn command(&self, args: &[&str]) -> Command {
        crate::git::at(&self.root, args)
    }

    /// Run git in the repo, returning stdout.
    ///
    /// # Errors
    /// The command's stderr when it exits non-zero.
    pub fn git(&self, args: &[&str]) -> Result<String, String> {
        let out = self
            .command(args)
            .output()
            .map_err(|e| format!("git {args:?}: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
        }
    }

    /// Write a file, creating parents.
    ///
    /// # Errors
    /// The filesystem failure, verbatim.
    pub fn write(&self, rel: &str, body: &str) -> Result<(), String> {
        let p = self.root.join(rel);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| format!("mkdir: {e}"))?;
        }
        std::fs::write(&p, body).map_err(|e| format!("write {rel}: {e}"))
    }

    /// Stage everything and commit.
    ///
    /// # Errors
    /// Propagates the git failure.
    pub fn commit(&self, msg: &str) -> Result<(), String> {
        self.git(&["add", "-A"])?;
        self.git(&["commit", "-q", "--no-verify", "-m", msg])
            .map(|_| ())
    }

    /// The repo root as a `&Path`.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    use super::TestRepo;

    /// The regression this file exists to prevent from recurring.
    ///
    /// git exports an ABSOLUTE `GIT_DIR` to hooks when the checkout is a
    /// worktree, and every child the hook spawns inherits it. A fixture that
    /// honoured it ran `init`, `add -A` and `commit` against the developer's
    /// own repository.
    #[test]
    fn the_fixture_refuses_the_git_environment_a_hook_exports() {
        let Ok(r) = TestRepo::new("env") else {
            unreachable!("the fixture builds")
        };
        let c = r.command(&["status"]);
        for k in crate::git::INHERITED {
            let removed = c
                .get_envs()
                .any(|(name, value)| name == k && value.is_none());
            assert!(
                removed,
                "`{k}` is still inherited -- an exported one aims this \
                 fixture's git at the repository the suite is running in"
            );
        }
    }

    /// The fixture's own repository is the one its commits reach.
    #[test]
    fn the_fixture_commits_land_in_its_own_repo() {
        let Ok(r) = TestRepo::new("own") else {
            unreachable!("the fixture builds")
        };
        let Ok(dir) = r.git(&["rev-parse", "--absolute-git-dir"]) else {
            unreachable!("the fixture is a repository")
        };
        // macOS hands out `/var/folders/...` and git reports the resolved
        // `/private/var/folders/...`, so both sides are canonicalised before
        // being compared.
        let Ok(root) = std::fs::canonicalize(&r.root) else {
            unreachable!("the fixture root exists")
        };
        let Ok(dir) = std::fs::canonicalize(&dir) else {
            unreachable!("the git dir exists")
        };
        assert!(
            dir.starts_with(&root),
            "the fixture's git dir is {}, outside its own root {}",
            dir.display(),
            root.display()
        );
    }
}
