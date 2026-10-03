//! V10: a row is claimed by the node owning the script it cites, before any
//! lens word is read. Each case sits beside one the lens still decides, so a
//! placer that ignores scripts, or ignores words, fails here.

use super::*;
use crate::testrepo::TestRepo;

/// `a` owns parsing and `b` rendering, by their lenses. Scripts sit where
/// the code does: `a/x/one.sh`, `b/two.sh`, a `run.sh` in each, and
/// `top.sh` at the root, which no node owns.
const SOURCE: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nparse and render\n\n\
## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n\
a|parser, input, malformed rejection|rendering, output colour|-\n\
b|rendering, output, colour|parser, input|-\n\n\
## \u{a7}V INVARIANTS\n\n\
V1: `one.sh` renders output colour\n\
V2: `b/two.sh` rejects malformed parser input\n\
V3: `one.sh` hands its result to `two.sh`\n\
V4: `run.sh` renders output colour\n\
V5: the parser rejects malformed input\n\
V6: `top.sh` rejects malformed parser input\n";

fn tree() -> Result<TestRepo, String> {
    let r = TestRepo::new("adopt-script")?;
    r.write("SPEC.md", SOURCE)?;
    r.write("a/SPEC.md", &spec::scaffold("a", &[]))?;
    r.write("b/SPEC.md", &spec::scaffold("b", &[]))?;
    for s in ["a/x/one.sh", "b/two.sh", "a/run.sh", "b/run.sh", "top.sh"] {
        r.write(s, "#!/bin/sh\n")?;
    }
    Ok(r)
}

fn home<'a>(p: &'a Proposal, id: &str) -> Option<&'a Placement> {
    p.placements.iter().find(|x| x.id == id)
}

/// The cited script decides, even against the other node's words.
#[test]
fn a_cited_script_claims_its_row_for_the_node_that_owns_it()
-> Result<(), String> {
    let r = tree()?;
    let p = propose(r.path())?;
    let v1 = home(&p, "V1").ok_or("V1 unplaced")?;
    assert_eq!(
        (v1.home.as_str(), v1.why.clone()),
        ("a", vec!["a/x/one.sh".to_string()])
    );
    let v2 = home(&p, "V2").ok_or("V2 unplaced")?;
    assert_eq!(
        (v2.home.as_str(), v2.why.clone()),
        ("b", vec!["b/two.sh".to_string()])
    );
    Ok(())
}

/// Scripts in two nodes, or one no node owns, leave the row at root,
/// named, as a tie does (V2).
#[test]
fn scripts_in_two_nodes_or_at_root_place_the_row_nowhere() -> Result<(), String>
{
    let r = tree()?;
    let p = propose(r.path())?;
    assert!(p.unplaced.contains(&"V3".to_string()), "{:?}", p.placements);
    assert!(p.unplaced.contains(&"V6".to_string()), "{:?}", p.placements);
    Ok(())
}

/// No resolvable script -- an ambiguous basename, or none at all -- and the
/// lens decides, exactly as before.
#[test]
fn without_a_resolvable_script_the_lens_decides() -> Result<(), String> {
    let r = tree()?;
    let p = propose(r.path())?;
    assert_eq!(home(&p, "V4").map(|x| x.home.as_str()), Some("b"));
    assert_eq!(home(&p, "V5").map(|x| x.home.as_str()), Some("a"));
    assert_eq!(p.rows_in, 6, "every row is still accounted for (V1)");
    assert_eq!(p.placements.len() + p.unplaced.len(), 6);
    Ok(())
}
