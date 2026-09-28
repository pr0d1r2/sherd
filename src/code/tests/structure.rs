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
