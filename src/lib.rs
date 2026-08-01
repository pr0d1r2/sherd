//! `bbx` -- two-axis federation of `SPEC.md` and source over a dir DAG.
//!
//! Module layout is the federation: every dir here is a node with its own
//! `SPEC.md` (§C: node = dir = Rust module). `mod.rs` composes, it does not
//! implement (V51).

pub mod fed;
pub mod lens;
pub mod plan;
pub mod review;
pub mod spec;
pub mod state;
pub mod tokens;

#[cfg(feature = "ollama")]
pub mod ollama;
#[cfg(feature = "ollama")]
pub mod tdd;
