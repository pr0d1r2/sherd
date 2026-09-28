use super::*;

/// A rule that matches nothing in files that DO exist.
///
/// Distinct from a pattern matching no files, which is the previous test:
/// here the sources are real and the rule finds nothing in them. An empty
/// slice is a SILENT failure -- it would overwrite a real document with
/// nothing and the drift check would then report agreement.
#[test]
fn a_rule_matching_nothing_in_real_files_is_an_error() {
    let d = Decl {
        output: std::path::PathBuf::from("out.md"),
        source: "SPEC.md".into(),
        rule: Rule::Section("\u{a7}NO-SUCH-SECTION".into()),
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let msg = render(root, &d).err().unwrap_or_default();
    assert!(
        msg.contains("matched nothing"),
        "an empty slice must be refused, not written: {msg}"
    );
}

/// A declaration whose SOURCE matches no files at all -- distinct from a
/// rule finding nothing inside files that exist.
#[test]
fn a_source_matching_no_files_is_an_error_not_an_empty_slice() {
    let d = Decl {
        output: std::path::PathBuf::from("out.md"),
        source: "no/such/dir/*".into(),
        rule: Rule::Lead(3),
    };
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let msg = render(root, &d).err().unwrap_or_default();
    assert!(msg.contains("matched no files"), "{msg}");
}

/// `drifted` reports which outputs no longer match their source.
#[test]
fn drift_is_reported_per_output_and_a_regenerated_tree_is_clean() {
    // Reads a file this repo has and the published crate excludes.
    crate::testrepo::dogfood(|| {
        // This repo's gate runs `sherd slice --check` on every commit and
        // requires it clean, so the empty answer here is independently held
        // true rather than merely asserted.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let d = drifted(root);
        assert!(d.is_ok(), "this repo's own declarations must parse");
        assert_eq!(
            d.unwrap_or_default().len(),
            0,
            "the gate keeps this tree regenerated"
        );
    });
}

#[test]
fn lead_takes_the_rule_and_drops_the_justification() {
    let doc = "# KISS\n\nKeep it simple.\n\nBecause complexity costs.\n\nAlso this.\n";
    assert_eq!(Rule::Lead(1).apply(doc), "Keep it simple.");
}

#[test]
fn lead_skips_the_heading_rather_than_counting_it() {
    // A heading is not a paragraph; counting it would return the title.
    let doc = "# T\n\nrule here\n";
    assert_eq!(Rule::Lead(1).apply(doc), "rule here");
}

#[test]
fn fence_takes_the_block_after_its_anchor() {
    let doc = "intro\n\n**Symbols**\n\n```\n! must\n⊥ never\n```\n\nafter\n";
    assert_eq!(Rule::Fence("Symbols".into()).apply(doc), "! must\n⊥ never");
}

#[test]
fn section_stops_at_the_next_heading() {
    let doc = "# A\n\nalpha\n\n# B\n\nbeta\n";
    assert_eq!(Rule::Section("A".into()).apply(doc), "alpha");
}

#[test]
fn prefix_pulls_table_rows() {
    let doc = "id|x\nB1|one\nnoise\nB2|two\n";
    assert_eq!(Rule::Prefix("B".into()).apply(doc), "B1|one\nB2|two");
}

#[test]
fn a_malformed_declaration_is_an_error_not_a_skip() {
    let e = parse_decls("out.txt only-two-fields\n").unwrap_err();
    assert!(e.contains(":1:"), "must name the line: {e}");
}

#[test]
fn comments_and_blanks_are_skipped() {
    let d = parse_decls("# c\n\na.txt b.md lead:2\n").unwrap();
    assert_eq!(d.len(), 1);
    assert_eq!(d.first().map(|x| &x.rule), Some(&Rule::Lead(2)));
}

#[test]
fn an_unknown_rule_names_the_alternatives() {
    let e = Rule::parse("summarise:3").unwrap_err();
    assert!(e.contains("lead:N"), "{e}");
}
