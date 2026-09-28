use super::*;
use crate::testrepo::TestRepo;

const BASE: &str = "pub fn old() -> u8 {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { super::old(); }\n}\n";

const WITH_STUB: &str = "pub fn old() -> u8 {\n    1\n}\n\npub fn added(_x: u8) -> u8 {\n    0\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn t() { super::old(); }\n    #[test]\n    fn u() { assert_eq!(super::added(1), 0); }\n}\n";

/// A commit adding a `pub fn` that only its own test calls.
fn repo_with_a_stub(tag: &str) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    r.write("src/thing/mod.rs", BASE)?;
    r.commit("base")?;
    r.write("src/thing/mod.rs", WITH_STUB)?;
    r.commit("add a fn")?;
    Ok(r)
}

#[test]
fn added_in_commit_reads_pub_fns_out_of_the_diff() {
    assert_eq!(check_added(), Ok(()));
}

/// Flatten to just the names, so each assertion states one thing.
fn names_in(added: &AddedFns) -> Vec<&str> {
    added
        .iter()
        .flat_map(|(_, v)| v.iter().map(String::as_str))
        .collect()
}

/// A diff touching one module, with every shape the parser must sort.
const MIXED: &str = concat!(
    "+++ b/src/x/mod.rs\n",
    "+pub fn added(a: u8) -> bool {\n",
    "+pub fn generic<T>(t: T) {\n",
    "-pub fn removed() {\n",
    " pub fn untouched() {\n",
    "+fn private() {\n",
    "+++ b/src/x/other.rs\n",
    "+pub fn not_a_module_file() {\n"
);

/// The diff parser, direct. It was only ever reachable through `git`, so
/// its edge cases needed a repo to state.
#[test]
fn only_added_pub_fns_in_a_module_are_collected() {
    let got = added_fns(MIXED);
    assert_eq!(
        names_in(&got),
        vec!["added", "generic"],
        "a REMOVED fn is not added, a private one is not public, and a \
             file that is not `mod.rs` is not a node module"
    );
}

#[test]
fn an_empty_diff_adds_nothing_without_erroring() {
    // Distinct from a diff that could not be READ, which is V7's whole
    // point: this one really did change nothing.
    assert!(added_fns("").is_empty());
}

fn check_added() -> Result<(), String> {
    let r = repo_with_a_stub("review-added")?;
    let added = added_in_commit(r.path(), "HEAD").map_err(|e| e.to_string())?;
    let names = names_in(&added);
    assert!(
        names.contains(&"added"),
        "the new fn must be seen: {names:?}"
    );
    assert!(
        !names.contains(&"old"),
        "not added by this commit: {names:?}"
    );
    Ok(())
}

#[test]
fn a_commit_touching_no_module_yields_nothing() {
    assert_eq!(check_spec_only(), Ok(()));
}

/// `added_in_commit` reports `mod.rs` only, so a spec-only commit has
/// nothing to review for added functions.
fn check_spec_only() -> Result<(), String> {
    let r = TestRepo::new("review-spec")?;
    r.write("SPEC.md", "# SPEC\n\nchanged\n")?;
    r.commit("spec only")?;
    assert!(
        added_in_commit(r.path(), "HEAD")
            .map_err(|e| e.to_string())?
            .is_empty(),
        "a spec-only commit adds no pub fn"
    );
    Ok(())
}

#[test]
fn commit_finds_the_stub_shape_end_to_end() {
    assert_eq!(check_stub_found(), Ok(()));
}

/// `added` reads its input, but nothing outside its own test calls it --
/// which is `unwired`, the finding that says a helper landed and was
/// never wired in. B1 and B2 are the two ways this was read wrong.
fn check_stub_found() -> Result<(), String> {
    let r = repo_with_a_stub("review-stub")?;
    let found = commit(r.path(), "HEAD").map_err(|e| e.to_string())?;
    assert!(
        found.iter().any(|(_, f)| f.rule == "unwired"),
        "expected an unwired finding, got {found:?}"
    );
    Ok(())
}

#[test]
fn an_unreadable_module_is_an_error_not_a_clean_review() {
    // "Unreadable is not clean" -- this module's own doc, and `.:V48`: a
    // file that could not be read is a failure, never a quiet zero.
    let missing = Path::new("/nonexistent/thing/mod.rs");
    assert!(node(missing, &["x".to_string()]).is_err());
}
