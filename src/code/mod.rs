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

/// The CUT POINT: where the first `#[cfg(test)]` region begins.
///
/// One definition of the cut, and two callers depend on it being a cut rather
/// than a measure: `src/tdd` writes a generated test region back over the
/// tail, and `src/review` reads what a commit added on each side of it. Both
/// need one position in the file, not a total.
///
/// What it is NOT is a measurement of how much of a file is code. Everything
/// after the first marker is the second half, production code included, and
/// [`split_regions`] is the function that answers that question instead
/// (`.:B29`).
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

/// The MEASURE: every non-test region of a file, and every test region.
///
/// A file may hold production code BELOW a test module, and the cut above
/// reads all of it as tests. `.:B29` is what that cost -- 337 lines of
/// scheduler sat below `src/plan`'s first test module and were weighed
/// against the TEST ceiling for as long as they existed, so the code number
/// `.:V50` reports did not move when they left the file.
///
/// A region opens on `#[cfg(test)]` at column 0 and closes on the next `}` at
/// column 0: the item the attribute applies to, whether that is a `mod` or a
/// single `fn` (`src/cli` has one of each). Column 0 for the same reason
/// `split_module` uses it, V1 -- a marker indented or inside a string is
/// content, not structure.
///
/// Returns owned strings because the regions are not contiguous. That is the
/// whole difference from the cut, and it is why the two cannot be one
/// function.
#[must_use]
pub fn split_regions(src: &str) -> (String, String) {
    let mut code = String::new();
    let mut tests = String::new();
    let mut in_tests = false;
    for line in src.lines() {
        if !in_tests && line.starts_with("#[cfg(test)]") {
            in_tests = true;
        }
        let half = if in_tests { &mut tests } else { &mut code };
        half.push_str(line);
        half.push('\n');
        // The close of the attributed item. Anything nested is indented, so
        // this is the end of the region rather than of a block inside it.
        if in_tests && line == "}" {
            in_tests = false;
        }
    }
    (code, tests)
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

/// Bindings a body indexes by a LITERAL, with how many times.
///
/// `xs[0]` and `xs[1]` are two; `xs[i]` is none, because a computed index
/// says nothing about the arity the author expects. Here rather than in
/// `src/review`, because reading Rust as text is this node's job and a second
/// scanner over there is `.:B13` exactly.
#[must_use]
pub fn literal_indexes(body: &str) -> Vec<(String, usize)> {
    let b = body.as_bytes();
    let mut seen: Vec<(String, usize)> = Vec::new();
    for (i, c) in b.iter().enumerate() {
        if *c != b'[' {
            continue;
        }
        let Some(name) = indexed_binding(body, b, i) else {
            continue;
        };
        match seen.iter_mut().find(|(n, _)| n == &name) {
            Some((_, n)) => *n = n.saturating_add(1),
            None => seen.push((name, 1)),
        }
    }
    seen
}

/// The binding immediately before a `[` that holds only digits.
fn indexed_binding(body: &str, b: &[u8], at: usize) -> Option<String> {
    let digits: String = body
        .get(at.saturating_add(1)..)?
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty()
        || b.get(at.saturating_add(1).saturating_add(digits.len()))
            != Some(&b']')
    {
        return None;
    }
    let name: String = body
        .get(..at)?
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Every function declared in a source, in order.
///
/// Here rather than in `src/review`, because `.:B13` is precisely the defect
/// of two nodes both reading Rust as text: the fix moved `public_fns` and
/// call-detection here, and a second name scanner over there would undo it.
#[must_use]
pub fn fn_names(src: &str) -> Vec<String> {
    src.lines()
        .filter_map(|l| {
            let s = l.trim_start();
            let r = s
                .strip_prefix("pub fn ")
                .or_else(|| s.strip_prefix("pub(crate) fn "))
                .or_else(|| s.strip_prefix("fn "))?;
            let name: String = r
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            (!name.is_empty()).then_some(name)
        })
        .collect()
}

/// The body of one function, brace-matched, or `None` if it is not declared.
///
/// Brace-matched rather than indentation-matched: a `match` arm and a closure
/// both indent, and the end of a function is the only thing a counter can be
/// sure of.
///
/// Braces inside a CHAR or STRING literal do not count. `s.split('{')` is
/// ordinary code here, and counting its brace made one body swallow every
/// function after it -- caught by probing the check this exists to feed,
/// against this crate, before writing a test (`src/review:B3`).
#[must_use]
pub fn fn_body<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let decl = format!("fn {name}(");
    let at = src.find(&decl)?;
    let open = src.get(at..)?.find('{')?.saturating_add(at);
    let b = src.as_bytes();
    let (mut depth, mut i) = (0usize, open);
    while i < b.len() {
        match b.get(i) {
            Some(&b'"' | &b'\'') => {
                i = skip_literal(b, i);
                continue;
            }
            Some(&b'{') => depth = depth.saturating_add(1),
            Some(&b'}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return src.get(open..=i);
                }
            }
            _ => {}
        }
        i = i.saturating_add(1);
    }
    src.get(open..)
}

/// Index just past the literal opening at `at`, honouring `\` escapes.
///
/// A LIFETIME is not a char literal: `<'a>` opens a quote that never closes,
/// and treating it as one made `fn_body` swallow every function after the
/// first generic one. A char literal is `'x'` or `'\\n'` -- the quote closes
/// within three bytes -- and anything else beginning with `'` is a lifetime.
fn skip_literal(b: &[u8], at: usize) -> usize {
    let Some(&quote) = b.get(at) else {
        return at.saturating_add(1);
    };
    if quote == b'\'' {
        let escaped = b.get(at.saturating_add(1)) == Some(&b'\\');
        let closes = b.get(at.saturating_add(2)) == Some(&b'\'');
        if !escaped && !closes {
            return at.saturating_add(1); // a lifetime
        }
    }
    let mut i = at.saturating_add(1);
    while i < b.len() {
        match b.get(i) {
            Some(&b'\\') => i = i.saturating_add(2),
            Some(c) if *c == quote => return i.saturating_add(1),
            _ => i = i.saturating_add(1),
        }
    }
    i
}

/// The string literals a body MATCHES ON -- the markers it recognises.
///
/// Two functions that scan for the same markers are two readings of one
/// parse, which is `.:B13`: `split_module`, `signatures` and `expected_calls`
/// lived in `src/tdd` while `public_fns` and call-detection lived in
/// `src/review`, both reading Rust as text.
///
/// Literals shorter than three characters are dropped: `" "`, `"("` and `"\n"`
/// appear in nearly every parser here and carry no signal about WHAT is being
/// parsed.
#[must_use]
pub fn markers(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut rest = body;
    while let Some(i) = rest.find('"') {
        let Some(tail) = rest.get(i.saturating_add(1)..) else {
            break;
        };
        // A `\"` inside the literal is not its end.
        let Some(end) = tail
            .find('"')
            .filter(|e| !tail.get(..*e).is_some_and(|s| s.ends_with('\\')))
        else {
            break;
        };
        if let Some(lit) = tail.get(..end)
            && lit.chars().count() >= 3
            && !lit.contains("{}")
            && !out.iter().any(|o| o == lit)
        {
            out.push(lit.to_string());
        }
        rest = tail.get(end.saturating_add(1)..).unwrap_or_default();
    }
    out.sort();
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
        .filter_map(|l| l.strip_prefix("use crate::"))
        .flat_map(use_names)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The module names ONE `use crate::` line reaches for.
///
/// A BRACE GROUP names several at once -- `use crate::{fed, spec};` -- and
/// reading only the leading identifier returned NOTHING for it, because the
/// first character is `{`. MEASURED on this crate: four such lines, and
/// `src/cli`, which names eight siblings on one of them, read as reaching for
/// none (`B1`).
///
/// A nested group (`use crate::{a::{b, c}, d}`) is split on the commas like
/// any other, so an inner item can be named as if it were a module. Line
/// oriented, like everything here, and §C states that trade for the node.
fn use_names(rest: &str) -> Vec<String> {
    let Some(group) = rest.strip_prefix('{') else {
        return leading_ident(rest).into_iter().collect();
    };
    group
        .split_once('}')
        .map_or(group, |(inner, _)| inner)
        .split(',')
        .filter_map(|part| leading_ident(part.trim()))
        .collect()
}

/// One `use crate::` reach, with the ITEMS it names rather than only the
/// module it enters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// The first segment after `crate::` -- `lint` in `use crate::lint::Level`.
    pub module: String,
    /// The last segment of each path under that module, or EMPTY when the
    /// line imports the module itself (`use crate::lint;`). Empty is not
    /// "nothing named": it is the whole module, which is the strongest reach
    /// a line can make.
    pub items: Vec<String>,
}

/// Every `use crate::` line, read one level deeper than [`crate_uses`].
///
/// `crate_uses` answers WHICH sibling a file reaches for, which is all
/// cohesion evidence needs. Telling a TYPE reference apart from a call needs
/// the item as well: `use crate::lint::Level` names a type, and once a seam
/// commit has declared it, the importing node does not wait for `lint`'s
/// logic (`src/wave:V4`, `src/wave:B1`).
///
/// Line oriented, like everything here: a path is read as `module` plus the
/// LAST segment of each branch, so `crate::a::b::Item` is `a` naming `Item`.
/// §C states that trade for the node.
#[must_use]
pub fn crate_imports(src: &str) -> Vec<Import> {
    src.lines()
        .map(str::trim)
        .filter_map(|l| l.strip_prefix("use crate::"))
        .flat_map(|rest| imports_of(rest.trim_end_matches(';').trim()))
        .collect()
}

/// One `use crate::` body as imports. A brace group at the TOP level names
/// several modules at once and each is its own reach.
fn imports_of(rest: &str) -> Vec<Import> {
    let Some(group) = rest.strip_prefix('{') else {
        return one_import(rest).into_iter().collect();
    };
    split_top(group.strip_suffix('}').unwrap_or(group))
        .iter()
        .filter_map(|part| one_import(part.trim()))
        .collect()
}

/// `lint::Level` -> module `lint` naming `Level`; `lint::{A, B}` -> both.
fn one_import(path: &str) -> Option<Import> {
    let module = leading_ident(path)?;
    let rest = path.get(module.len()..).unwrap_or("");
    let Some(tail) = rest.strip_prefix("::") else {
        // `use crate::lint;` -- the module itself, items deliberately empty.
        return Some(Import {
            module,
            items: vec![],
        });
    };
    let items = match tail.strip_prefix('{') {
        Some(g) => split_top(g.strip_suffix('}').unwrap_or(g))
            .iter()
            .filter_map(|p| last_segment(p.trim()))
            .collect(),
        None => last_segment(tail).into_iter().collect(),
    };
    Some(Import { module, items })
}

/// The final identifier of a path: `a::b::Item` is `Item`.
fn last_segment(path: &str) -> Option<String> {
    let last = path.rsplit("::").next().unwrap_or(path).trim();
    // A nested group under a deeper segment (`a::{b, c}`) has no single last
    // identifier, and reading `{b` as one would invent an item named `b` with
    // a brace on it. Those are rare enough to drop, and dropping them counts
    // the edge as BLOCKING, which is the safe direction (`src/wave:V4`).
    (!last.starts_with('{'))
        .then(|| leading_ident(last))
        .flatten()
}

/// Split on commas that are NOT inside a nested brace group.
fn split_top(group: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut depth = 0usize;
    for c in group.chars() {
        match c {
            '{' => depth = depth.saturating_add(1),
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(String::new());
                continue;
            }
            _ => {}
        }
        if let Some(last) = out.last_mut() {
            last.push(c);
        }
    }
    out
}

/// The identifier a path segment begins with, or nothing.
fn leading_ident(s: &str) -> Option<String> {
    let name: String = s
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// One public TYPE, and the word that declared it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PubType {
    /// The declared name -- `Edge` in `pub struct Edge {`.
    pub name: String,
    /// `struct`, `enum`, `trait` or `type`.
    pub kind: String,
}

/// The words that open a type declaration. `fn` and `const` are deliberately
/// absent: `public_fns` and `signatures` answer those, and this question is
/// about the VOCABULARY -- the names a sibling's signature can spell before
/// either node is written (`.:R57`).
const TYPE_KINDS: [&str; 4] = ["struct", "enum", "trait", "type"];

/// The public types a source declares.
///
/// `pub` only, and `pub(crate)` is NOT one, for the reason [`mod_decls`]
/// already gives: visibility inside the crate is what a bare declaration
/// carries anyway, and reading it as API calls an internal boundary a shared
/// one.
///
/// Over the WHOLE file rather than [`split_module`]'s impl half: that cut
/// lands at the first column-0 `#[cfg(test)]`, and this very file declares
/// `ModDecl` below one. Splitting first would drop a real type from the
/// report while the report still read clean (`.:V48`).
///
/// A declaration inside a STRING LITERAL is counted, which is the trade §C
/// states for the whole node. It is not theoretical: the first fixture
/// written for this function was a block literal of declarations, and the
/// report named four types `src/code` does not have. The fixtures here are
/// written with `\n` escapes for the same reason `V1` matches at column 0.
#[must_use]
pub fn public_types(src: &str) -> Vec<PubType> {
    src.lines().filter_map(|l| one_type(l.trim())).collect()
}

/// The public types a SET of sources declares: deduplicated, sorted by name.
///
/// The vocabulary of a whole node rather than of one file. Both callers that
/// ask -- `seam`, which reports it, and `wave`, which asks whether an import
/// names one -- would otherwise write this fold themselves, and two readings
/// of one question is the defect this node exists to prevent (`V1`).
#[must_use]
pub fn types_in(sources: &[String]) -> Vec<PubType> {
    let mut out: Vec<PubType> = Vec::new();
    for src in sources {
        for t in public_types(src) {
            if !out.contains(&t) {
                out.push(t);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// One `pub struct Foo`, or nothing.
///
/// The keyword is matched with the space after it, so `pub structure` -- or
/// any name merely beginning with a keyword -- declares no type.
fn one_type(line: &str) -> Option<PubType> {
    let rest = line.strip_prefix("pub ")?;
    let kind = TYPE_KINDS
        .iter()
        .find(|k| rest.strip_prefix(**k).is_some_and(|r| r.starts_with(' ')))?;
    let name: String = rest
        .get(kind.len().saturating_add(1)..)?
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then(|| PubType {
        name,
        kind: (*kind).to_string(),
    })
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

    /// `B1`: a brace GROUP names several modules on one line, and taking only
    /// the leading identifier saw NONE of them -- the first character is `{`.
    /// `src/cli` names eight siblings that way and read as reaching for
    /// nothing, which puts it in the first ready set of any graph built from
    /// this.
    ///
    /// Written with `\n` escapes, and naming modules this crate does not
    /// have, for the reason the `TYPES` const below gives: this file is read
    /// as Rust TEXT like any other, so a fixture spelling `use crate::{fed,
    /// spec};` at the start of a line would put edges into the repository's
    /// own code DAG (`src/wave:V1`).
    #[test]
    fn a_brace_group_names_every_module_in_it() {
        let src = "use crate::{render, units};\nuse crate::bpe;\n\
                   use std::path::Path;\n";
        assert_eq!(crate_uses(src), vec!["bpe", "render", "units"]);
    }

    /// A group with one member, and a trailing comma, are the same group.
    #[test]
    fn a_single_member_group_is_still_a_group() {
        assert_eq!(crate_uses("use crate::{bpe};\n"), vec!["bpe"]);
        assert_eq!(crate_uses("use crate::{bpe, };\n"), vec!["bpe"]);
    }

    /// The item half, which `crate_uses` throws away and `src/wave:V4`
    /// needs: WHICH names a line reaches for, not only which module.
    ///
    /// Written with `\n` escapes and naming modules this crate has not got,
    /// for the reason the test above gives.
    #[test]
    fn an_import_carries_the_items_it_names() {
        let one = |src: &str| -> (String, Vec<String>) {
            let i = crate_imports(src).first().cloned().unwrap_or(Import {
                module: String::new(),
                items: vec![],
            });
            (i.module, i.items)
        };
        assert_eq!(
            one("use crate::lint::Level;\n"),
            ("lint".to_string(), vec!["Level".to_string()])
        );
        assert_eq!(
            one("use crate::lint::{Level, Rule};\n"),
            ("lint".into(), vec!["Level".into(), "Rule".into()])
        );
        // A deeper path is the module plus the LAST segment.
        assert_eq!(
            one("use crate::lint::rules::Rule;\n"),
            ("lint".into(), vec!["Rule".into()])
        );
        // The module ITSELF: no item named, which is the strongest reach a
        // line can make and never type-only (`src/wave:V4`).
        assert_eq!(one("use crate::lint;\n"), ("lint".into(), vec![]));

        // A top-level group is several reaches; a NESTED one stays with its
        // own module rather than leaking into the outer list.
        let many = crate_imports("use crate::{lint::Level, render};\n");
        assert_eq!(many.len(), 2, "{many:?}");
        assert_eq!(
            many.iter().map(|i| i.module.clone()).collect::<Vec<_>>(),
            vec!["lint", "render"]
        );
        assert_eq!(
            many.first().map(|i| i.items.clone()),
            Some(vec!["Level".into()])
        );
        assert_eq!(many.get(1).map(|i| i.items.clone()), Some(vec![]));
    }

    /// The vocabulary of a NODE rather than of one file: deduplicated across
    /// sources and sorted, so `seam` and `wave` read one answer.
    #[test]
    fn types_across_sources_are_pooled_and_deduplicated() {
        let sources = vec![
            "pub struct Edge;\npub enum Kind { A }\n".to_string(),
            "pub struct Edge;\npub trait Reader {}\n".to_string(),
        ];
        let names: Vec<String> =
            types_in(&sources).into_iter().map(|t| t.name).collect();
        assert_eq!(names, vec!["Edge", "Kind", "Reader"]);
    }

    /// Written with `\n` escapes rather than as a block, for the reason
    /// `V1` gives one function over: a declaration at column 0 inside a
    /// string literal is read as a real one, and this node reads Rust as
    /// TEXT (§C). Measured -- the block form made `sherd seam` report four
    /// types for `src/code` that exist only in this fixture.
    const TYPES: &str = "pub struct Edge {\n    pub dir: String,\n}\
         \npub enum Verdict { Fits, Over }\
         \npub trait Transport {}\
         \npub type Rows = Vec<Edge>;\
         \nstruct Hidden;\npub(crate) struct Internal;\
         \npub fn go() {}\npub const N: u8 = 1;\n";

    /// The four declaration words, and only those. A `pub fn` or a `pub
    /// const` is a different question -- `public_fns` and `signatures`
    /// already answer it -- and this one is the VOCABULARY a sibling's
    /// signature spells (`.:R57`).
    #[test]
    fn every_public_type_form_is_named_with_the_word_that_declared_it() {
        let t = public_types(TYPES);
        let of = |n: &str| t.iter().find(|p| p.name == n).map(|p| &p.kind);
        assert_eq!(of("Edge"), Some(&"struct".to_string()));
        assert_eq!(of("Verdict"), Some(&"enum".to_string()));
        assert_eq!(of("Transport"), Some(&"trait".to_string()));
        assert_eq!(of("Rows"), Some(&"type".to_string()));
        assert_eq!(t.len(), 4, "a fn and a const are not types: {t:?}");
    }

    /// `pub(crate)` is visibility INSIDE the crate, which is what `mod_decls`
    /// already refuses to read as published: a sibling node cannot name it,
    /// so it is not vocabulary a parallel build can share.
    #[test]
    fn a_private_or_crate_visible_type_is_not_vocabulary() {
        let named: Vec<String> =
            public_types(TYPES).into_iter().map(|p| p.name).collect();
        assert!(!named.contains(&"Hidden".to_string()), "{named:?}");
        assert!(
            !named.contains(&"Internal".to_string()),
            "pub(crate) is not published: {named:?}"
        );
    }

    /// A source declaring no type yields NOTHING rather than erroring --
    /// absence is an answer, not a failure (`.:src/cli:V12`).
    #[test]
    fn a_source_with_no_types_yields_an_empty_vocabulary() {
        assert!(public_types("pub fn a() {}\n").is_empty());
        assert!(public_types("").is_empty());
    }

    /// The keyword is matched WITH the space after it, so a name that merely
    /// begins with one declares nothing.
    #[test]
    fn a_word_beginning_with_a_keyword_is_not_a_declaration() {
        assert!(public_types("pub structure_of(x: u8) {}\n").is_empty());
    }
}
