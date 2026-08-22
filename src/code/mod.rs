//! Reading Rust source AS TEXT. The mirror of `src/spec`, which reads
//! `SPEC.md` as text -- `§G` pairs them, and now the code does too.
//!
//! Promoted because the concern was implemented TWICE (`.:B13`): `src/tdd`
//! had `signatures`, `expected_calls` and `split_module`; `src/review` had
//! `public_fns` and the call-detection inside `unwired`. Both line-oriented
//! heuristics over the same text, both already carrying `§B` rows for reading
//! it wrong. No per-file gate could see it, because the duplication spanned
//! two files (`.:V109`).
//!
//! A SIBLING at depth 2, not a child of either owner: it deepens no chain,
//! where a child would bill every descendant every turn (`.:V110`).
//!
//! Everything here is a pure function over `&str`. No IO, no model, no
//! subprocess -- which is what makes it testable without either.

/// Split a Rust source file at the `#[cfg(test)]` boundary.
///
/// ONE definition. The code ceiling (`.:V50`) needs exactly this split, and
/// two readings of one rule is the defect this project exists to end.
///
/// Matches at column 0 only, so the marker inside a string literal -- a test
/// fixture carrying `"#[cfg(test)]\nmod t {"` -- does not split the file.
#[must_use]
pub fn split_module(src: &str) -> (&str, &str) {
    match src.find("\n#[cfg(test)]") {
        Some(i) => (&src[..i + 1], &src[i + 1..]),
        None => (src, ""),
    }
}

/// The `pub fn` names a source declares.
#[must_use]
pub fn public_fns(src: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| {
            let t = l.trim().strip_prefix("pub fn ")?;
            Some(t.split(['(', '<']).next()?.trim().to_string())
        })
        .collect()
}

/// Is `name` CALLED anywhere in `src`, outside its own declaration?
///
/// Extracted from `src/review::unwired`, which mixed this question with the
/// judgement it fed. Two details are load-bearing and were bought with bugs:
///
/// - Search the WHOLE crate, not the declaring module: a `pub fn` called from
///   a sibling node is wired (`src/review:B1`).
/// - A generic declaration `pub fn f<'a>(` does not contain `f(`, so counting
///   occurrences and assuming "declaration plus one" read a called function
///   as uncalled (`src/review:B2`).
///
/// Skips only THIS function's declaration, not every line starting with `fn`,
/// since a one-line body declares and calls on the same line.
#[must_use]
pub fn is_called(src: &str, name: &str) -> bool {
    let declares = |l: &str| {
        l.contains(&format!("fn {name}(")) || l.contains(&format!("fn {name}<"))
    };
    src.lines()
        .any(|l| l.contains(&format!("{name}(")) && !declares(l))
}

/// Calls a test makes that do not exist yet -- the contract step 2 must fill.
///
/// Deterministic parse, no model (`.:V18`). A run failed when the test called
/// `check_edge_depths(root, &edges)` and step 2 invented a different name, which
/// three repairs could not recover (B12): step 2 was never told what to define.
#[must_use]
pub fn expected_calls(test_src: &str, existing: &str) -> Vec<String> {
    const SKIP: [&str; 18] = [
        "fn",
        "if",
        "for",
        "while",
        "match",
        "let",
        "return",
        "assert",
        "assert_eq",
        "assert_ne",
        "panic",
        "println",
        "format",
        "vec",
        "write",
        "read",
        "Some",
        "Ok",
    ];
    let b = test_src.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !(b[i].is_ascii_alphabetic() || b[i] == b'_') {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
            i += 1
        }
        let name = &test_src[start..i];
        // a call is `name(`; a macro is `name!(`; a method is `.name(`
        if i >= b.len() || b[i] != b'(' {
            continue;
        }
        if start > 0 && (b[start - 1] == b'.' || b[start - 1] == b'!') {
            continue;
        }
        // `fn name(` is a DEFINITION, not a call -- including the test's own
        let mut k = start;
        while k > 0 && (b[k - 1] == b' ' || b[k - 1] == b'\t') {
            k -= 1
        }
        if k >= 2 && &test_src[k - 2..k] == "fn" {
            continue;
        }
        if SKIP.contains(&name) || existing.contains(&format!("fn {name}")) {
            continue;
        }
        // keep the call verbatim, arguments included -- the signature is the point
        let mut depth = 0usize;
        let mut j = i;
        while j < b.len() {
            if b[j] == b'(' {
                depth += 1
            } else if b[j] == b')' {
                depth -= 1;
                if depth == 0 {
                    break;
                }
            }
            j += 1;
        }
        let call =
            test_src[start..(j + 1).min(test_src.len())].replace('\n', " ");
        let call = call.split_whitespace().collect::<Vec<_>>().join(" ");
        if !out.contains(&call) {
            out.push(call)
        }
    }
    out
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
            if depth > 0 {
                out.push_str(line);
                out.push('\n');
            } else {
                pending.push(line);
            }
            continue;
        }
        let is_sig = s.starts_with("pub fn")
            || s.starts_with("pub struct")
            || s.starts_with("pub enum")
            || s.starts_with("pub const");
        if depth > 0 {
            // inside a type body: keep field lines, they are part of the shape
            if s == "}" {
                depth = 0;
                out.push_str("}\n");
            } else if !s.is_empty() {
                out.push_str(line);
                out.push('\n');
            }
            continue;
        }
        if !is_sig {
            pending.clear();
        }
        if is_sig {
            for d in pending.drain(..) {
                out.push_str(d);
                out.push('\n');
            }
            if s.starts_with("pub fn") {
                let sig = s.split('{').next().unwrap_or(s).trim_end();
                out.push_str(sig);
                out.push_str(" { /* ... */ }\n");
            } else {
                out.push_str(line);
                out.push('\n');
                if s.ends_with('{') {
                    depth = 1;
                }
            }
        }
    }
    out
}

/// The names a TEST module declares, one per line.
///
/// Not [`signatures`], and deliberately a different shape. `signatures`
/// answers "what API may a writer call", so it carries doc comments and
/// field lines. This answers "what names are in scope for a judge reading a
/// test", where the field bodies are noise the prompt pays for by the token
/// (R17: prefill cost is superlinear).
///
/// It also does not require `pub`: a test module's doubles are private to it
/// -- `struct Flaky` is never `pub` -- so `signatures` extracts NOTHING from
/// a test half, and the judge was told to check names against a data model
/// that could not contain them (`.:tdd:B27`).
#[must_use]
pub fn test_decls(tests_src: &str) -> String {
    let mut out = String::new();
    for line in tests_src.lines() {
        let s = line.trim().trim_start_matches("pub ");
        let keep = s.starts_with("fn ")
            || s.starts_with("struct ")
            || s.starts_with("enum ")
            || s.starts_with("const ")
            || s.starts_with("impl ");
        if keep {
            let head = s.split('{').next().unwrap_or(s).trim_end();
            out.push_str(head);
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test module's doubles are PRIVATE to it, so `signatures` -- which
    /// requires `pub` -- extracts nothing from a test half. The judge was
    /// shown only the impl half and told to check names against it, so an
    /// authored test reusing an existing double was rejected for referring
    /// to something that "does not appear" (`.:tdd:B27`).
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

    #[test]
    fn public_fns_reads_declarations_only() {
        let v =
            public_fns("pub fn a() {}\nfn b() {}\n    pub fn c<T>(x: T) {}\n");
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
}
