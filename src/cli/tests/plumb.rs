use super::*;

fn rec(path: &str) -> Finding {
    Finding::new(Path::new(path), "sherd/V9", "a \"quoted\" msg".to_string())
}

/// The text line is `path[:line]: [rule: ]message` -- the shape every
/// check family printed before it collected, so the porcelain is unchanged.
#[test]
fn a_finding_prints_the_line_the_text_form_always_printed() {
    assert_eq!(
        rec("/r/SPEC.md").at(3).text(),
        "/r/SPEC.md:3: sherd/V9: a \"quoted\" msg"
    );
    assert_eq!(
        rec("/r/SPEC.md").text(),
        "/r/SPEC.md: sherd/V9: a \"quoted\" msg"
    );
    assert_eq!(
        Finding::bare(Path::new("/r/a"), "edge skips".into()).text(),
        "/r/a: edge skips",
        "no rule label where the text form printed none"
    );
}

/// The json names the file ROOT-relative, the root itself as `.`, and a
/// missing line or rule as `null` -- never an invented id or a 0.
#[test]
fn a_finding_serialises_root_relative_with_nulls_for_what_it_lacks() {
    let root = Path::new("/r");
    assert_eq!(
        rec("/r/a/SPEC.md").at(0).advisory().json(root),
        r#"{"file":"a/SPEC.md","line":0,"rule":"sherd/V9","message":"a \"quoted\" msg","fatal":false}"#
    );
    assert_eq!(
        Finding::bare(Path::new("/r"), "x".into()).json(root),
        r#"{"file":".","line":null,"rule":null,"message":"x","fatal":true}"#
    );
}

fn row(node: &str, chain: u64, over_by: Option<u64>) -> BudgetRow {
    BudgetRow {
        node: PathBuf::from(node),
        chain_tokens: chain,
        own_tokens: 7,
        chain_nodes: 2,
        ceiling: 50,
        over_by,
    }
}

fn two_rows() -> Budget {
    Budget {
        rows: vec![row("/r", 40, None), row("/r/a", 60, Some(10))],
        total: 100,
        over: 1,
        method: Some("o200k"),
        ..Budget::default()
    }
}

/// Pinned WHOLE, as `src/plan:V25` pins `plan`'s: a caller parsing a layout
/// breaks silently on a cosmetic edit, so the keys are the contract.
#[test]
fn budget_json_is_pinned_whole() {
    let want = format!(
        concat!(
            r#"{{"version":"{}","ok":false,"window":131072,"entry":28543,"working":102529,"#,
            r#""method":"o200k","nodes_examined":2,"all_chains_tokens":100,"over":1,"cold":false,"#,
            r#""nodes":[{{"node":".","chain_tokens":40,"own_tokens":7,"chain_nodes":2,"ceiling":50,"over_by":null}},"#,
            r#"{{"node":"a","chain_tokens":60,"own_tokens":7,"chain_nodes":2,"ceiling":50,"over_by":10}}],"#,
            r#""unmeasured":[]}}"#
        ),
        VERSION
    );
    assert_eq!(budget_json(Path::new("/r"), &two_rows()), want);
}

/// The text form of the same report, byte for byte as `budget` printed it
/// before it had a json one.
#[test]
fn budget_text_is_unchanged() {
    assert_eq!(
        budget_text(Path::new("/r"), &two_rows()),
        concat!(
            "window 131072 · entry 28543 · working 102529\n\n",
            "  .                        chain     40 tok  (2 nodes)  ceiling     50\n",
            "  a                        chain     60 tok  (2 nodes)  ceiling     50  OVER by 10\n",
            "\n  2 nodes examined · 100 tok if all chains loaded · 1 over ceiling\n",
        )
    );
}

/// One exit code, read by the process AND by the json `ok`: unmeasured 1,
/// nothing matched 2, over a WARM ceiling 1, over a cold one 0.
#[test]
fn the_budget_verdict_is_decided_once() {
    let mut b = two_rows();
    assert_eq!(b.code(), 1, "over a warm ceiling");
    b.cold = true;
    assert_eq!(b.code(), 0, "cold: advisory");
    b.stderr = Some("sherd: x".into());
    assert_eq!(b.code(), 1, "unmeasured, whatever else");
    assert_eq!(Budget::default().code(), 2, "no node matched");
}

/// `validate`'s slice registry three ways, and an unreadable one is ONE
/// failure (`.:V48`) -- pinned whole.
#[test]
fn validate_json_is_pinned_whole() {
    let v = ValidateReport {
        nodes: 2,
        structural: vec![],
        edges: vec![Finding::bare(Path::new("/r"), "edge".into())],
        over: vec![],
        cold: true,
        slices: Slices::Unreadable("bad line".into()),
    };
    assert_eq!(v.failures(), 2, "an edge and an unreadable registry");
    assert_eq!(
        validate_json(Path::new("/r"), &v),
        format!(
            concat!(
                r#"{{"version":"{}","ok":false,"nodes_examined":2,"structural":[],"#,
                r#""edges":[{{"file":".","line":null,"rule":null,"message":"edge","fatal":true}}],"#,
                r#""over_ceiling":[],"ceilings_cold":true,"slices":"unreadable","#,
                r#""slice_error":"bad line","drifted":[]}}"#
            ),
            VERSION
        )
    );
    let read = ValidateReport {
        slices: Slices::Read(vec![PathBuf::from("/r/s.md")]),
        edges: vec![],
        ..v
    };
    assert_eq!(read.failures(), 1, "one drifted slice");
    assert!(validate_json(Path::new("/r"), &read).ends_with(
        r#""slices":"read","slice_error":null,"drifted":["s.md"]}"#
    ));
}

/// `--format` is lifted out wherever it sits and the verb stays at `[0]`,
/// so `arg_dir` reads `[dir]` from the same place with or without it.
#[test]
fn with_format_keeps_the_verb_and_drops_the_flag() {
    let a = |s: &[&str]| s.iter().map(|x| (*x).to_string()).collect::<Vec<_>>();
    assert_eq!(
        with_format(&a(&["budget", "--format", "json", "src"])),
        Ok((true, a(&["budget", "src"])))
    );
    assert_eq!(with_format(&a(&["check"])), Ok((false, a(&["check"]))));
    assert!(with_format(&a(&["validate", "--format", "yaml"])).is_err());
}
