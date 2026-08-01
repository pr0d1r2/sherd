//! TDD as separate LLM round-trips, each with a minimal declared profile.
//!
//! Monolithic TDD loads ~81% of a repo, because one call needs spec AND tests
//! AND implementation (root V87). Split into round-trips, no single call needs
//! all three -- measured ~1.5k tokens max on this repo against 157k.
//!
//!   1  RED test      spec + existing tests        (NOT the implementation)
//!   1b judge         invariant + the test         (NOT the implementation)
//!   2  GREEN         the test + implementation    (NOT the whole spec)
//!   3  gate          LOCAL, deterministic, ZERO tokens
//!   4  repair        test + impl + failure, capped
//!
//! Step 1b exists for the failure no other guard catches: one model writes a
//! test encoding its own misreading of an invariant, then implements to match,
//! and everything passes. The judge never sees the implementation.

use crate::{fed, ollama, spec};
use std::path::Path;
use std::process::Command;

/// What one round-trip cost. Steps without their cost are not evidence.
#[derive(Debug)]
pub struct Step {
    pub label: &'static str,
    pub prompt_tokens: u64,
    pub eval_tokens: u64,
    pub ms: u128,
}

/// Split a Rust source file at the `#[cfg(test)]` boundary.
///
/// One definition, because the code ceiling (root V50) needs exactly this
/// split -- code and tests counted separately -- and two readings of one rule
/// is the defect this project exists to end.
#[must_use]
pub fn split_module(src: &str) -> (&str, &str) {
    match src.find("\n#[cfg(test)]") {
        Some(i) => (&src[..i + 1], &src[i + 1..]),
        None => (src, ""),
    }
}

/// The public SURFACE of an implementation: signatures and type shapes, no
/// bodies. Step 1 needs this and must not have the bodies -- it is `§I`, not
/// `§V`. Written after a run where the test author, given only the spec, could
/// not see `Edge`'s fields and reached for the wrong one (B1 here).
#[must_use]
pub fn signatures(impl_src: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for line in impl_src.lines() {
        let s = line.trim();
        let is_sig = s.starts_with("pub fn") || s.starts_with("pub struct")
            || s.starts_with("pub enum") || s.starts_with("pub const");
        if depth > 0 {
            // inside a type body: keep field lines, they are part of the shape
            if s == "}" { depth = 0; out.push_str("}\n"); }
            else if !s.starts_with("//") && !s.is_empty() { out.push_str(line); out.push('\n'); }
            continue;
        }
        if is_sig {
            if s.starts_with("pub fn") {
                let sig = s.split('{').next().unwrap_or(s).trim_end();
                out.push_str(sig); out.push_str(" { /* ... */ }\n");
            } else {
                out.push_str(line); out.push('\n');
                if s.ends_with('{') { depth = 1; }
            }
        }
    }
    out
}

/// Append a test into the tests module. The ONLY function that writes there --
/// steps 2 and 4 structurally cannot touch the test, which is the guard
/// against an implementation that games it.
fn insert_test(src: &str, test_fn: &str) -> String {
    let idx = src.trim_end().rfind('}').unwrap_or(src.len());
    format!("{}\n{}\n{}", &src[..idx], test_fn.trim_end(), &src[idx..])
}

/// Append to the implementation region, above `#[cfg(test)]`.
fn insert_impl(src: &str, code: &str) -> String {
    let (impl_r, tests) = split_module(src);
    format!("{}\n{}\n\n{}", impl_r.trim_end(), code.trim(), tests)
}

/// Step 3. Local, deterministic, zero tokens. Reports what RAN, not only what
/// failed (root V48).
fn gate(root: &Path) -> (bool, String) {
    let cargo = std::env::var("BBX_CARGO").unwrap_or_else(|_| "cargo".into());
    let out = Command::new(&cargo).args(["test", "--offline"]).current_dir(root).output();
    let (tests_ok, mut report) = match out {
        Ok(o) => {
            let s = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            (o.status.success(), format!("=== cargo test: {} ===\n{}", if o.status.success() { "PASS" } else { "FAIL" }, tail(&s, 2500)))
        }
        Err(e) => (false, format!("=== cargo test: COULD NOT RUN ===\n{e}")),
    };
    // spec::check runs in-process -- no subprocess, no stdout scraping.
    let mut viol = 0;
    let nodes = fed::discover(root);
    for n in &nodes {
        if let Ok(t) = std::fs::read_to_string(n.join("SPEC.md")) {
            viol += spec::check(&t).len();
        }
    }
    report.push_str(&format!("\n=== bbx check: {} === {} nodes examined, {viol} violations\n",
        if viol == 0 { "PASS" } else { "FAIL" }, nodes.len()));
    (tests_ok && viol == 0, report)
}

fn tail(s: &str, n: usize) -> &str {
    if s.len() <= n { s } else { &s[s.len() - n..] }
}

fn run(prompt: &str, label: &'static str, log: &mut Vec<Step>) -> Result<String, String> {
    // Count locally too: the server's number and ours must agree, and a
    // silent divergence means the prompt is not what this code thinks it is.
    let local = crate::tokens::count(prompt);
    let r = ollama::generate(prompt)?;
    if r.prompt_tokens.abs_diff(local.tokens) > local.tokens / 10 {
        eprintln!("  [{label}] WARNING local {} vs server {} -- >10% apart",
                  local.tokens, r.prompt_tokens);
    }
    eprintln!("  [{label}] sent {} tok · gen {} · {}ms", r.prompt_tokens, r.eval_tokens, r.ms);
    log.push(Step { label, prompt_tokens: r.prompt_tokens, eval_tokens: r.eval_tokens, ms: r.ms });
    Ok(r.text)
}

/// Drive one invariant from red to green.
///
/// # Errors
/// Returns the reason the loop could not proceed. A rejected test, a test that
/// is already green, or an exhausted repair budget are all reported -- never
/// silently swallowed.
pub fn drive(root: &Path, node: &Path, invariant: &str, task: &str, max_repair: usize)
    -> Result<Vec<Step>, String>
{
    let spec_path = node.join("SPEC.md");
    let mod_path = node.join("mod.rs");
    let spec_txt = std::fs::read_to_string(&spec_path).map_err(|e| format!("{}: {e}", spec_path.display()))?;
    let original = std::fs::read_to_string(&mod_path).map_err(|e| format!("{}: {e}", mod_path.display()))?;
    let (impl_r, tests_r) = split_module(&original);

    let inv = spec_txt.lines().find(|l| l.starts_with(&format!("{invariant}:")))
        .ok_or_else(|| format!("{invariant} not declared in {} -- a test for an invariant that does not exist encodes an unstated rule", spec_path.display()))?
        .to_string();
    eprintln!("node {} · {}\n", node.display(), inv.trim());

    let surface = signatures(impl_r);
    let mut log = Vec::new();

    // 1 -- RED test. Sees spec and existing tests, not the implementation.
    let test_fn = ollama::rust_block(&run(&format!(
        "{spec_txt}\n\n--- public surface (signatures only) ---\n{surface}\n\n\
         --- existing tests in this module ---\n{tests_r}\n\n\
         Write ONE new Rust `#[test]` function proving this invariant:\n  {inv}\n\n\
         Task: {task}\n\n\
         It must FAIL against the current implementation, and fail at an assertion -- \
         not by failing to compile. Use only items that already exist, plus the ONE new \
         public function you expect to be written. Reply with a single ```rust fenced \
         block containing only the test function."), "1 red-test", &mut log)?);

    // 1b -- independent judge. Sees the invariant and the test, never the impl.
    let verdict = run(&format!(
        "--- data model ---\n{surface}\n\nInvariant:\n  {inv}\n\n\
         Proposed test:\n```rust\n{test_fn}\n```\n\n\
         Answer YES only if BOTH hold: (a) the test exercises the quantity the \
         invariant is actually about -- check the field names against the data model \
         above, a test asserting on the wrong field proves nothing; and (b) an \
         implementation violating the invariant would fail it. Exhaustiveness is NOT \
         required. Answer YES or NO on the first line, then one sentence."), "1b judge", &mut log)?;
    let first = verdict.trim().lines().next().unwrap_or("").to_string();
    eprintln!("  judge: {}", first.chars().take(80).collect::<String>());
    if !first.trim().to_uppercase().starts_with("YES") {
        return Err(format!("judge rejected the test before it became law: {verdict}"));
    }

    std::fs::write(&mod_path, insert_test(&original, &test_fn)).map_err(|e| e.to_string())?;
    let (red_ok, red_out) = gate(root);
    if red_ok {
        std::fs::write(&mod_path, &original).map_err(|e| e.to_string())?;
        return Err("test passes already -- not a red test, nothing to drive".into());
    }
    eprintln!("  gate: RED as required");

    // 2 -- GREEN. Sees the one test and the implementation, not the whole spec.
    let code = ollama::rust_block(&run(&format!(
        "--- implementation ---\n{impl_r}\n\n--- failing test ---\n```rust\n{test_fn}\n```\n\n\
         --- failure ---\n{}\n\n\
         Write ONLY the new function(s) to ADD to the implementation so this test passes. \
         Do not restate existing code. Do not modify the test. Reply with a single ```rust \
         fenced block.", tail(&red_out, 1500)), "2 green", &mut log)?);
    let cur = std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
    std::fs::write(&mod_path, insert_impl(&cur, &code)).map_err(|e| e.to_string())?;

    let (mut ok, mut out) = gate(root);
    // 4 -- repair, capped. On exhaustion, report what was tried.
    for i in 0..max_repair {
        if ok { break }
        eprintln!("  gate: FAIL -- repair {}/{}", i + 1, max_repair);
        let cur = std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
        let (cur_impl, cur_tests) = split_module(&cur);
        let label: &'static str = if i == 0 { "4 repair-1" } else { "4 repair-n" };
        let fixed = ollama::rust_block(&run(&format!(
            "--- implementation ---\n{cur_impl}\n\n--- test ---\n```rust\n{test_fn}\n```\n\n\
             --- failure ---\n{}\n\n\
             Reply with the corrected FULL implementation region (everything above \
             #[cfg(test)]) in one ```rust block. Do not modify the test.",
            tail(&out, 2000)), label, &mut log)?);
        std::fs::write(&mod_path, format!("{}\n\n{}", fixed.trim(), cur_tests)).map_err(|e| e.to_string())?;
        let g = gate(root);
        ok = g.0;
        out = g.1;
    }

    let sent: u64 = log.iter().map(|s| s.prompt_tokens).sum();
    let max = log.iter().map(|s| s.prompt_tokens).max().unwrap_or(0);
    eprintln!("\n  {} round-trips · {sent} tok sent · max single call {max}", log.len());
    if ok {
        eprintln!("  VERDICT: MERGEABLE -- gates green");
        Ok(log)
    } else {
        eprintln!("{}", tail(&out, 2000));
        Err("NOT mergeable -- gates red after repair budget".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "pub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";

    #[test]
    fn signatures_keep_shape_and_drop_bodies() {
        let src = "pub struct E {\n    pub dir: String,\n}\n\npub fn go(a: u8) -> bool {\n    secret();\n    true\n}\n";
        let s = signatures(src);
        assert!(s.contains("pub dir: String"), "field shape must survive: {s}");
        assert!(s.contains("pub fn go(a: u8) -> bool"), "signature must survive: {s}");
        assert!(!s.contains("secret()"), "body must NOT survive: {s}");
    }

    #[test]
    fn split_finds_the_test_boundary() {
        let (i, t) = split_module(SRC);
        assert!(i.contains("pub fn a"));
        assert!(!i.contains("cfg(test)"));
        assert!(t.starts_with("#[cfg(test)]"));
    }

    #[test]
    fn split_of_a_file_with_no_tests_is_all_impl() {
        let (i, t) = split_module("pub fn a() {}\n");
        assert_eq!(i, "pub fn a() {}\n");
        assert_eq!(t, "");
    }

    #[test]
    fn insert_impl_never_touches_the_test_region() {
        let out = insert_impl(SRC, "pub fn b() {}");
        let (_, t) = split_module(&out);
        assert_eq!(t, split_module(SRC).1, "test region must be byte-identical");
        assert!(out.contains("pub fn b"));
    }

    #[test]
    fn insert_test_lands_inside_the_tests_module() {
        let out = insert_test(SRC, "    #[test]\n    fn u() {}");
        assert!(split_module(&out).1.contains("fn u()"));
    }
}
