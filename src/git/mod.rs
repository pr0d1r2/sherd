//! One shape for every git invocation, aimed at one repository.
//!
//! `current_dir` decides where git RUNS. It does not decide which repository
//! git ACTS ON: an inherited `GIT_DIR` overrides directory discovery
//! entirely, and git exports an absolute one to every hook when the checkout
//! is a worktree. So a `sherd` verb invoked from a hook -- or a test running
//! under one -- reached past the root it was handed and operated on whatever
//! repository the hook belonged to (`.:B25`).
//!
//! Nine call sites across `plan`, `review` and `land` each wrote the same
//! three lines, so nine of them were wrong in the same way. This module is
//! the one reading, which is the shape §C asks for.

use std::path::Path;
use std::process::Command;

/// The git variables a hook exports to everything it spawns.
///
/// `GIT_DIR` alone does the damage. The rest steer the same discovery, and
/// no caller here has a reason to honour any of them.
pub const INHERITED: [&str; 8] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_COMMON_DIR",
    "GIT_PREFIX",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_NAMESPACE",
];

/// Strip the inherited environment from a command already built.
fn disinherit(c: &mut Command) {
    for k in INHERITED {
        c.env_remove(k);
    }
}

/// A git command aimed at `root` and at no other repository.
#[must_use]
pub fn at(root: &Path, args: &[&str]) -> Command {
    let mut c = Command::new("git");
    c.args(args).current_dir(root);
    disinherit(&mut c);
    c
}

/// A git command whose own arguments name the repository (`--git-dir`, or a
/// path operand), so it takes no working directory of its own.
#[must_use]
pub fn anywhere(args: &[&str]) -> Command {
    let mut c = Command::new("git");
    c.args(args);
    disinherit(&mut c);
    c
}

#[cfg(test)]
mod tests {
    use super::{INHERITED, anywhere, at};

    #[test]
    fn a_command_aimed_at_a_root_refuses_the_exported_environment() {
        let c = at(std::path::Path::new("."), &["status"]);
        for k in INHERITED {
            assert!(
                c.get_envs().any(|(n, v)| n == k && v.is_none()),
                "`{k}` survives -- an exported one re-aims this command"
            );
        }
    }

    #[test]
    fn a_command_naming_its_own_repo_refuses_it_too() {
        let c = anywhere(&["--git-dir", "x", "branch"]);
        for k in INHERITED {
            assert!(
                c.get_envs().any(|(n, v)| n == k && v.is_none()),
                "`{k}` survives on the cwd-less form"
            );
        }
    }
}
