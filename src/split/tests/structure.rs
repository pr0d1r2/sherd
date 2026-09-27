use super::*;

/// This crate is already federated, so every module `lib.rs` declares is
/// a directory: the strongest grade, and nothing left to infer.
#[test]
fn an_already_federated_crate_proposes_its_directories() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let found = structure(root);
    assert!(!found.is_empty());
    assert!(
        found.iter().all(|p| p.evidence == Evidence::Drawn),
        "every module of an already-federated crate is a directory: {found:?}"
    );
}

/// A family with a shared hub: the `use crate::` intersection across all
/// members, which is what makes eleven `*cmd` files one node instead of
/// eleven (`src/plan:V17`).
#[test]
fn a_family_is_proposed_with_the_hub_its_members_share() {
    let dir = std::env::temp_dir().join(format!(
        "sherd-family-{}-{}",
        std::process::id(),
        line!()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let src = dir.join("src");
    let Ok(()) = std::fs::create_dir_all(&src) else {
        unreachable!("a src dir is creatable")
    };
    let write = |name: &str, body: &str| {
        let Ok(()) = std::fs::write(src.join(name), body) else {
            unreachable!("a fixture file is writable")
        };
    };
    write("lib.rs", "mod acmd;\nmod bcmd;\nmod ccmd;\n");
    write("acmd.rs", "use crate::render;\nuse crate::units;\n");
    write("bcmd.rs", "use crate::render;\n");
    write("ccmd.rs", "use crate::render;\n");

    let found = structure(&dir);
    let family = found.iter().find(|p| p.members.len() == 3);
    let Some(family) = family else {
        unreachable!("three of a suffix is a family: {found:?}")
    };
    assert_eq!(family.evidence, Evidence::Cohesion);
    assert_eq!(family.shared, vec!["render"], "units is used by one");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A hub is what EVERY member reaches for. `render` is shared; `units`
/// is used by one member and is not.
#[test]
fn a_shared_hub_is_present_in_every_member() {
    let lists = vec![
        vec!["cli".to_string(), "render".to_string(), "units".to_string()],
        vec!["cli".to_string(), "render".to_string()],
    ];
    assert_eq!(shared_across(&lists), vec!["cli", "render"]);
    assert!(shared_across(&[]).is_empty());
}

/// Two of a suffix is a coincidence; three is a family. Without the
/// floor, every `*s` plural in a crate becomes a proposed node.
#[test]
fn a_family_needs_more_than_a_pair() {
    let decl = |n: &str| crate::code::ModDecl {
        name: n.to_string(),
        is_pub: false,
    };
    let pair = [decl("acmd"), decl("bcmd")];
    assert!(families(&pair).is_empty());
    let three = [decl("acmd"), decl("bcmd"), decl("ccmd")];
    assert_eq!(families(&three).len(), 1);
}
