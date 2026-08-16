//! `bbx` -- two-axis federation of `SPEC.md` and source over a dir DAG.
//!
//! Module layout is the federation: every dir here is a node with its own
//! `SPEC.md` (§C: node = dir = Rust module). `mod.rs` composes, it does not
//! implement (V51).

pub mod cli;
pub mod code;
pub mod fed;
pub mod land;
pub mod lens;
pub mod plan;
pub mod review;
pub mod slice;
pub mod spec;
pub mod state;
pub mod tokens;

#[cfg(feature = "ollama")]
pub mod ollama;
#[cfg(feature = "ollama")]
pub mod tdd;

#[cfg(test)]
mod tests {
    /// V101. A path dep is a shared working tree: the sibling moves and this
    /// crate stops compiling with nothing committed here (B5). `itok` is the
    /// one recorded exception -- unpublished, no public mirror -- and T71
    /// removes it. A dep added by path is a decision, so it fails here until
    /// this list and a §B row say why.
    #[test]
    fn path_deps_are_the_one_recorded_exception() {
        const ALLOWED: &[&str] = &["itok"];
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let text =
            std::fs::read_to_string(&manifest).expect("Cargo.toml is readable");
        let by_path: Vec<&str> = text
            .lines()
            // `path = "src/lib.rs"` under [lib]/[[bin]] is a target, not a dep:
            // the dep form always names the crate FIRST.
            .filter(|l| {
                l.contains("path = \"") && !l.trim_start().starts_with("path")
            })
            .filter_map(|l| l.split_once(' ').map(|(name, _)| name))
            .collect();
        for name in &by_path {
            assert!(
                ALLOWED.contains(name),
                "V101: `{name}` is a path dep -- pin a published version, or record why not"
            );
        }
    }
}
