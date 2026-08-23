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

    /// Run git in the repo, returning stdout.
    ///
    /// # Errors
    /// The command's stderr when it exits non-zero.
    pub fn git(&self, args: &[&str]) -> Result<String, String> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.root)
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
