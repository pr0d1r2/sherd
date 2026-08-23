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

/// Names that open a paren but are never the function a test is driving:
/// control flow, the assertion family, and the macros every test uses.
const NOT_A_CALL: [&str; 18] = [
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

/// The next identifier at or after `from`, as `(start, end)`.
fn next_ident(b: &[u8], from: usize) -> Option<(usize, usize)> {
    let mut i = from;
    while i < b.len() && !(b[i].is_ascii_alphabetic() || b[i] == b'_') {
        i = i.saturating_add(1);
    }
    if i >= b.len() {
        return None;
    }
    let start = i;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
        i = i.saturating_add(1);
    }
    Some((start, i))
}

/// Is the identifier ending at `end` a CALL of a free function?
///
/// A call is `name(`; a method is `.name(` and a macro is `name!(`, neither of
/// which is a function this test expects to be written. `fn name(` is a
/// DEFINITION -- including the test's own.
fn opens_call(src: &str, b: &[u8], start: usize, end: usize) -> bool {
    if b.get(end) != Some(&b'(') {
        return false;
    }
    if start > 0 && matches!(b.get(start.wrapping_sub(1)), Some(&b'.' | &b'!'))
    {
        return false;
    }
    let mut k = start;
    while k > 0 && matches!(b.get(k.wrapping_sub(1)), Some(&b' ' | &b'\t')) {
        k = k.saturating_sub(1);
    }
    k < 2 || src.get(k.saturating_sub(2)..k) != Some("fn")
}

/// The `)` matching the `(` at `open`, or the end of input.
fn closing_paren(b: &[u8], open: usize) -> usize {
    let mut depth = 0usize;
    for (j, c) in b.iter().enumerate().skip(open) {
        if *c == b'(' {
            depth = depth.saturating_add(1);
        } else if *c == b')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return j;
            }
        }
    }
    b.len()
}

/// The call verbatim, arguments included, whitespace collapsed.
///
/// The arguments are the point: step 2 is told to define EXACTLY this name and
/// signature, so a call with its arguments is the contract.
fn call_text(src: &str, b: &[u8], start: usize, open: usize) -> String {
    let end = closing_paren(b, open).saturating_add(1).min(src.len());
    src.get(start..end)
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Calls a test makes that do not exist yet -- the contract step 2 must fill.
///
/// Deterministic parse, no model (`.:V18`). A run failed when the test called
/// `check_edge_depths(root, &edges)` and step 2 invented a different name,
/// which three repairs could not recover (B12): step 2 was never told what to
/// define.
#[must_use]
pub fn expected_calls(test_src: &str, existing: &str) -> Vec<String> {
    let b = test_src.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while let Some((start, end)) = next_ident(b, i) {
        i = end;
        let Some(name) = test_src.get(start..end) else {
            continue;
        };
        if !opens_call(test_src, b, start, end)
            || NOT_A_CALL.contains(&name)
            || existing.contains(&format!("fn {name}"))
        {
            continue;
        }
        let call = call_text(test_src, b, start, end);
        if !out.contains(&call) {
            out.push(call);
        }
    }
    out
}

/// Does this line open a public item worth showing?
fn is_signature(s: &str) -> bool {
    ["pub fn", "pub struct", "pub enum", "pub const"]
        .iter()
        .any(|k| s.starts_with(k))
}

/// The public surface, accumulated one line at a time.
///
/// `in_body` is a BOOL rather than a depth counter: the old code carried a
/// `usize` that only ever held 0 or 1, because a type body is entered from top
/// level and left at the first unindented `}`. A counter that cannot count
/// invites a reader to look for the nesting it implies.
#[derive(Default)]
struct Surface<'a> {
    out: String,
    /// Inside a type body, where every field line is part of the shape.
    in_body: bool,
    /// Doc lines seen at top level, belonging to the item still to come.
    pending: Vec<&'a str>,
}

impl<'a> Surface<'a> {
    fn line(&mut self, line: &'a str) {
        let s = line.trim();
        if s.starts_with("///") {
            self.doc(line);
        } else if self.in_body {
            self.body(line, s);
        } else if is_signature(s) {
            self.signature(line, s);
        } else {
            self.pending.clear();
        }
    }

    /// Doc comments ARE the semantics. Bare field names cannot tell a judge
    /// whether `not_owns` holds a path or prose, and that is precisely the
    /// question it has to answer (B4).
    ///
    /// Inside a type body a doc belongs to the FIELD below it, so it is
    /// emitted in place; at top level it belongs to the item still to come.
    fn doc(&mut self, line: &'a str) {
        if self.in_body {
            self.emit(line);
        } else {
            self.pending.push(line);
        }
    }

    fn body(&mut self, line: &str, s: &str) {
        if s == "}" {
            self.in_body = false;
            self.out.push_str("}\n");
        } else if !s.is_empty() {
            self.emit(line);
        }
    }

    /// A `pub fn` keeps its signature and drops its body; a type keeps the
    /// whole declaration, because its fields are the shape.
    fn signature(&mut self, line: &str, s: &str) {
        for d in std::mem::take(&mut self.pending) {
            self.out.push_str(d);
            self.out.push('\n');
        }
        if s.starts_with("pub fn") {
            self.out
                .push_str(s.split('{').next().unwrap_or(s).trim_end());
            self.out.push_str(" { /* ... */ }\n");
            return;
        }
        self.emit(line);
        self.in_body = s.ends_with('{');
    }

    fn emit(&mut self, line: &str) {
        self.out.push_str(line);
        self.out.push('\n');
    }
}

/// The public SURFACE of an implementation: signatures and type shapes, no
/// bodies. Step 1 needs this and must not have the bodies -- it is `§I`, not
/// `§V`. Written after a run where the test author, given only the spec, could
/// not see `Edge`'s fields and reached for the wrong one (B1 here).
#[must_use]
pub fn signatures(impl_src: &str) -> String {
    let mut s = Surface::default();
    for line in impl_src.lines() {
        s.line(line);
    }
    s.out
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
/// that could not contain them (`.:src/tdd:B27`).
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

/// One `mod` declaration, and whether the author published it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModDecl {
    pub name: String,
    /// `pub mod` -- the author's own statement that this is API, and the
    /// second-strongest evidence of a federation boundary (`.:src/plan:V17`).
    pub is_pub: bool,
}

/// Every `mod` a file declares.
///
/// Line-oriented and deliberately so: this reads Rust AS TEXT, like every
/// other function here. A `mod foo;` inside a comment or a string would be
/// counted, and that is the trade the whole node makes -- a parser is a
/// dependency and a second reading of the language.
///
/// `mod foo { .. }` (an inline module) is skipped: it declares no file, so
/// it can never become a directory node.
/// A `#[cfg(test)]` module is skipped: it does not ship, so it can never be
/// a federation node. `testrepo` in this crate is exactly that shape, and it
/// was proposed as one until this line existed.
#[must_use]
pub fn mod_decls(src: &str) -> Vec<ModDecl> {
    let mut out = Vec::new();
    let mut test_only = false;
    for line in src.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if line.starts_with("#[cfg(test)]") {
            test_only = true;
            continue;
        }
        if let Some(decl) = one_decl(line)
            && !test_only
        {
            out.push(decl);
        }
        test_only = false;
    }
    out
}

/// One `mod foo;` or `pub mod foo;`, or nothing.
/// `pub(crate)` and `pub(super)` are NOT published: they are visibility
/// inside the crate, which is the same information a bare `mod` carries.
/// `microlith` declares all eleven of its modules `pub(crate)`, so reading
/// that as `pub` would call an internal boundary an API one -- and failing
/// to parse it at all made a crate with eleven modules propose nothing
/// (`.:src/plan:B13`).
fn one_decl(line: &str) -> Option<ModDecl> {
    let (is_pub, rest) = match line.strip_prefix("pub(") {
        Some(r) => (false, r.split_once(") ").map(|(_, r)| r)?),
        None => line
            .strip_prefix("pub ")
            .map_or((false, line), |r| (true, r)),
    };
    let name = rest.strip_prefix("mod ")?.strip_suffix(';')?;
    (!name.contains(char::is_whitespace)).then(|| ModDecl {
        name: name.to_string(),
        is_pub,
    })
}

/// The crate-internal modules a file reaches for: `use crate::X`.
///
/// COHESION evidence. A module every member of a family reaches for is a hub
/// the family shares, which is what tells eleven `*cmd` files apart from
/// eleven independent concerns (`.:src/plan:V17`).
#[must_use]
pub fn crate_uses(src: &str) -> Vec<String> {
    let mut out: Vec<String> = src
        .lines()
        .map(str::trim)
        .filter_map(|l| {
            let rest = l.strip_prefix("use crate::")?;
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty()).then_some(name)
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod structure_tests {
    use super::*;

    const SRC: &str = "\
#[cfg(test)]
mod testonly;
pub(crate) mod internal;
mod args;
pub mod bpe;
mod capcmd;
pub mod cli;
mod inline { fn x() {} }
use crate::render::Line;
use crate::units;
use std::path::Path;
";

    /// `pub(crate)` is visibility INSIDE the crate, which is what a bare
    /// `mod` already says -- not the published API `pub` declares.
    #[test]
    fn pub_and_pub_crate_and_private_are_three_different_things() {
        let mods = mod_decls(SRC);
        assert_eq!(mods.len(), 5, "{mods:?}");
        assert!(mods.iter().any(|m| m.name == "bpe" && m.is_pub));
        assert!(mods.iter().any(|m| m.name == "args" && !m.is_pub));
        assert!(
            mods.iter().any(|m| m.name == "internal" && !m.is_pub),
            "pub(crate) is not published: {mods:?}"
        );
    }

    /// An inline `mod foo { .. }` declares no FILE, so it can never become a
    /// directory node and is not a candidate. A `#[cfg(test)]` module does
    /// not ship, so it is not one either.
    #[test]
    fn an_inline_or_test_only_module_is_not_a_candidate() {
        let mods = mod_decls(SRC);
        assert!(!mods.iter().any(|m| m.name == "inline"));
        assert!(!mods.iter().any(|m| m.name == "testonly"));
    }

    #[test]
    fn crate_uses_names_internal_modules_and_ignores_the_rest() {
        assert_eq!(crate_uses(SRC), vec!["render", "units"]);
    }

    #[test]
    fn a_file_reaching_for_nothing_internal_yields_nothing() {
        assert!(crate_uses("use std::path::Path;\nfn main() {}\n").is_empty());
    }
}
