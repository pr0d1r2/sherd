//! `bbx` -- two-axis federation of `SPEC.md` and source over a dir DAG.
//!
//! Module layout is the federation: every dir here is a node with its own
//! `SPEC.md` (§C: node = dir = Rust module). `mod.rs` composes, it does not
//! implement (V51).

pub mod assay;
pub mod cli;
pub mod code;

// Test-only: a scratch git repo. Compiled into no binary.
pub mod fed;
pub mod land;
pub mod lens;
pub mod plan;
pub mod review;
pub mod slice;
pub mod spec;
pub mod state;
#[cfg(test)]
pub mod testrepo;
pub mod tokens;

#[cfg(feature = "ollama")]
pub mod ollama;
#[cfg(feature = "ollama")]
pub mod tdd;

#[cfg(test)]
mod tests {
    /// Crates this manifest names by PATH. `path = "src/lib.rs"` under
    /// `[lib]`/`[[bin]]` is a target, not a dep: the dep form always names the
    /// crate FIRST.
    fn path_deps(manifest: &str) -> Vec<&str> {
        manifest
            .lines()
            .filter(|l| {
                l.contains("path = \"") && !l.trim_start().starts_with("path")
            })
            .filter_map(|l| l.split_once(' ').map(|(name, _)| name))
            .collect()
    }

    /// Path deps the allow-list does not record. Returned rather than
    /// asserted, so the rule can be exercised against a manifest that HAS one
    /// -- with the list empty, this crate's own manifest can no longer reach
    /// the failing branch, and a check that cannot fire is `.:B6`.
    fn unrecorded<'a>(manifest: &'a str, allowed: &[&str]) -> Vec<&'a str> {
        path_deps(manifest)
            .into_iter()
            .filter(|name| !allowed.contains(name))
            .collect()
    }

    /// V101. A path dep is a shared working tree: the sibling moves and this
    /// crate stops compiling with nothing committed here (B5). `itok` was the
    /// one exception until T71; it is published now, so the list is EMPTY and
    /// a clean clone builds. A dep added by path is a decision, so it fails
    /// here until this list and a §B row say why.
    #[test]
    fn path_deps_are_only_what_the_list_records() {
        const ALLOWED: &[&str] = &[];
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let text =
            std::fs::read_to_string(&manifest).expect("Cargo.toml is readable");
        let found = unrecorded(&text, ALLOWED);
        assert!(
            found.is_empty(),
            "V101: {found:?} named by path -- pin a published version, or record why not"
        );
    }

    #[test]
    fn a_path_dep_is_seen_and_a_target_path_is_not() {
        let manifest = "\
[lib]
path = \"src/lib.rs\"

[dependencies]
sibling = { path = \"../sibling\" }
registry = \"1\"
";
        assert_eq!(path_deps(manifest), vec!["sibling"]);
        assert_eq!(unrecorded(manifest, &[]), vec!["sibling"]);
        assert!(unrecorded(manifest, &["sibling"]).is_empty());
    }
}
