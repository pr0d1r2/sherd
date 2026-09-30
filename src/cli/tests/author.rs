use super::super::fixtures::*;
use super::*;

/// `init` REFUSES an existing spec, and the refusal is the feature. A
/// scaffold that can overwrite `SPEC.md` can erase every invariant a
/// repository has recorded, so there is no `--force` to test.
#[test]
fn init_refuses_to_overwrite_an_existing_spec() {
    let Ok(repo) = crate::testrepo::TestRepo::new("cli-init-refuse") else {
        unreachable!("a fixture repository is buildable")
    };
    // `TestRepo::new` writes a SPEC.md, so the root already has one.
    assert_eq!(init_cmd(repo.path(), &argv(&["init"])), ExitCode::from(1));
}

/// A directory with children gets a row per child; one without gets no
/// `§F` table at all, because an empty table is a claim of no children
/// rather than an absence of information.
/// A repository with a `node/deep` directory: enough for one child row.
fn init_fixture(tag: &str) -> (crate::testrepo::TestRepo, PathBuf) {
    let Ok(repo) = crate::testrepo::TestRepo::new(tag) else {
        unreachable!("a fixture repository is buildable")
    };
    let node = repo.path().join("node");
    let Ok(()) = std::fs::create_dir_all(node.join("deep")) else {
        unreachable!("a nested dir is creatable")
    };
    (repo, node)
}

/// Idempotence here is REFUSAL, not a silent rewrite: the second run
/// finds the file the first one wrote and declines to touch it.
#[test]
fn init_run_twice_refuses_the_second_time() {
    let (repo, _node) = init_fixture("cli-init-twice");
    assert_eq!(
        init_cmd(repo.path(), &argv(&["init", "node"])),
        ExitCode::SUCCESS
    );
    assert_eq!(
        init_cmd(repo.path(), &argv(&["init", "node"])),
        ExitCode::from(1)
    );
}

#[test]
fn init_writes_a_scaffold_with_a_row_per_child() {
    let (repo, node) = init_fixture("cli-init-write");
    assert_eq!(
        init_cmd(repo.path(), &argv(&["init", "node"])),
        ExitCode::SUCCESS
    );
    let Ok(body) = std::fs::read_to_string(node.join("SPEC.md")) else {
        unreachable!("init wrote a spec")
    };
    assert!(body.contains("deep|WHAT IT OWNS"), "the child row: {body}");
    assert!(
        crate::spec::check(&body).is_empty(),
        "our checker accepts it"
    );
}

/// `--stdout` is the preview, and previewing must never write.
#[test]
fn init_stdout_writes_nothing() {
    let Ok(repo) = crate::testrepo::TestRepo::new("cli-init-stdout") else {
        unreachable!("a fixture repository is buildable")
    };
    let node = repo.path().join("preview");
    let Ok(()) = std::fs::create_dir_all(&node) else {
        unreachable!("a dir is creatable")
    };
    assert_eq!(
        init_cmd(repo.path(), &argv(&["init", "preview", "--stdout"])),
        ExitCode::SUCCESS
    );
    assert!(!node.join("SPEC.md").exists(), "preview wrote a file");
}

/// The failure path: a directory that cannot be read. `init` reports the
/// cause and exits 1 rather than scaffolding an empty `§F` table, which
/// would claim "no children" about a directory it never saw.
#[test]
fn init_reports_a_directory_it_cannot_read() {
    let missing = std::env::temp_dir().join("sherd-no-such-dir-init");
    let _ = std::fs::remove_dir_all(&missing);
    let Err(msg) = init_body(Path::new("/"), &missing) else {
        unreachable!("an unreadable directory is an error")
    };
    assert!(msg.contains("sherd-no-such-dir-init"), "names it: {msg}");
    assert_eq!(init_failed(&msg), ExitCode::from(1));
}

#[test]
fn init_on_a_path_that_is_not_a_directory_is_usage() {
    let Ok(repo) = crate::testrepo::TestRepo::new("cli-init-nodir") else {
        unreachable!("a fixture repository is buildable")
    };
    assert_eq!(
        init_cmd(repo.path(), &argv(&["init", "no-such-dir"])),
        ExitCode::from(2)
    );
}

/// `sync` is idempotent, `--check` never writes, and a stale `§N` is
/// reported by both. The exit code inverts on purpose: writing means the
/// committed tree WAS stale, which CI has to hear about.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the assertions are a SEQUENCE -- stale, then check writes \
                  nothing, then the fix writes, then it is a no-op -- and \
                  splitting them into separate tests loses the ordering, \
                  which is the property under test"
)]
fn sync_writes_once_then_reports_clean() {
    let repo = routing_fixture("cli-sync");
    let root = repo.path();
    write_spec(
        root,
        "alpha",
        "widgets\n\n## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\ndeep|the deep bit|the rest|-",
    );
    let Ok(()) = std::fs::create_dir_all(root.join("alpha").join("deep"))
    else {
        unreachable!("a nested dir is creatable")
    };

    assert_eq!(sync_cmd(root, None, true), ExitCode::from(1), "stale");
    let before = std::fs::read_to_string(root.join("SPEC.md")).ok();
    assert_eq!(
        std::fs::read_to_string(root.join("SPEC.md")).ok(),
        before,
        "--check wrote to the tree"
    );

    assert_eq!(sync_cmd(root, None, false), ExitCode::from(1), "wrote");
    assert_eq!(sync_cmd(root, None, false), ExitCode::SUCCESS, "idempotent");
    assert_eq!(sync_cmd(root, None, true), ExitCode::SUCCESS, "clean");
}

/// `sync` exits 1 when it WROTE (or, under `--check`, found a node stale).
/// A node it could not read is neither, so it is 2 (`src/cli:V1`, `B11`).
#[test]
fn a_node_sync_cannot_read_is_usage_not_stale() -> Result<(), String> {
    let r = crate::testrepo::TestRepo::new("cli-sync-unread")?;
    let node = r.path().join("a");
    std::fs::create_dir_all(node.join("SPEC.md")).map_err(|e| e.to_string())?;
    assert_eq!(sync_cmd(r.path(), Some(&node), true), ExitCode::from(2));
    assert_eq!(sync_cmd(r.path(), Some(&node), false), ExitCode::from(2));
    Ok(())
}

/// A tree with one slice: `out.txt` is the lead paragraph of `doc.md`.
fn slice_fixture(tag: &str) -> Result<crate::testrepo::TestRepo, String> {
    let r = crate::testrepo::TestRepo::new(tag)?;
    r.write("doc.md", "First paragraph.\n\nSecond paragraph.\n")?;
    r.write(".sherd-slices", "out.txt doc.md lead:1\n")?;
    Ok(r)
}

/// The whole `slice` round trip through the verb: generate, check clean,
/// hand-edit the output, check DRIFT (1), list. `--check` exiting 1 on drift
/// is what the gate's `slice` step relies on, and no test had reached it.
#[test]
fn slice_generates_then_check_catches_a_hand_edit() -> Result<(), String> {
    let r = slice_fixture("cli-slice")?;
    assert_eq!(slice_cmd(r.path(), ""), ExitCode::SUCCESS, "generate");
    let out = std::fs::read_to_string(r.path().join("out.txt"))
        .map_err(|e| e.to_string())?;
    assert!(out.contains("First paragraph."), "{out}");
    assert!(!out.contains("Second paragraph."), "lead:1 is one: {out}");
    assert_eq!(slice_cmd(r.path(), "--check"), ExitCode::SUCCESS, "clean");

    r.write("out.txt", "edited by hand\n")?;
    assert_eq!(slice_cmd(r.path(), "--check"), ExitCode::from(1), "drift");
    assert_eq!(slice_cmd(r.path(), "--list"), ExitCode::SUCCESS);
    Ok(())
}

/// A registry line that is not `<output> <source> <rule>` is usage (2);
/// a declaration whose source matches nothing cannot be rendered (1).
#[test]
fn slice_refuses_a_malformed_registry_and_a_missing_source()
-> Result<(), String> {
    let r = slice_fixture("cli-slice-bad")?;
    r.write(".sherd-slices", "out.txt doc.md\n")?;
    assert_eq!(slice_cmd(r.path(), "--check"), ExitCode::from(2));

    r.write(".sherd-slices", "out.txt missing/*.md lead:1\n")?;
    assert_eq!(slice_cmd(r.path(), "--check"), ExitCode::from(1));
    Ok(())
}
