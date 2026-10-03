use super::super::{Confidence, Kind, footprint, invalidators_of, plan_in};
use crate::testrepo::TestRepo;

const ROOT: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nroot\n\n## \u{a7}F FEDERATION\n\n\
dir|owns|\u{22a5}owns|tokens\nscripts|the scripts|-|-\ntools|the tools|-|-\n\n\
## \u{a7}T TASKS\n\nid|status|task|cites\n";

/// #105's fixture: root -> `scripts` (`a.sh`, `b.sh`) holding a row that
/// cites `a.sh`, and a `tools` node with a script of its own.
fn tree(tag: &str, root_rows: &str) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    r.write("SPEC.md", &format!("{ROOT}{root_rows}"))?;
    r.write(
        "scripts/SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nscripts\n\n## \u{a7}V INVARIANTS\n\n\
         V1: a.sh exits 0\n\n## \u{a7}T TASKS\n\nid|status|task|cites\n\
         T1|.|fix a.sh|V1\nT2|.|tidy the node|-\n",
    )?;
    r.write("tools/SPEC.md", "# SPEC\n\n## \u{a7}G GOAL\n\ntools\n")?;
    for s in ["scripts/a.sh", "scripts/b.sh", "tools/c.sh"] {
        r.write(s, "#!/bin/sh\n")?;
    }
    Ok(r)
}

/// V26: a row in a shell node is a STEP at that node, touching the script
/// it cites -- not a root row with no `mod.rs`.
#[test]
fn a_shell_node_row_is_planned_at_its_node() -> Result<(), String> {
    let r = tree("plan-shell-node", "")?;
    let (p, _) = plan_in(r.path(), None);
    let t1 = p
        .steps
        .iter()
        .find(|t| t.id == "T1")
        .ok_or("T1 not a step")?;
    assert_eq!(t1.node, std::path::Path::new("scripts"));
    assert_eq!(footprint(r.path(), t1), Some(vec!["scripts/a.sh".into()]));
    let why =
        invalidators_of(Confidence::Next, t1, Some(&["scripts/a.sh".into()]));
    assert_eq!(why, vec!["a change to scripts/a.sh", "a change to V1"]);
    Ok(())
}

/// A row citing no script is still planned at its node, its footprint
/// UNKNOWN (an empty list) and bounded by the whole node.
#[test]
fn a_row_citing_no_script_has_an_unknown_footprint() -> Result<(), String> {
    let r = tree("plan-shell-unknown", "")?;
    let (p, _) = plan_in(r.path(), None);
    let t2 = p
        .steps
        .iter()
        .find(|t| t.id == "T2")
        .ok_or("T2 not a step")?;
    assert_eq!(footprint(r.path(), t2), Some(vec![]));
    assert_eq!(
        invalidators_of(Confidence::Next, t2, Some(&[])),
        vec!["a change to any script under scripts"]
    );
    Ok(())
}

/// A root row citing a script MOVES to the node owning it; one citing
/// scripts in two nodes stays unmanaged, with a reason.
#[test]
fn a_root_row_goes_to_the_one_node_owning_its_scripts() -> Result<(), String> {
    let r = tree(
        "plan-shell-root",
        "T7|.|harden scripts/a.sh|-\nT8|.|a.sh calls c.sh|-\n",
    )?;
    let (p, _) = plan_in(r.path(), None);
    let t7 = p
        .steps
        .iter()
        .find(|t| t.id == "T7")
        .ok_or("T7 not a step")?;
    assert_eq!(t7.node, std::path::Path::new("scripts"));
    let t8 = p.unmanaged.iter().find(|(t, _)| t.id == "T8");
    assert_eq!(
        t8.map(|(_, k)| *k),
        Some(Kind::MultiFile),
        "{:?}",
        p.unmanaged
    );
    Ok(())
}

/// A Rust step keeps the tdd loop's invalidators exactly (V26: a tree
/// without shell nodes plans as before).
#[test]
fn a_rust_step_keeps_the_loops_invalidators() {
    let t = super::super::Task {
        node: "src/x".into(),
        id: "T1".into(),
        status: '.',
        text: "add f".into(),
        cites: "V1".into(),
    };
    for c in [Confidence::Next, Confidence::Likely, Confidence::Tentative] {
        let want: Vec<String> =
            c.invalidators().iter().map(|w| (*w).to_string()).collect();
        assert_eq!(invalidators_of(c, &t, None), want);
    }
}
