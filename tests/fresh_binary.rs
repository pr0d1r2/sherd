//! `scripts/fresh-binary.sh`, the body of the `fresh-binary` step in
//! `hk.pkl` (`.:V125`), run against a scratch target directory.
//!
//! The step refuses when a binary's dep-info lists the unpacked
//! `target/package/sherd-<ver>/src` copy, because cargo then calls that
//! binary fresh forever and every dogfood step judges old code (`.:B34`).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// A scratch target directory holding `debug/<bin>.d` for each entry.
fn target(tag: &str, deps: &[(&str, &str)]) -> PathBuf {
    let d = std::env::temp_dir()
        .join(format!("sherd-fresh-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("debug"))
        .unwrap_or_else(|e| panic!("scratch {}: {e}", d.display()));
    for (bin, body) in deps {
        std::fs::write(d.join("debug").join(format!("{bin}.d")), body)
            .unwrap_or_else(|e| panic!("dep-info {bin}: {e}"));
    }
    d
}

fn run(dir: &Path) -> Output {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/fresh-binary.sh");
    Command::new("bash")
        .arg(script)
        .arg(dir)
        .output()
        .unwrap_or_else(|e| panic!("spawn bash: {e}"))
}

fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}

/// The POSITIVE case (`src/fed:V10`): B34's exact dep-info is refused,
/// naming the file and the copy it was built from.
#[test]
fn a_binary_built_from_the_package_copy_is_refused() {
    let d = target(
        "stale",
        &[(
            "sherd",
            "/r/target/debug/sherd: /r/target/package/sherd-0.5.3/src/lib.rs /r/target/package/sherd-0.5.3/src/main.rs\n",
        )],
    );
    let o = run(&d);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("debug/sherd.d"), "{}", stderr(&o));
    assert!(
        stderr(&o).contains("/r/target/package/sherd-0.5.3/"),
        "{}",
        stderr(&o)
    );
    assert!(stderr(&o).contains("cargo clean -p sherd -p sherd-dev"));
    let _ = std::fs::remove_dir_all(&d);
}

/// `sherd-dev` is checked too: `readme-generated` runs it.
#[test]
fn a_stale_sherd_dev_is_refused_too() {
    let d = target(
        "stale-dev",
        &[
            ("sherd", "/r/target/debug/sherd: /r/src/lib.rs\n"),
            (
                "sherd-dev",
                "/r/target/debug/sherd-dev: /r/target/package/sherd-0.5.3/src/lib.rs /r/dev/src/main.rs\n",
            ),
        ],
    );
    let o = run(&d);
    assert_eq!(o.status.code(), Some(1), "{}", stderr(&o));
    assert!(stderr(&o).contains("sherd-dev.d"), "{}", stderr(&o));
    let _ = std::fs::remove_dir_all(&d);
}

/// Built from this tree, or not built yet: the step passes and says how
/// many dep-info files it read, so an empty pass is visible as one.
#[test]
fn a_tree_build_or_no_build_passes_and_says_what_it_examined() {
    let fresh = target(
        "fresh",
        &[(
            "sherd",
            "/r/target/debug/sherd: /r/src/lib.rs /r/src/main.rs\n",
        )],
    );
    let o = run(&fresh);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        String::from_utf8_lossy(&o.stdout)
            .contains("1 dep-info file(s) examined")
    );
    let none = target("none", &[]);
    let o = run(&none);
    assert_eq!(o.status.code(), Some(0), "{}", stderr(&o));
    assert!(
        String::from_utf8_lossy(&o.stdout)
            .contains("0 dep-info file(s) examined")
    );
    let _ = std::fs::remove_dir_all(&fresh);
    let _ = std::fs::remove_dir_all(&none);
}
