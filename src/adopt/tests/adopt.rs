use super::*;
use crate::testrepo::TestRepo;

/// A monolith of the shape every unfederated repository has: rules,
/// tasks and bugs in one file, citing each other by bare id.
const SOURCE: &str = "# SPEC\n\n## \u{a7}G GOAL\n\nparse input, render output\n\n\
## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\nsrc|parsing and rendering code|the goal itself|-\n\n\
## \u{a7}V INVARIANTS\n\nV1: the parser rejects malformed input rather than guessing\n\
V2: rendering never mutates the tree the parser produced\nV3: every release is tagged, see V1\n\n\
## \u{a7}T TASKS\n\nid|status|task|cites\nT1|.|nested groups|V1\nT2|.|render colour output|V2\n\n\
## \u{a7}B BUGS\n\nid|date|cause|fix\nB1|2026-01-01|the parser looped on empty input|V1 restated\n";

/// The `§F` table that DECLARES the two nodes. Placement may name these
/// and nothing else (V3).
const CHILD: &str = "# SPEC\n\n## \u{a7}G GOAL\n\ncode nodes\n\n\
## \u{a7}F FEDERATION\n\ndir|owns|\u{22a5}owns|tokens\n\
parse|parser, input, malformed rejection|rendering, output colour|-\n\
render|rendering, output, colour|parsing, input|-\n";

/// A repo mid-adoption: one monolith, a declared federation, and two
/// nodes scaffolded by `init` and so carrying zero ids of their own.
fn tree(tag: &str) -> Result<TestRepo, String> {
    let r = TestRepo::new(tag)?;
    r.write("SPEC.md", SOURCE)?;
    r.write("src/SPEC.md", CHILD)?;
    r.write("src/parse/SPEC.md", &spec::scaffold("src/parse", &[]))?;
    r.write("src/render/SPEC.md", &spec::scaffold("src/render", &[]))?;
    Ok(r)
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| e.to_string())
}

/// V1. Six rows in, six rows accounted for. A row read and then dropped
/// costs the memory of a defect, and nothing else would notice.
#[test]
fn every_row_is_accounted_for_exactly_once() -> Result<(), String> {
    let r = tree("adopt-conserve")?;
    let p = propose(r.path())?;
    assert_eq!(p.rows_in, 6, "V1 V2 V3 T1 T2 B1");
    assert_eq!(
        p.placements.len().saturating_add(p.unplaced.len()),
        p.rows_in,
        "placed {:?}, unplaced {:?}",
        p.placements.iter().map(|x| &x.id).collect::<Vec<_>>(),
        p.unplaced
    );
    Ok(())
}

/// V2. `V2` names rendering AND the parser, so both lenses claim it
/// equally. A tie is not an answer: the row stays at root and is named,
/// rather than being rounded to whichever node was walked first.
#[test]
fn a_row_two_nodes_claim_equally_is_placed_nowhere() -> Result<(), String> {
    let r = tree("adopt-tie")?;
    let p = propose(r.path())?;
    assert!(
        p.unplaced.contains(&"V2".to_string()),
        "a tie is not a placement: {:?}",
        p.placements.iter().map(|x| &x.id).collect::<Vec<_>>()
    );
    Ok(())
}

/// V3. A home no `§F` row declares is refused. Inventing an edge here
/// would place a row into a node no reader can reach by descent.
#[test]
fn a_home_no_table_declares_is_refused() -> Result<(), String> {
    let r = tree("adopt-undeclared")?;
    let map = read_map("V1 src/nowhere")?;
    let refused = refusals(r.path(), &map);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused.iter().any(|m| m.contains("src/nowhere")),
        "{refused:?}"
    );
    Ok(())
}

/// V1, in the direction that is easy to miss: a destination that already
/// declares the id would end up with TWO rows numbered `V1`, which is the
/// copy V1 forbids wearing the costume of a move.
#[test]
fn a_destination_already_holding_the_id_is_refused() -> Result<(), String> {
    let r = tree("adopt-collide")?;
    let held = read(r.path(), "src/parse/SPEC.md")?
        + "V1: a rule this node already had\n";
    r.write("src/parse/SPEC.md", &held)?;
    let map = read_map("V1 src/parse")?;
    assert!(
        refusals(r.path(), &map).iter().any(|m| m.contains("V1")),
        "two rows with one id must be refused"
    );
    Ok(())
}

/// V4. The number survives, the owner is added, and the row itself is
/// carried verbatim rather than retyped.
#[test]
fn a_moved_row_keeps_its_number_and_its_citers_follow() -> Result<(), String> {
    let r = tree("adopt-requalify")?;
    apply(r.path(), &read_map("V1 src/parse\nT2 src/render")?)?;
    let root = read(r.path(), "SPEC.md")?;
    let parse = read(r.path(), "src/parse/SPEC.md")?;
    assert!(root.contains("tagged, see `src/parse:V1`"), "{root}");
    assert!(!root.contains("V1: the parser"), "the row left root");
    assert!(parse.contains("V1: the parser rejects"), "arrived intact");
    assert!(
        read(r.path(), "src/render/SPEC.md")?.contains("`.:V2`"),
        "a citation pointing back at root gains root's own owner"
    );
    Ok(())
}

/// `B1`. A brownfield spec generally does not pass `check` already, so a
/// migration is judged on what it ADDS. ashlar's `§B` rows run B3, B2, B1
/// -- refusing on the total refused every repository the verb is for.
#[test]
fn a_violation_the_source_already_had_does_not_refuse_the_move()
-> Result<(), String> {
    let r = tree("adopt-carried")?;
    let unsorted = read(r.path(), "SPEC.md")?.replace(
        "B1|2026-01-01",
        "B3|2026-01-01|a later bug, filed first|V1 restated\n\
             B1|2026-01-01",
    );
    r.write("SPEC.md", &unsorted)?;
    assert!(
        !spec::check(&unsorted).is_empty(),
        "the source is already bad"
    );
    let out = apply(r.path(), &read_map("T2 src/render")?)?;
    assert_eq!(out.moved, 1, "the move still happens");
    assert!(!out.carried.is_empty(), "and the tree's own is reported");
    Ok(())
}

/// A monolith whose milestone row lists its tasks as a RANGE -- the form
/// `microlith/V15` documents as the cheap way to maintain the column, and
/// so the form a brownfield spec is most likely to carry.
fn tree_with_milestone(tag: &str, cell: &str) -> Result<TestRepo, String> {
    let r = tree(tag)?;
    let source = read(r.path(), "SPEC.md")?.replace(
        "## \u{a7}T TASKS\n\nid",
        &format!(
            "## \u{a7}T TASKS\n\n| id | scope | tasks | done-when |\n\
                 |----|-------|-------|-----------|\n\
                 | M1 | first milestone | {cell} | both are done |\n\nid"
        ),
    );
    r.write("SPEC.md", &source)?;
    Ok(r)
}

/// V9 (`B5`). Three readings of one tree, because the rule is a
/// DIFFERENCE and asserting only the refusal would be satisfied by a
/// check that refuses everything.
#[test]
fn a_milestone_range_is_refused_only_when_the_map_splits_it()
-> Result<(), String> {
    let r = tree_with_milestone("adopt-range", "T1-T2")?;
    let refused = refusals(r.path(), &read_map("T1 src/parse")?);
    assert_eq!(refused.len(), 1, "{refused:?}");
    let first = refused.first().map_or("", String::as_str);
    assert!(first.contains("`M1`"), "{first}");
    assert!(first.contains("RANGE (`T1-T2`)"), "the cell AS WRITTEN");
    assert!(first.contains("moves T1"), "{first}");

    // A range NOTHING moves is left alone, or `V6`'s rerun over a
    // migrated tree would fail forever.
    let untouched = refusals(r.path(), &read_map("B1 src/parse")?);
    assert!(untouched.is_empty(), "{untouched:?}");

    // And the spelled-out form migrates, which is the whole point of
    // naming the range rather than refusing every milestone table.
    let ids = tree_with_milestone("adopt-ids", "T1, T2")?;
    let ok = refusals(ids.path(), &read_map("T1 src/parse")?);
    assert!(ok.is_empty(), "{ok:?}");
    assert_eq!(apply(ids.path(), &read_map("T1 src/parse")?)?.moved, 1);
    Ok(())
}

/// V5. The output is checked BEFORE it is offered: a migration whose
/// result the project's own checker rejects has shipped a second dialect.
#[test]
fn the_migration_passes_the_checker_that_gates_every_spec() -> Result<(), String>
{
    let r = tree("adopt-checked")?;
    let map = read_map("V1 src/parse\nB1 src/parse\nT2 src/render")?;
    let report = apply(r.path(), &map)?;
    assert_eq!(report.moved, 3);
    assert_eq!(report.moved.saturating_add(report.stayed), report.rows_in);
    for rel in ["SPEC.md", "src/parse/SPEC.md", "src/render/SPEC.md"] {
        let found = spec::check(&read(r.path(), rel)?);
        assert!(found.is_empty(), "{rel}: {found:?}");
    }
    Ok(())
}

/// V7. A node that already holds rows RECEIVES in id order. Appending put
/// a moved `T3` below a resident `T88`, and V5's own check then refused
/// the whole migration over an order the verb itself had produced (`B3`).
#[test]
fn received_rows_land_in_id_order_among_resident_rows() -> Result<(), String> {
    let r = tree("adopt-order")?;
    r.write(
        "src/parse/SPEC.md",
        "# SPEC\n\n## \u{a7}G GOAL\n\nparsing\n\n\
## \u{a7}V INVARIANTS\n\nV5: a rule this node already had\n\n\
## \u{a7}T TASKS\n\nid|status|task|cites\nT9|.|a task this node already had|-\n",
    )?;
    apply(r.path(), &read_map("V1 src/parse\nT1 src/parse")?)?;
    let parse = read(r.path(), "src/parse/SPEC.md")?;
    let at = |needle: &str| {
        parse
            .find(needle)
            .ok_or(format!("{needle} missing:\n{parse}"))
    };
    assert!(at("V1: the parser")? < at("V5: a rule")?, "{parse}");
    assert!(at("T1|.|nested")? < at("T9|.|a task")?, "{parse}");
    assert!(spec::check(&parse).is_empty(), "{parse}");
    Ok(())
}

/// V5, the half that matters more: a refused migration writes NOTHING.
/// Half-applied is worse than not started, because the counts that would
/// tell you so are the ones that never ran.
#[test]
fn a_refused_map_leaves_every_file_untouched() -> Result<(), String> {
    let r = tree("adopt-atomic")?;
    let before = read(r.path(), "SPEC.md")?;
    let map = read_map("V1 src/parse\nV3 src/nowhere")?;
    assert!(apply(r.path(), &map).is_err(), "the map names no such node");
    assert_eq!(read(r.path(), "SPEC.md")?, before);
    assert!(!read(r.path(), "src/parse/SPEC.md")?.contains("V1:"));
    Ok(())
}

/// V6. A foreign repo is edited between the proposal and the apply, so a
/// verb unsafe to repeat is a verb nobody dares finish. A second run over
/// a migrated tree finds nothing to move.
#[test]
fn a_second_run_over_a_migrated_tree_moves_nothing() -> Result<(), String> {
    let r = tree("adopt-rerun")?;
    let first = propose(r.path())?;
    let text: String = first
        .placements
        .iter()
        .map(|p| format!("{} {}\n", p.id, p.home))
        .collect();
    apply(r.path(), &read_map(&text)?)?;
    let again = propose(r.path())?;
    assert!(
        again.placements.is_empty(),
        "still proposing: {:?}",
        again.placements.iter().map(|p| &p.id).collect::<Vec<_>>()
    );
    Ok(())
}

/// `B2`. Requalification writes the destination's PATH into every row
/// that cited a moved id, so scoring the raw text lets one migration
/// justify the next: a rerun over a migrated ashlar proposed twenty more
/// moves, each because the previous run had written `src/git` into the
/// row. A citation is a link, not a subject.
#[test]
fn a_citation_is_not_a_word_the_row_matches_on() {
    let cited = spec::Row {
        section: 'T',
        id: "T9".to_string(),
        text: "T9|.|tagged release|`src/render:V2`".to_string(),
        line: 1,
    };
    let homes = vec![fed::Home {
        node: "src/render".to_string(),
        owns: "render, output, template".to_string(),
        not_owns: String::new(),
    }];
    assert!(
        best(&cited, &homes).is_none(),
        "`render` reached the row only through the citation"
    );
}

/// The map is a file a human edits, so it forgives comments and blank
/// lines and refuses the two things that would silently change what gets
/// moved: a line that is not a mapping, and an id given two homes.
#[test]
fn a_map_forgives_comments_and_refuses_ambiguity() {
    let ok = read_map("# a note\n\nV1 src/parse  # why\nT2 src/render\n");
    assert_eq!(ok.map(|m| m.len()), Ok(2));
    assert!(
        read_map("V1 src/parse\nV1 src/render").is_err(),
        "two homes"
    );
    assert!(read_map("V1").is_err(), "not a mapping");
}
