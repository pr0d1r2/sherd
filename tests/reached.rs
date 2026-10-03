//! Every file in a `tests/` tree is REACHED by a `#[path]` module (`.:V124`).
//!
//! A test file nothing includes is never compiled, so its tests pass by not
//! existing. That is how `src/adopt:V10` shipped in 0.5.3 with no code behind
//! it: its tests sat in `src/adopt/tests/script.rs` and no module named the
//! file (`.:B33`). Cargo has no opinion about an unreached `.rs` file, so the
//! check lives here.

use std::path::{Path, PathBuf};

/// Each `tests/*.rs` under `dir` that no `.rs` file beside its `tests/`
/// directory names as `#[path = "tests/<file>"]`.
fn unreached(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.flatten().map(|e| e.path()) {
        if !path.is_dir() {
            continue;
        }
        if path.file_name().is_some_and(|n| n == "tests") {
            out.extend(orphans(dir, &path));
        }
        walk(&path, out);
    }
}

/// The files in `tests` that no `.rs` file in `parent` includes.
fn orphans(parent: &Path, tests: &Path) -> Vec<PathBuf> {
    let includes: String = rs_files(parent)
        .iter()
        .filter_map(|f| std::fs::read_to_string(f).ok())
        .collect();
    rs_files(tests)
        .into_iter()
        .filter(|f| {
            let name = f.file_name().map(|n| n.to_string_lossy().into_owned());
            name.is_none_or(|n| !includes.contains(&format!("\"tests/{n}\"")))
        })
        .collect()
}

fn rs_files(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .map(|es| {
            es.flatten()
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "rs"))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn every_test_file_under_src_is_compiled() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for tree in ["src", "dev/src"] {
        let found = unreached(&root.join(tree));
        assert!(found.is_empty(), "never compiled: {found:?}");
    }
}

/// The detector's POSITIVE case (`src/fed:V10`): a check that only ever
/// sees a clean tree is satisfied by one that finds nothing.
#[test]
fn an_unreached_test_file_is_found() {
    let d = std::env::temp_dir()
        .join(format!("sherd-reached-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    let node = d.join("node");
    let write = |p: PathBuf, body: &str| {
        std::fs::create_dir_all(p.parent().unwrap_or(&d))
            .and_then(|()| std::fs::write(&p, body))
            .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    };
    write(
        node.join("mod.rs"),
        "#[cfg(test)]\n#[path = \"tests/kept.rs\"]\nmod kept;\n",
    );
    write(node.join("tests/kept.rs"), "");
    write(node.join("tests/lost.rs"), "");
    assert_eq!(unreached(&d), vec![node.join("tests/lost.rs")]);
    let _ = std::fs::remove_dir_all(&d);
}
