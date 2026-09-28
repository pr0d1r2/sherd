use super::*;

/// A `flake.lock` shaped like ours: `nix-hk` sorts first AND carries an
/// `inputs` entry named `nixpkgs`, which is what made the prefix match
/// read the wrong node.
const LOCK: &str = r#"{
  "nodes": {
    "nix-hk": {
      "inputs": {
        "nixpkgs": [
          "nix-hk",
          "nixpkgs-lock",
          "nixpkgs"
        ]
      },
      "locked": {
        "lastModified": 1786698718,
        "rev": "a687c1404575d67f425e17c6bee9ad75dfa14728",
        "type": "github"
      },
      "original": {
        "owner": "pr0d1r2",
        "repo": "nix-hk",
        "type": "github"
      }
    },
    "nixpkgs": {
      "locked": {
        "lastModified": 1786535285,
        "rev": "9f78f44a87948854445dae0b6bf82b2e87e4efb5",
        "type": "github"
      },
      "original": {
        "owner": "NixOS",
        "ref": "nixos-26.05",
        "repo": "nixpkgs",
        "type": "github"
      }
    }
  }
}"#;

const MANIFEST: &str = "\
[package]
name = \"sherd\"
edition = \"2024\"
rust-version = \"1.95\"

[[bin]]
name = \"sherd\"
path = \"src/main.rs\"

[dependencies]
# a comment that is not a dependency
itok = { version = \"0.3\", default-features = false, features = [
  \"bpe\",
] }
microlith = \"0.6\"
ureq = { version = \"3\", default-features = false, features = [
  \"rustls\",
], optional = true }

[lints.rust]
unsafe_code = \"forbid\"
";

#[test]
fn manifest_values_come_from_the_package_block() {
    assert_eq!(manifest_value(MANIFEST, "edition").as_deref(), Some("2024"));
    assert_eq!(
        manifest_value(MANIFEST, "rust-version").as_deref(),
        Some("1.95")
    );
    assert_eq!(manifest_value(MANIFEST, "absent"), None);
}

/// The failure this function exists for: three keys spread over seven
/// lines, one comment, and a `[lints.rust]` table that must not be
/// counted as a dependency.
#[test]
fn a_wrapped_dependency_is_still_one_dependency() {
    assert_eq!(direct_dependencies(MANIFEST), 3);
}

#[test]
fn a_manifest_with_no_dependencies_counts_none() {
    assert_eq!(direct_dependencies("[package]\nname = \"x\"\n"), 0);
}

#[test]
fn ratchets_read_their_own_line() {
    let cov = "# comment\nlines 90.58\n# trailing note\n";
    assert_eq!(ratchet(cov, "lines").as_deref(), Some("90.58"));
    assert_eq!(ratchet("total 270\n", "total").as_deref(), Some("270"));
    assert_eq!(ratchet(cov, "total"), None);
}

/// Truncation, not rounding: 98.06 and 98.04 must land on the same
/// tenth, or the badge is a number one platform cannot reproduce.
#[test]
fn a_percentage_truncates_rather_than_rounds() {
    assert_eq!(truncate_tenth("98.06").as_deref(), Some("98.0"));
    assert_eq!(truncate_tenth("98.04").as_deref(), Some("98.0"));
    assert_eq!(truncate_tenth("90.58").as_deref(), Some("90.5"));
    assert_eq!(truncate_tenth("not a number"), None);
}

/// The regression `dev:B1` is made of: `nix-hk`'s INPUTS block carries a
/// line beginning `"nixpkgs":`, indistinguishable from a node header to
/// a prefix match, and the first `rev` after it is nix-hk's. The badge
/// named the wrong project's commit until this test existed.
#[test]
fn a_node_header_is_read_and_an_inputs_entry_of_the_same_name_is_not() {
    let lock = LOCK;
    let np = locked(lock, "nixpkgs").unwrap_or_else(|| unreachable!());
    assert_eq!(np.rev, "9f78f44", "the nixpkgs NODE, not nix-hk's input");
    assert_eq!(np.date, "2026-08-12");
    assert_eq!(np.release.as_deref(), Some("26.05"));

    let hk = locked(lock, "nix-hk").unwrap_or_else(|| unreachable!());
    assert_eq!(hk.rev, "a687c14");
    assert_eq!(
        hk.release, None,
        "an input with no branch claims no release"
    );

    assert!(locked(lock, "absent").is_none());
}

#[test]
fn a_shield_field_doubles_what_shields_would_read_as_syntax() {
    assert_eq!(shield_text("2026-08-12"), "2026--08--12");
    assert_eq!(shield_text("lint_debt"), "lint__debt");
    assert_eq!(shield_text("26.05"), "26.05");
}

#[test]
fn a_timestamp_becomes_a_utc_date() {
    assert_eq!(civil_date(1_786_535_285), "2026-08-12");
    assert_eq!(civil_date(0), "1970-01-01");
    // A leap day, because February is where a hand-rolled calendar
    // breaks and nothing else would notice.
    assert_eq!(civil_date(1_709_164_800), "2024-02-29");
}

/// `check` is BOTH a step and a hook here, so a name-based exclusion
/// would drop a real step and look correct doing it.
#[test]
fn gate_steps_counts_steps_and_not_hooks() {
    let pkl = "\
env {
  [\"HK_HIDE_WHEN_DONE\"] = \"true\"
}

local fast = new Mapping<String, Step> {
  [\"fmt\"] {
  }
  [\"check\"] {
  }
}

local all = (fast) {
  [\"coverage\"] {
  }
}

hooks {
  [\"pre-commit\"] {
  }
  [\"check\"] {
  }
}
";
    assert_eq!(gate_steps(pkl), 3);
}

#[test]
fn platforms_come_from_the_matrix_and_ubuntu_means_two_vendors() {
    let yml = "        os: [ubuntu-latest, ubuntu-24.04-arm, macos-latest]\n";
    assert_eq!(
        ci_platforms(yml),
        vec![
            ("amd".to_string(), "linux".to_string()),
            ("arm".to_string(), "linux".to_string()),
            ("arm".to_string(), "macos".to_string()),
            ("intel".to_string(), "linux".to_string()),
        ]
    );
}

#[test]
fn a_workflow_with_no_matrix_yields_no_platform_claim() {
    assert!(ci_platforms("jobs:\n  gate:\n").is_empty());
}

fn facts() -> Facts {
    Facts {
        edition: "2024".to_string(),
        msrv: "1.95".to_string(),
        deps: 4,
        gate_steps: 23,
        coverage_floor: "90.5".to_string(),
        lint_debt: "17.0".to_string(),
        nodes: 16,
        nixpkgs: Locked {
            rev: "9f78f44".to_string(),
            date: "2026-08-12".to_string(),
            release: Some("26.05".to_string()),
        },
        platforms: vec![("arm".to_string(), "macos".to_string())],
    }
}

#[test]
fn every_fact_reaches_the_rendered_block() {
    let out = render(&facts());
    for expected in [
        "edition-2024",
        "MSRV-1.95",
        "direct_dependencies-4",
        "gate_steps-23",
        "90.5",
        "17.0%2FKLoC",
        "federated_nodes-16",
        "9f78f44",
        "logo=arm",
        "unsafe-forbidden",
        // The nixpkgs badge names the RELEASE, the day the rev was last
        // modified, and the rev -- with every literal dash doubled, or
        // shields.io reads the date as field separators.
        "nixpkgs-26.05_(2026--08--12_--_9f78f44)",
    ] {
        assert!(out.contains(expected), "missing {expected} in:\n{out}");
    }
}

/// No badge may claim something that does not exist yet.
#[test]
fn nothing_claims_a_registry_or_a_run() {
    let out = render(&facts());
    assert!(!out.contains("crates.io"));
    assert!(!out.contains("docs.rs"));
    assert!(!out.contains("badge.svg)](https://github.com"));
}

#[test]
fn splice_replaces_only_between_one_blocks_markers() {
    let readme =
        "# t\n\n<!-- BEGIN badges -->\nold\n<!-- END badges -->\n\nbody\n";
    let out = splice_named(readme, "badges", "new\n");
    assert_eq!(
        out.as_deref(),
        Some(
            "# t\n\n<!-- BEGIN badges -->\nnew\n<!-- END badges -->\n\nbody\n"
        )
    );
    assert_eq!(
        current_named(out.as_deref().unwrap_or_default(), "badges").as_deref(),
        Some("new\n")
    );
}

/// Splicing is IDEMPOTENT, which is what lets `--check` be a diff: a
/// second render over its own output must change nothing.
#[test]
fn splicing_twice_changes_nothing_the_second_time() {
    let readme = "# t\n<!-- BEGIN badges -->\nold\n<!-- END badges -->\n";
    let once = splice_named(readme, "badges", "new\n").unwrap_or_default();
    assert_eq!(
        splice_named(&once, "badges", "new\n").as_deref(),
        Some(once.as_str())
    );
}

/// A block named in the generator but absent from the document is a
/// document that has not opted in -- never a rewrite of a file that did
/// not ask for one.
#[test]
fn a_block_whose_markers_are_absent_is_absent_not_stale() {
    let readme = "# t\n<!-- BEGIN badges -->\nx\n<!-- END badges -->\n";
    assert_eq!(current_named(readme, "graph-tree"), None);
    assert_eq!(splice_named(readme, "graph-tree", "x\n"), None);
}
