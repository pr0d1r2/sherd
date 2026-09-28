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
