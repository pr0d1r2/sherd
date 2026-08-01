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

/// The notation contract: what the symbols in an invariant MEAN.
///
/// Every prompt that must READ a caveman invariant gets this. A slice of
/// `FORMAT.md`, not the whole file -- V82's contract-not-implementation rule
/// applied to our own prompts (B6).
pub const NOTATION: &str = include_str!("notation.txt");
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

/// Calls a test makes that do not exist yet -- the contract step 2 must fill.
///
/// Deterministic parse, no model (`.:V18`). A run failed when the test called
/// `check_edge_depths(root, &edges)` and step 2 invented a different name, which
/// three repairs could not recover (B12): step 2 was never told what to define.
#[must_use]
pub fn expected_calls(test_src: &str, existing: &str) -> Vec<String> {
    const SKIP: [&str; 18] = ["fn", "if", "for", "while", "match", "let", "return",
        "assert", "assert_eq", "assert_ne", "panic", "println", "format", "vec",
        "write", "read", "Some", "Ok"];
    let b = test_src.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !(b[i].is_ascii_alphabetic() || b[i] == b'_') { i += 1; continue }
        let start = i;
        while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') { i += 1 }
        let name = &test_src[start..i];
        // a call is `name(`; a macro is `name!(`; a method is `.name(`
        if i >= b.len() || b[i] != b'(' { continue }
        if start > 0 && (b[start - 1] == b'.' || b[start - 1] == b'!') { continue }
        // `fn name(` is a DEFINITION, not a call -- including the test's own
        let mut k = start;
        while k > 0 && (b[k - 1] == b' ' || b[k - 1] == b'\t') { k -= 1 }
        if k >= 2 && &test_src[k - 2..k] == "fn" { continue }
        if SKIP.contains(&name) || existing.contains(&format!("fn {name}")) { continue }
        // keep the call verbatim, arguments included -- the signature is the point
        let mut depth = 0usize;
        let mut j = i;
        while j < b.len() {
            if b[j] == b'(' { depth += 1 } else if b[j] == b')' {
                depth -= 1;
                if depth == 0 { break }
            }
            j += 1;
        }
        let call = test_src[start..(j + 1).min(test_src.len())].replace('\n', " ");
        let call = call.split_whitespace().collect::<Vec<_>>().join(" ");
        if !out.contains(&call) { out.push(call) }
    }
    out
}

/// The RULE depth of a spec: §G §C §I §V only.
///
/// §B and §R are rationale and history -- what was tried, what broke, what was
/// measured. A prompt that must AUTHOR a test needs the rule, not the archive.
/// Measured: adding one §B row to a node spec pushed step 1 from 1,210 to 1,520
/// tokens and turned a run that produced correct code into one the judge
/// rejected (B9). V43/V45 in the root spec say rules inline, rationale by
/// reference; this is that rule applied to our own prompts.
#[must_use]
pub fn rule_depth(spec: &str) -> String {
    // §T is the PLAN, not the archive -- the row being implemented names the
    // work. Dropping it cost a run (B9). §B/§R are history and stay out.
    const KEEP: [&str; 5] = ["\u{a7}G", "\u{a7}C", "\u{a7}I", "\u{a7}V", "\u{a7}T"];
    let mut out = String::new();
    let mut keeping = true;
    for line in spec.lines() {
        if line.starts_with("## \u{a7}") {
            keeping = KEEP.iter().any(|k| line.contains(k));
        }
        if keeping {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
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
    let mut pending: Vec<&str> = Vec::new();
    for line in impl_src.lines() {
        let s = line.trim();
        // Doc comments ARE the semantics. Bare field names cannot tell a judge
        // whether `not_owns` holds a path or prose, and that is precisely the
        // question it has to answer (B4).
        if s.starts_with("///") {
            // Inside a type body a doc belongs to the FIELD below it, so emit
            // it in place; at top level it belongs to the item still to come.
            if depth > 0 { out.push_str(line); out.push('\n'); } else { pending.push(line); }
            continue;
        }
        let is_sig = s.starts_with("pub fn") || s.starts_with("pub struct")
            || s.starts_with("pub enum") || s.starts_with("pub const");
        if depth > 0 {
            // inside a type body: keep field lines, they are part of the shape
            if s == "}" { depth = 0; out.push_str("}\n"); }
            else if !s.is_empty() { out.push_str(line); out.push('\n'); }
            continue;
        }
        if !is_sig { pending.clear(); }
        if is_sig {
            for d in pending.drain(..) { out.push_str(d); out.push('\n'); }
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
    use std::io::Write;
    // Count locally too: the server's number and ours must agree, and a
    // silent divergence means the prompt is not what this code thinks it is.
    let local = crate::tokens::count(prompt);
    // Say what is being sent, and what it should COST, before sending it. A
    // silent 40-90s wait is indistinguishable from a hang (V21), and a
    // prediction is what lets the escalation guards mean anything.
    let eta = ollama::predict_for(label, local.tokens);
    eprintln!("  [{label}] -> {} tok ({:.1} KB) - eta {:.0}s cold / {:.0}s if cached (~{} gen)",
              local.tokens, prompt.len() as f64 / 1024.0,
              eta.total_s(), eta.cached_s(), eta.gen_est);
    eprint!("       ");
    let _ = std::io::stderr().flush();
    if ollama::verbose() {
        eprintln!("\n--- prompt [{label}] ---\n{prompt}\n--- end prompt ---");
    }
    let mut n = 0usize;
    let r = ollama::generate_with(prompt, eta, &mut |chunk| {
        if ollama::verbose() {
            eprint!("{chunk}");
        } else {
            n += 1;
            // One dot per ~25 chunks: visible motion, not a firehose.
            if n % 25 == 0 {
                eprint!(".");
            }
        }
        let _ = std::io::stderr().flush();
    })?;
    eprintln!();
    ollama::observe_gen(label, r.eval_tokens);
    if ollama::verbose() {
        if !r.thinking.is_empty() {
            eprintln!("\n--- reasoning [{label}] ---\n{}\n--- end reasoning ---", r.thinking);
        }
        eprintln!("--- end reply [{label}] ---");
    }
    // Prediction against telemetry -- the comparison is the point. A delta
    // that stays large means the pace model is wrong about THIS endpoint.
    let actual = r.ms as f64 / 1000.0;
    let basis = if ollama::last_cached() { eta.cached_s() } else { eta.total_s() };
    let delta = (actual - basis) / basis * 100.0;
    eprintln!("  [{label}] <- {} sent · {} gen · {actual:.1}s (eta {:.0}s, {delta:+.0}%){}",
              r.prompt_tokens, r.eval_tokens, basis,
              if ollama::last_cached() { "  [prefix CACHED]" } else { "" });
    if !r.thinking.is_empty() {
        eprintln!("       (+{} reasoning tokens, hidden -- see -v)", r.thinking.len() / 4);
    }
    if r.prompt_tokens.abs_diff(local.tokens) > local.tokens / 10 {
        eprintln!("  [{label}] note: local count {} vs server {} -- >10% apart",
                  local.tokens, r.prompt_tokens);
    }
    log.push(Step { label, prompt_tokens: r.prompt_tokens, eval_tokens: r.eval_tokens, ms: r.ms });
    Ok(r.text)
}

/// The MONOLITH arm of the premise gate (root V60): everything in one call.
///
/// Full spec including §B/§R, full implementation bodies, full tests, asked
/// for test AND implementation together. This is what blackbox claims to beat.
/// Same gate, same node, same invariant -- only the context shape differs.
///
/// # Errors
/// Returns the reason it could not proceed, same as [`drive`].
pub fn oneshot(root: &Path, node: &Path, invariant: &str, task: &str) -> Result<Vec<Step>, String> {
    let spec_path = node.join("SPEC.md");
    let mod_path = node.join("mod.rs");
    let spec_txt = std::fs::read_to_string(&spec_path).map_err(|e| e.to_string())?;
    let original = std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
    let (impl_r, tests_r) = split_module(&original);
    let inv = spec_txt.lines().find(|l| l.starts_with(&format!("{invariant}:")))
        .ok_or_else(|| format!("{invariant} not declared"))?.to_string();
    let mut log = Vec::new();

    let reply = run(&format!(
        "{}\n--- spec (complete) ---\n{spec_txt}\n\n\
         --- implementation (complete) ---\n{impl_r}\n\n\
         --- existing tests ---\n{tests_r}\n\n\
         Prove and implement this invariant:\n  {inv}\n\nTask: {task}\n\n\
         Reply with TWO ```rust fenced blocks: first the new `#[test]` function, \
         then the new implementation function(s) to add. The test must fail against \
         the current implementation and pass against your new one.", NOTATION), "monolith", &mut log)?;

    let blocks: Vec<&str> = reply.split("```").skip(1).step_by(2)
        .map(|b| b.strip_prefix("rust").unwrap_or(b).trim()).collect();
    if blocks.len() < 2 {
        return Err(format!("monolith returned {} code blocks, expected 2", blocks.len()));
    }
    std::fs::write(&mod_path, insert_impl(&insert_test(&original, blocks[0]), blocks[1]))
        .map_err(|e| e.to_string())?;
    let (ok, out) = gate(root);
    let sent: u64 = log.iter().map(|s| s.prompt_tokens).sum();
    eprintln!("\n  1 round-trip · {sent} tok sent · max single call {sent}");
    if ok { eprintln!("  VERDICT: MERGEABLE -- gates green"); Ok(log) }
    else { eprintln!("{}", tail(&out, 1200)); Err("NOT mergeable -- gates red".into()) }
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

    let spec_rules = rule_depth(&spec_txt);
    let surface = signatures(impl_r);
    let mut log = Vec::new();

    // 1 -- RED test, with the judge's objection fed back on rejection. The
    // judge's reason is actionable signal; discarding it and hand-tuning the
    // prompt instead is what I did for six configurations before noticing (B10).
    let base = format!(
        "{NOTATION}\n--- spec (rules) ---\n{spec_rules}\n\n\
         --- public surface (signatures only) ---\n{surface}\n\n\
         --- existing tests in this module ---\n{tests_r}\n\n\
         Write ONE new Rust `#[test]` function proving this invariant:\n  {inv}\n\n\
         Task: {task}\n\n\
         It must FAIL against the current implementation, and fail at an assertion -- \
         not by failing to compile. It MUST include data that actually violates the \
         invariant, and assert that the violation is reported. Use only items that \
         already exist, plus the ONE new public function you expect to be written. \
         Reply with a single ```rust fenced block containing only the test function.");

    let mut test_fn = String::new();
    let mut accepted = false;
    let mut objection = String::new();
    for attempt in 0..3 {
        let prompt = if attempt == 0 {
            base.clone()
        } else {
            format!("{base}\n\nYour previous attempt was REJECTED by review:\n\
                     ```rust\n{test_fn}\n```\nReason: {objection}\n\
                     Write a corrected test that answers that objection.")
        };
        let label: &'static str = if attempt == 0 { "1 red-test" } else { "1 red-retry" };
        test_fn = ollama::rust_block(&run(&prompt, label, &mut log)?);

        let verdict = run(&format!(
            "{NOTATION}\n--- data model ---\n{surface}\n\nInvariant:\n  {inv}\n\n\
             Proposed test:\n```rust\n{test_fn}\n```\n\n\
             Answer YES only if BOTH hold: (a) the test exercises the quantity the \
             invariant is actually about -- check the field names against the data model \
             above, a test asserting on the wrong field proves nothing; and (b) an \
             implementation violating the invariant would fail it. Exhaustiveness is NOT \
             required. Answer YES or NO on the first line, then one sentence."),
            if attempt == 0 { "1b judge" } else { "1b re-judge" }, &mut log)?;
        let first = verdict.trim().lines().next().unwrap_or("").to_string();
        eprintln!("  judge: {}", first.chars().take(78).collect::<String>());
        if first.trim().to_uppercase().starts_with("YES") { accepted = true; break }
        objection = verdict.trim().to_string();
    }
    if !accepted {
        return Err(format!("judge rejected the test 3 times -- last objection: {objection}"));
    }

    std::fs::write(&mod_path, insert_test(&original, &test_fn)).map_err(|e| e.to_string())?;
    let (red_ok, red_out) = gate(root);
    if red_ok {
        std::fs::write(&mod_path, &original).map_err(|e| e.to_string())?;
        return Err("test passes already -- not a red test, nothing to drive".into());
    }
    eprintln!("  gate: RED as required");

    // 2 -- GREEN. Sees the one test and the implementation, not the whole spec.
    let wanted = expected_calls(&test_fn, &surface);
    let contract = if wanted.is_empty() { String::new() } else {
        format!("--- the test calls these; define EXACTLY these names and signatures ---\n{}\n\n",
                wanted.join("\n"))
    };
    eprintln!("  contract: {}", if wanted.is_empty() { "(none detected)".into() } else { wanted.join(", ") });
    let code = ollama::rust_block(&run(&format!(
        "--- existing API (signatures; call these, do not reimplement) ---\n{surface}\n\n--- failing test ---\n```rust\n{test_fn}\n```\n\n\
         {contract}\
         --- failure ---\n{}\n\n\
         Write ONLY the new function(s) to ADD to the implementation so this test passes. \
         Do not restate existing code. \
         Do not modify the test. Reply with a single ```rust fenced block.", tail(&red_out, 1500)), "2 green", &mut log)?);
    let cur = std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
    std::fs::write(&mod_path, insert_impl(&cur, &code)).map_err(|e| e.to_string())?;
    // Track exactly what we added, so repair REPLACES it rather than guessing
    // at a name prefix or rewriting the whole region (B5).
    let mut last_added = code.clone();

    let (mut ok, mut out) = gate(root);
    // 4 -- repair, capped. On exhaustion, report what was tried.
    for i in 0..max_repair {
        if ok { break }
        eprintln!("  gate: FAIL -- repair {}/{}", i + 1, max_repair);
        let cur = std::fs::read_to_string(&mod_path).map_err(|e| e.to_string())?;
        let (cur_impl, cur_tests) = split_module(&cur);
        let cur_surface = signatures(cur_impl);
        let label: &'static str = if i == 0 { "4 repair-1" } else { "4 repair-n" };
        let fixed = ollama::rust_block(&run(&format!(
            "--- existing API (signatures) ---\n{cur_surface}\n\n--- your current attempt ---\n{last_added}\n\n--- test ---\n```rust\n{test_fn}\n```\n\n\
             --- failure ---\n{}\n\n\
             Reply with ONLY the corrected version of the function(s) you previously \
             added, in one ```rust block. Do not restate unrelated code, do not remove \
             module documentation, and do not change the behaviour of functions that \
             already existed. Do not modify the test.",
            tail(&out, 2000)), label, &mut log)?);
        let replaced = if cur_impl.contains(last_added.trim()) {
            cur_impl.replace(last_added.trim(), fixed.trim())
        } else {
            // Could not find what we added -- refuse to guess. Appending would
            // duplicate the definition, rewriting would destroy unrelated code.
            return Err("repair lost track of the previous insertion -- refusing to                         guess where it went".into());
        };
        last_added = fixed.clone();
        std::fs::write(&mod_path, format!("{}\n\n{}", replaced.trim_end(), cur_tests))
            .map_err(|e| e.to_string())?;
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
/// Classify a failure report as a compile‑time error.
///
/// The function returns `true` if the given report looks like a Rust compiler
/// error (e.g., starts with `"error"` or contains an error code such as
/// `"error[E0425]"`).  All other reports, including assertion failures,
/// are considered non‑compile errors and return `false`.
pub fn classify_failure(report: &str) -> bool {
    let s = report.trim_start();
    // Most compiler errors start with "error" or contain an error code in brackets.
    s.starts_with("error") || s.contains("error[")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "pub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";

    #[test]
    fn expected_calls_finds_the_undefined_one_only() {
        let t = "#[test]\nfn x() {\n    let e = edges(\"a\");\n    let v = check_edge_depths(root, &e);\n    assert!(v.is_empty());\n    e.len();\n}";
        let existing = "pub fn edges(text: &str) -> Vec<Edge> { }";
        let c = expected_calls(t, existing);
        assert_eq!(c, vec!["check_edge_depths(root, &e)"], "got {c:?}");
    }

    #[test]
    fn expected_calls_skips_macros_and_methods() {
        let t = "assert_eq!(a, b); x.len(); vec![1];";
        assert!(expected_calls(t, "").is_empty(), "{:?}", expected_calls(t, ""));
    }

    #[test]
    fn rule_depth_drops_the_archive_sections() {
        let s = "## \u{a7}G GOAL\ngoal\n\n## \u{a7}V INVARIANTS\nV1: a\n\n## \u{a7}B BUGS\nB1|x|cause|fix\n";
        let r = rule_depth(s);
        assert!(r.contains("V1: a"), "rules must survive: {r}");
        assert!(!r.contains("B1|"), "§B must be dropped: {r}");
        let s2 = "## \u{a7}T TASKS\nT1|.|do the thing|V1\n\n## \u{a7}B BUGS\nB1|x|c|f\n";
        assert!(rule_depth(s2).contains("T1|"), "§T is the plan and must survive");
        assert!(!r.contains("BUGS"), "§B header must be dropped: {r}");
    }

    #[test]
    fn signatures_keep_shape_and_drop_bodies() {
        let src = "/// what it owns\npub struct E {\n    /// a path\n    pub dir: String,\n}\n\n/// does the thing\npub fn go(a: u8) -> bool {\n    secret();\n    true\n}\n";
        let s = signatures(src);
        assert!(s.contains("/// a path"), "doc comments ARE the semantics: {s}");
        assert!(s.contains("/// does the thing"), "fn docs must survive: {s}");
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

#[test]
fn classify_failure_works() {
    // A typical compiler error – represents a red test that fails to build.
    let compile_report = r#"error[E0425]: cannot find value `foo` in this scope"#;

    // An assertion failure message – represents a red test that runs but panics.
    let assert_report =
        "thread 'main' panicked at 'assertion failed: x == y', src/main.rs:10:5";

    // The new public function we expect to be written:
    //   pub fn classify_failure(report: &str) -> bool
    //
    // It should return true for compile‑time failures and false otherwise.
    assert!(
        classify_failure(compile_report),
        "Compile error should be classified as a compile failure"
    );

    // A red test that merely panics must NOT be treated as a compile failure.
    assert!(
        !classify_failure(assert_report),
        "Assertion failure should not be classified as a compile failure"
    );
}
}
