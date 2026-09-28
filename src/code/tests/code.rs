use super::*;

/// A test module's doubles are PRIVATE to it, so `signatures` -- which
/// requires `pub` -- extracts nothing from a test half. The judge was
/// shown only the impl half and told to check names against it, so an
/// authored test reusing an existing double was rejected for referring
/// to something that "does not appear" (`.:src/tdd:B27`).
const TEST_HALF: &str = "#[cfg(test)]\nmod tests {\n    use super::*;\n\n    \
         struct Flaky {\n        fail_times: Cell<u32>,\n    }\n\n    \
         impl Transport for Flaky {\n        fn post(&self) -> u8 { 1 }\n    }\n\n    \
         fn slow_eta() -> Eta {\n        Eta::default()\n    }\n\n    \
         #[test]\n    fn a_case() {\n        assert!(true);\n    }\n}\n";

#[test]
fn a_private_double_is_invisible_to_signatures_and_visible_to_test_decls() {
    assert_eq!(
        signatures(TEST_HALF).lines().count(),
        0,
        "nothing in a test module is `pub`, so the API surface is empty"
    );
    let d = test_decls(TEST_HALF);
    assert!(
        d.contains("struct Flaky"),
        "the double is a NAME in scope: {d}"
    );
    assert!(d.contains("fn slow_eta"), "so is its helper: {d}");
    assert!(d.contains("impl Transport for Flaky"), "and the impl: {d}");
}

#[test]
fn test_decls_carries_names_not_bodies() {
    // The judge pays for this prompt by the token and R17 says prefill
    // cost is superlinear, so it gets what names EXIST and not every
    // field. Measured on `src/ollama`: 42 lines rather than 386.
    let d = test_decls(TEST_HALF);
    assert!(!d.contains("fail_times"), "field lines are not names: {d}");
    assert!(!d.contains("assert!"), "bodies are not names: {d}");
    assert!(!d.contains('{'), "declarations are truncated at the brace");
}

/// `V50`'s limit reached these two, and `T3` asked for the seams. The
/// pieces are named here so a reader sees the parse as four questions
/// rather than one 75-line scan.
#[test]
fn a_call_is_a_free_function_not_a_method_or_a_macro() {
    let src = "fn t() { helper(1); x.method(2); vec![3]; assert!(y); }";
    let b = src.as_bytes();
    let at = |n: &str| src.find(n).unwrap_or_default();
    assert!(
        opens_call(src, b, at("helper"), at("helper") + 6),
        "a bare name followed by `(` is a call"
    );
    assert!(
        !opens_call(src, b, at("method"), at("method") + 6),
        "`.method(` is a method, not a function to define"
    );
    // `fn t(` is the test's own definition, never a call it makes.
    assert!(!opens_call(src, b, at("t()"), at("t()") + 1));
}

/// A brace inside a CHAR LITERAL is not a brace. `s.split('{')` is
/// ordinary code here, and counting it made one body swallow every
/// function after it -- found by probing the duplication check against
/// this crate before writing a test for it.
#[test]
fn a_brace_in_a_char_literal_does_not_open_a_block() {
    let src = "fn first(s: &str) -> &str {\n    \
                   s.split('{').next().unwrap_or(s)\n}\n\
                   fn second() -> u8 {\n    7\n}\n";
    let b = fn_body(src, "first").unwrap_or_default();
    assert!(b.contains("split"), "{b}");
    assert!(
        !b.contains("second"),
        "the body stops at its own brace: {b}"
    );
}

/// A LIFETIME is not a char literal. `<'a>` opens a quote that never
/// closes, and treating it as one made every generic function's body run
/// to the end of the file -- 68 false pairs against 7 real ones.
#[test]
fn a_lifetime_does_not_open_a_literal() {
    let src = "fn first<'a>(s: &'a str) -> &'a str {\n    s\n}\n\
                   fn second() -> u8 {\n    7\n}\n";
    let b = fn_body(src, "first").unwrap_or_default();
    assert!(!b.contains("second"), "the body stops at its brace: {b}");
    assert_eq!(fn_body(src, "absent"), None);
}

/// `fn_names` sees every declaration form this crate uses, because the
/// duplication check compares a new function against ALL of them and one
/// it cannot name is one it cannot compare.
#[test]
fn every_declaration_form_in_this_crate_is_named() {
    let src = "pub fn a() {}\nfn b() {}\npub(crate) fn c() {}\n    \
                   fn d() {}\npub fn e<'x>(v: &'x str) {}\nstruct S;\n";
    assert_eq!(fn_names(src), vec!["a", "b", "c", "d", "e"]);
    assert!(fn_names("struct S;\nlet x = 1;\n").is_empty());
}

/// An escaped quote is not the end of a literal, and a marker shorter
/// than three characters carries no signal about WHAT is parsed --
/// `" "` and `"("` appear in every parser here.
#[test]
fn markers_are_the_long_literals_a_body_matches_on() {
    let body = "{ s.contains(\"## \u{a7}T\") && s.contains(\"a\") \
                    && s.starts_with(\"say \\\"hi\\\"\") }";
    let m = markers(body);
    assert!(m.contains(&"## \u{a7}T".to_string()), "{m:?}");
    assert!(!m.contains(&"a".to_string()), "too short to mean anything");
    assert!(!m.is_empty());
}

/// Nested parens are why this cannot be a `find(')')`: the contract is the
/// call WITH its arguments, and an argument may itself be a call.
#[test]
fn a_call_keeps_its_arguments_including_nested_ones() {
    let src = "fn t() { check(edges(root), 3); }";
    let calls = expected_calls(src, "");
    assert!(
        calls.contains(&"check(edges(root), 3)".to_string()),
        "the outer call keeps the inner one: {calls:?}"
    );
}

/// An unbalanced call runs to the end rather than panicking or looping:
/// the input is a model's output and may be truncated mid-call.
#[test]
fn an_unclosed_call_ends_at_the_input_rather_than_panicking() {
    let src = "fn t() { truncated(1, 2";
    assert_eq!(closing_paren(src.as_bytes(), src.len()), src.len());
    let calls = expected_calls(src, "");
    assert_eq!(calls, vec!["truncated(1, 2".to_string()]);
}

/// A type body keeps its fields AND their docs -- the doc is what tells a
/// judge whether a field holds a path or prose (B4).
#[test]
fn a_type_keeps_its_fields_and_a_fn_keeps_only_its_line() {
    let src = "/// what it is\npub struct E {\n    /// a path\n    pub dir: String,\n}\n\
                   /// what it does\npub fn go(n: u64) -> bool {\n    n > 0\n}\n";
    let s = signatures(src);
    assert!(s.contains("/// a path"), "field docs survive: {s}");
    assert!(s.contains("pub dir: String,"), "fields survive: {s}");
    assert!(s.contains("pub fn go(n: u64) -> bool { /* ... */ }"), "{s}");
    assert!(!s.contains("n > 0"), "a fn body does not: {s}");
}

const SRC: &str = "pub fn a() {}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() {}\n}\n";

#[test]
fn split_finds_the_test_boundary() {
    let (i, t) = split_module(SRC);
    assert!(i.contains("pub fn a"), "impl half: {i}");
    assert!(t.contains("#[cfg(test)]"), "test half: {t}");
    assert!(!i.contains("#[cfg(test)]"), "boundary leaked: {i}");
}

#[test]
fn split_of_a_file_with_no_tests_is_all_impl() {
    let (i, t) = split_module("pub fn a() {}\n");
    assert_eq!(t, "", "no test module means an empty test half");
    assert!(i.contains("pub fn a"));
}

#[test]
fn the_marker_inside_a_string_does_not_split_the_file() {
    // The corpora in `src/tdd` carry `"#[cfg(test)]\nmod t {"` inside
    // string literals. Matching at column 0 is what keeps a fixture from
    // cutting the file in half.
    let src =
        "pub const F: &str = \"#[cfg(test)]\\nmod t {}\";\npub fn a() {}\n";
    let (i, t) = split_module(src);
    assert_eq!(t, "", "a marker inside a literal is not a boundary");
    assert!(i.contains("pub fn a"));
}

/// `.:B29`, planted. 337 lines of scheduler sat below `src/plan`'s first
/// test module and were weighed against the TEST ceiling for as long as
/// they existed -- so when they moved to another node, the code number
/// `.:V50` reports did not change. The CUT still cuts at the first
/// marker, which is what `src/tdd` and `src/review` edit against; the
/// MEASURE sums every region, and this asserts both readings side by
/// side, since agreeing was the bug.
#[test]
fn code_below_a_test_module_is_code_to_the_measure_and_not_to_the_cut() {
    let src = "pub fn above() {}\n\
                   #[cfg(test)]\n\
                   mod t {\n    #[test]\n    fn x() {}\n}\n\
                   pub fn below() {}\n";

    let (code, tests) = split_regions(src);
    assert!(code.contains("pub fn above"), "{code}");
    assert!(code.contains("pub fn below"), "the whole finding: {code}");
    assert!(!code.contains("#[cfg(test)]"), "{code}");
    assert!(tests.contains("fn x"), "{tests}");
    assert!(!tests.contains("pub fn below"), "{tests}");

    // The cut is UNCHANGED, and that is deliberate: it answers where the
    // test region starts, not how much of the file is code.
    let (_, tail) = split_module(src);
    assert!(tail.contains("pub fn below"), "the cut still cuts: {tail}");
}

/// The attribute applies to an ITEM, and the item is not always a module:
/// `src/cli` carries a `#[cfg(test)] fn repo_root()`. The old cut read
/// every line after it as tests; the measure closes the region at the
/// item's own `}` and keeps reading code afterwards.
#[test]
fn a_cfg_test_function_closes_its_own_region() {
    let src = "pub fn a() {}\n\
                   #[cfg(test)]\n\
                   fn helper() -> u8 {\n    1\n}\n\
                   pub fn b() {}\n";
    let (code, tests) = split_regions(src);
    assert!(
        code.contains("pub fn a") && code.contains("pub fn b"),
        "{code}"
    );
    assert!(tests.contains("fn helper"), "{tests}");
    assert!(!tests.contains("pub fn b"), "{tests}");
}

/// A body-less item -- `#[cfg(test)] #[path = "tests/x.rs"] mod x;`,
/// the form `.:V124` puts a suite in -- has no `}` of its own. Waiting
/// for one closed the region at the NEXT item's brace, and every line of
/// production code in between was measured as tests.
#[test]
fn a_bodyless_test_module_closes_at_its_semicolon() {
    let src = "pub fn a() {}\n\
                   #[cfg(test)]\n\
                   #[path = \"tests/x.rs\"]\n\
                   mod x;\n\
                   pub fn b() {\n    1\n}\n";
    let (code, tests) = split_regions(src);
    assert!(
        code.contains("pub fn b"),
        "production code after the declaration: {code}"
    );
    assert!(tests.contains("mod x;"), "{tests}");
}

/// V1 holds for the measure as it does for the cut: a marker indented or
/// inside a string literal is content. `src/tdd`'s corpora carry one.
#[test]
fn the_measure_matches_at_column_zero_only() {
    let src =
        "pub const F: &str = \"#[cfg(test)]\\nmod t {}\";\npub fn a() {}\n";
    let (code, tests) = split_regions(src);
    assert_eq!(tests, "", "a marker inside a literal opens no region");
    assert!(code.contains("pub fn a"));

    // And a file with no tests at all is all code, both readings.
    let (code, tests) = split_regions("pub fn a() {}\n");
    assert_eq!(tests, "");
    assert_eq!(code, "pub fn a() {}\n");
}

#[test]
fn public_fns_reads_declarations_only() {
    let v = public_fns("pub fn a() {}\nfn b() {}\n    pub fn c<T>(x: T) {}\n");
    assert!(v.contains(&"a".to_string()), "{v:?}");
    assert!(
        !v.contains(&"b".to_string()),
        "private is not public: {v:?}"
    );
    assert!(v.contains(&"c".to_string()), "generic counts: {v:?}");
}

#[test]
fn is_called_sees_a_generic_declaration_as_a_declaration() {
    // src/review:B2 -- `pub fn f<'a>(` does not contain `f(`, so counting
    // occurrences read a called function as uncalled.
    let src = "pub fn f<'a>(x: &'a str) {}\n";
    assert!(!is_called(src, "f"), "its own declaration is not a call");
    let with_call = format!("{src}fn g() {{ f(\"x\"); }}\n");
    assert!(is_called(&with_call, "f"), "a real call is a call");
}

#[test]
fn is_called_accepts_a_one_line_body_that_declares_and_calls() {
    // Skipping every line starting with `fn` would miss this.
    let src = "pub fn f() {}\npub fn g() { f(); }\n";
    assert!(is_called(src, "f"));
}

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
    assert!(
        expected_calls(t, "").is_empty(),
        "{:?}",
        expected_calls(t, "")
    );
}

#[test]
fn signatures_keep_shape_and_drop_bodies() {
    let src = "/// what it owns\npub struct E {\n    /// a path\n    pub dir: String,\n}\n\n/// does the thing\npub fn go(a: u8) -> bool {\n    secret();\n    true\n}\n";
    let s = signatures(src);
    assert!(s.contains("/// a path"), "doc comments ARE semantics: {s}");
    assert!(s.contains("/// does the thing"), "fn docs survive: {s}");
    assert!(s.contains("pub fn go(a: u8) -> bool"), "signature: {s}");
    assert!(!s.contains("secret()"), "body must not leak: {s}");
}
