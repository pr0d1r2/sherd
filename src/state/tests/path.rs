use super::*;

fn tmp(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    // Unique per INSTANCE, not per process -- `src/review:V6`, learned
    // when two fixtures sharing a tag deleted each other's directory.
    std::env::temp_dir()
        .join(format!("sherd-state-{tag}-{}-{n}", std::process::id()))
}

#[test]
fn a_state_remembers_where_it_came_from() {
    let p = tmp("roundtrip");
    let mut a = State::at(&p);
    a.set("obs", "k", "v");
    a.save();
    let b = State::at(&p);
    assert_eq!(b.get("obs", "k"), Some("v"), "save must write to `path`");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn two_states_at_different_paths_share_nothing() {
    // THE POINT of T13. Before this, every test in the process wrote the
    // same `.sherd-state`, so `derived_prefill` branched on what a previous
    // test had left and coverage measured 75.26-75.35% for one unchanged
    // tree (`src/ollama:B8`).
    let (p, q) = (tmp("iso-a"), tmp("iso-b"));
    let mut a = State::at(&p);
    a.set("gen", "1 red", "2500");
    a.save();
    let b = State::at(&q);
    assert_eq!(b.get("gen", "1 red"), None, "a fresh path is a cold start");
    let _ = std::fs::remove_file(&p);
    let _ = std::fs::remove_file(&q);
}

#[test]
fn a_missing_file_is_a_cold_start_not_an_error() {
    let s = State::at(tmp("absent"));
    assert_eq!(s.all("obs").len(), 0);
}

#[test]
fn the_default_path_is_read_from_the_environment_at_the_edge() {
    // `SHERD_STATE`, else `.sherd-state`. Read HERE and nowhere below, which
    // is what let the path become a parameter -- `std::env::set_var` is
    // unsafe under edition 2024 and this crate forbids unsafe, so a
    // per-test env var was never an option (`src/ollama:V19`).
    let p = default_path();
    assert!(p.to_string_lossy().contains("sherd-state"), "{p:?}");
}
