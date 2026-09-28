use super::*;

#[test]
fn flags_a_fn_only_tests_call() {
    let f = unwired(
        "pub fn helper(x: u8) -> bool { true }\n",
        "assert!(helper(1));",
        &["helper".into()],
    );
    assert_eq!(f.len(), 1, "is_ignored_dir landed exactly like this");
    assert_eq!(f.first().map(|x| x.rule), Some("unwired"));
}

/// `src/debt:§C`: ONE walker. `node` had its own, which never skipped
/// `target/` and concatenated in `read_dir` order -- so the same tree
/// could yield a different `crate_src` between runs, and `unwired` and
/// `duplication` both read that string.
///
/// Determinism is the assertion, because "it happened to agree" is what a
/// filesystem-ordered walk gives you until it does not.
#[test]
fn the_crate_source_is_assembled_in_a_stable_order() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let read = || -> String {
        crate::fed::rust_files(&root)
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .map(|s| split_module(&s).0.to_string())
            .collect()
    };
    assert_eq!(read(), read(), "two reads of one tree are one string");
    assert!(
        !crate::fed::rust_files(&root)
            .iter()
            .any(|p| p.components().any(|c| c.as_os_str() == "target")),
        "a build product is not crate source"
    );
}

/// `T3`/`.:B13`: two functions recognising the same markers are two
/// readings of one parse. The real case was cross-node -- `src/tdd` and
/// `src/review` both read Rust as text -- so this searches the whole
/// crate like `unwired` does.
#[test]
fn a_new_fn_recognising_an_existing_fn_s_markers_is_flagged() {
    let src = "fn old(s: &str) -> bool {\n    \
                   s.starts_with(\"## \u{a7}T\") && s.contains(\"status\")\n}\n\
                   fn fresh(s: &str) -> bool {\n    \
                   s.contains(\"## \u{a7}T\") || s.contains(\"status\")\n}\n";
    let f = duplication(src, &["fresh".to_string()]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f.first().map(|x| x.rule), Some("duplication"));
    assert!(
        f.first().is_some_and(|x| x.detail.contains("`old`")),
        "the report names what it duplicates: {f:?}"
    );
}

/// A function that CALLS the other reuses it rather than re-parsing, so
/// sharing markers with it is not a finding -- that is the fix, not the
/// defect.
#[test]
fn calling_the_existing_fn_is_reuse_and_not_a_finding() {
    let src = "fn old(s: &str) -> bool {\n    \
                   s.starts_with(\"## \u{a7}T\") && s.contains(\"status\")\n}\n\
                   fn fresh(s: &str) -> bool {\n    \
                   old(s) && s.contains(\"## \u{a7}T\") && s.contains(\"status\")\n}\n";
    assert!(duplication(src, &["fresh".to_string()]).is_empty());
}

/// ONE shared marker is not evidence. Measured against this crate a
/// threshold of one fires on every pair mentioning `SPEC.md`; two leaves
/// seven pairs, of which three are real.
#[test]
fn one_shared_marker_is_below_the_threshold() {
    let src = "fn old(s: &str) -> bool {\n    \
                   s.starts_with(\"## \u{a7}T\") && s.contains(\"other\")\n}\n\
                   fn fresh(s: &str) -> bool {\n    \
                   s.contains(\"## \u{a7}T\") && s.contains(\"unrelated\")\n}\n";
    assert!(duplication(src, &["fresh".to_string()]).is_empty());
}

/// The three FIX SHAPES, each on the historical example it was mined
/// from. Named, never applied (`V3`): `cargo clippy --fix` changes
/// nothing on this tree, because all 252 warnings we carry are the ones
/// upstream marks as needing judgement (`.:B24`).
#[test]
fn a_length_check_beside_positional_indexes_names_the_slice_pattern() {
    // `fed::parses_rows_and_stops_at_next_section` before the rewrite.
    let src = "fn t() {\n    let e = edges(F);\n    \
                   assert_eq!(e.len(), 2);\n    assert_eq!(e[0].dir, \"src\");\n    \
                   assert_eq!(e[1].tokens, None);\n}\n";
    let f = fix_shapes(src, &["t".to_string()]);
    assert_eq!(f.first().map(|x| x.rule), Some("positional-index"), "{f:?}");
    assert!(
        f.first().is_some_and(|x| x.detail.contains("as_slice()")),
        "the finding names the shape to write: {f:?}"
    );
}

/// One index with no length check is not the shape -- a `match` arm that
/// already proved the arity indexes safely.
#[test]
fn a_single_index_without_a_length_check_is_not_the_shape() {
    let src = "fn t() {\n    let v = go();\n    use_it(v[0]);\n}\n";
    assert!(fix_shapes(src, &["t".to_string()]).is_empty());
}

/// `code::expected_calls` before the split: 75 lines, 11 byte indexes.
#[test]
fn a_long_byte_scanner_names_the_named_boundary_shape() {
    let mut src = String::from("fn t(s: &str) {\n    let b = s.as_bytes();\n");
    for _ in 0..16 {
        src.push_str("    if b[i] == b'x' { i += 1; }\n");
    }
    src.push_str("}\n");
    let f = fix_shapes(&src, &["t".to_string()]);
    assert!(f.iter().any(|x| x.rule == "byte-scanner"), "{f:?}");
}

/// `review::added_in_commit` before `V7`: `Ok` means the process RAN, and
/// `git show <unknown rev>` then looked like an empty diff.
#[test]
fn a_subprocess_whose_status_is_never_read_is_named() {
    let src = "fn t() {\n    let out = Command::new(\"git\").output();\n    \
                   let Ok(out) = out else { return };\n    \
                   let diff = String::from_utf8_lossy(&out.stdout);\n}\n";
    let f = fix_shapes(src, &["t".to_string()]);
    assert!(f.iter().any(|x| x.rule == "status-unread"), "{f:?}");
    // And reading the status clears it.
    let ok = src.replace(
        "let Ok(out) = out",
        "let Ok(out) = out.filter(|o| o.status.success())",
    );
    assert!(
        !fix_shapes(&ok, &["t".to_string()])
            .iter()
            .any(|x| x.rule == "status-unread")
    );
}

/// `V5`: the report names every rule that ran. The list drifted once --
/// `undocumented` shipped and the report never mentioned it -- so this
/// asserts the count rather than trusting a format string two files away.
#[test]
fn every_rule_that_runs_is_named_in_the_report() {
    assert_eq!(RULES.len(), 8);
    for r in [
        "unwired",
        "negative-only",
        "ignored-input",
        "undocumented",
        "duplication",
        "positional-index",
        "byte-scanner",
        "status-unread",
    ] {
        assert!(RULES.contains(&r), "{r} is not named");
    }
}

#[test]
fn accepts_a_fn_the_impl_actually_calls() {
    let f = unwired(
        "pub fn helper(x: u8) -> bool { true }\nfn go() { helper(2); }\n",
        "assert!(helper(1));",
        &["helper".into()],
    );
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn a_generic_declaration_is_still_a_declaration() {
    // `pub fn f<'a>(` does not contain `f(`, which made a CALLED function
    // read as uncalled (B2). My earlier test used a non-generic fn, so it
    // never touched this branch.
    let crate_src =
        "pub fn helper<'a>(x: &'a str) -> bool { true }\nlet v = helper(s);\n";
    assert!(
        unwired(crate_src, "assert!(helper(1));", &["helper".into()])
            .is_empty()
    );
}

#[test]
fn flags_a_generic_fn_nothing_calls() {
    let crate_src = "pub fn helper<'a>(x: &'a str) -> bool { true }\n";
    assert_eq!(
        unwired(crate_src, "assert!(helper(1));", &["helper".into()]).len(),
        1
    );
}

#[test]
fn accepts_a_fn_called_from_a_sibling_node() {
    // find_exhaustive_violations lives in fed and is called from cli.
    // Checking only the declaring module called that unwired (B1).
    let crate_src = "pub fn helper(x: u8) -> bool { true }\n\
                         // ... src/cli/mod.rs ...\nlet v = helper(3);\n";
    assert!(
        unwired(crate_src, "assert!(helper(1));", &["helper".into()])
            .is_empty()
    );
}

/// A DETECTOR: something that can find nothing.
const DETECTOR: &str = "pub fn detect(e: &[u8]) -> Vec<String> { vec![] }";

#[test]
fn a_new_pub_fn_without_a_doc_is_flagged() {
    // T13's first merit win shipped `post_with_retry` with no doc, and
    // `signatures` would have put a bare signature in every later
    // prompt for that node (`src/code:B4`).
    let f = undocumented(
        "pub fn post_with_retry(t: &T) -> u8 { 1 }",
        &["post_with_retry".into()],
    );
    assert_eq!(f.len(), 1, "{f:?}");
}

#[test]
fn a_documented_one_is_not() {
    let f = undocumented(
        "/// Retries a failing post.\npub fn post_with_retry() -> u8 { 1 }",
        &["post_with_retry".into()],
    );
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn an_attribute_between_the_doc_and_the_fn_still_counts_as_documented() {
    // `#[must_use]` sits between them all over this crate, so reading
    // only the immediately preceding line would flag the house style.
    let f = undocumented(
        "/// Retries.\n#[must_use]\npub fn go() -> u8 { 1 }",
        &["go".into()],
    );
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn a_fn_the_commit_did_not_add_is_not_judged() {
    // Scoped to NEW functions, like every rule here: 69 undocumented
    // public items already exist and are a separate debt.
    let f = undocumented("pub fn old() -> u8 { 1 }", &["new_one".into()]);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn flags_a_detector_tested_only_on_empty() {
    let f = negative_only(
        DETECTOR,
        "let c = detect(&e); assert!(c.is_empty());",
        &["detect".into()],
    );
    assert_eq!(f.len(), 1, "detect_cycles passed exactly like this");
}

#[test]
fn accepts_a_detector_with_a_positive_case() {
    let f = negative_only(
        DETECTOR,
        "let c = detect(&e); assert!(!c.is_empty());",
        &["detect".into()],
    );
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn a_scalar_returning_fn_is_not_a_detector() {
    // V1 says the subject is a DETECTOR, and `src/fed:B6` is
    // `detect_cycles -> Vec::new()`. "Found nothing" is only a failure
    // mode where nothing is expressible. Applied to every new `pub fn`,
    // the rule flagged `double(n) -> u8` and every other scalar the loop
    // writes -- and V23 made that FATAL, so no scalar could ever land
    // (B6 here).
    let f = negative_only(
        "pub fn double(n: u8) -> u8 { n * 2 }",
        "assert_eq!(double(2), 4);",
        &["double".into()],
    );
    assert!(f.is_empty(), "a scalar cannot 'find nothing': {f:?}");
}

#[test]
fn an_option_returning_fn_is_still_a_detector() {
    // `Option` is the other shape where absence is the answer, so the
    // narrowing must not let a real stub through.
    let f = negative_only(
        "pub fn find(k: &str) -> Option<u8> { None }",
        "assert!(find(\"x\").is_none());",
        &["find".into()],
    );
    assert_eq!(f.len(), 1, "None-only is the same defect: {f:?}");
}

#[test]
fn flags_a_new_fn_that_ignores_an_input() {
    let src =
        "pub fn hint(root: &Path, _budget: u64) -> Vec<PathBuf> { vec![] }\n";
    let f = ignored_input(src, &["hint".into()]);
    assert_eq!(
        f.len(),
        1,
        "check_split_hint ignored _budget exactly like this"
    );
    assert_eq!(f.first().map(|x| x.rule), Some("ignored-input"));
}

#[test]
fn accepts_a_fn_that_uses_every_input() {
    let src =
        "pub fn hint(root: &Path, budget: u64) -> Vec<PathBuf> { vec![] }\n";
    assert!(ignored_input(src, &["hint".into()]).is_empty());
}
