//! `bbx` -- two-axis federation of `SPEC.md` and source over a dir DAG.
//!
//! Module layout is the federation: every dir here is a node with its own
//! `SPEC.md` (§C: node = dir = Rust module). `mod.rs` composes, it does not
//! implement (V51).

pub mod tokens;
