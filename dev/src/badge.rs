//! The README badge block, rendered from the files that own each number.
//!
//! Every function here is a pure function of `&str` and returns what it
//! found, so the whole set is testable without a repository -- which is the
//! shape a fleet crate would need if these move out of this repo later, and
//! the shape `src/cli:V6` says a test must be handed rather than discover.
//!
//! A number typed into prose is true the day it is typed and quietly wrong
//! after. `.:B4` is that failure in six commit messages; a README is where it
//! would be read by strangers instead.

/// The value of a top-level `key = "value"` line, unquoted.
///
/// Deliberately NOT a TOML parser. It reads keys that live at the top of
/// `[package]` before any nested table, which is where `edition` and
/// `rust-version` are and where they will stay -- and the crate that could
/// parse this properly is a dependency this repo does not want for two
/// strings.
pub fn manifest_value(manifest: &str, key: &str) -> Option<String> {
    manifest
        .lines()
        .map(str::trim_end)
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(" = "))
        .map(|v| v.trim_matches('"').to_string())
}

/// Count of DIRECT dependencies: the keys of `[dependencies]`.
///
/// A dependency is a KEY, not a line. Counting lines is wrong twice over and
/// the siblings hit both: comment lines inflate the count, and a formatter
/// wrapping one long entry across several lines inflates it again the moment
/// a dep grows a `features` array. `itok` in this manifest is exactly that
/// shape, so the naive count would read 6 for 4 dependencies.
pub fn direct_dependencies(manifest: &str) -> usize {
    let mut in_deps = false;
    let mut n: usize = 0;
    let mut depth: i32 = 0;
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') && depth == 0 {
            in_deps = t == "[dependencies]";
            continue;
        }
        if !in_deps {
            continue;
        }
        // A wrapped entry is still one key: only count while no earlier
        // entry is still open across lines.
        if depth == 0 && !t.starts_with('#') && t.contains('=') {
            n = n.saturating_add(1);
        }
        depth = depth
            .saturating_add(count_of(t, '{'))
            .saturating_add(count_of(t, '['))
            .saturating_sub(count_of(t, '}'))
            .saturating_sub(count_of(t, ']'));
    }
    n
}

fn count_of(s: &str, c: char) -> i32 {
    i32::try_from(s.matches(c).count()).unwrap_or(i32::MAX)
}

/// A ratchet file's single number: `lines 90.58`, `total 270`.
pub fn ratchet(text: &str, key: &str) -> Option<String> {
    text.lines()
        .find_map(|l| l.strip_prefix(key)?.strip_prefix(' '))
        .map(str::trim)
        .map(str::to_string)
}

/// A percentage TRUNCATED to one decimal.
///
/// Coverage is platform-dependent -- the siblings measured one tree at 98.04
/// on macOS and 98.06 on ubuntu -- so a two-decimal badge is a number no
/// single machine reproduces. Truncation rather than rounding, because
/// rounding sends those two values to different tenths and understating is
/// the safe direction for a floor.
pub fn truncate_tenth(value: &str) -> Option<String> {
    let n: f64 = value.parse().ok()?;
    let t = (n * 10.0).floor() / 10.0;
    Some(format!("{t:.1}"))
}

/// The `rev` of one named node in `flake.lock`, short.
///
/// Named rather than first-found: this lock carries `nixpkgs`, `nix-hk` and
/// `nixpkgs-lock`, and taking the first `rev` in the file would badge
/// whichever node happens to sort first.
pub fn locked_rev(lock: &str, node: &str) -> Option<String> {
    let mut inside = false;
    for line in lock.lines() {
        let t = line.trim();
        if t.starts_with(&format!("\"{node}\":")) {
            inside = true;
        }
        if inside && t.starts_with("\"rev\":") {
            let rev: String = t
                .trim_start_matches("\"rev\":")
                .trim()
                .trim_matches(|c| c == '"' || c == ',')
                .chars()
                .take(7)
                .collect();
            return Some(rev);
        }
    }
    None
}

/// How many steps the gate declares.
///
/// Counted INSIDE the `local fast` and `local all` mappings only. A flat
/// count of `["name"]` reads 28 rather than 23, because it also counts the
/// four hook entries and the env block -- and those cannot be excluded by
/// NAME, because `check` is both a hook and a real step here. Scope tells
/// them apart; a name list would have been wrong and looked right.
pub fn gate_steps(pkl: &str) -> usize {
    let mut inside = false;
    let mut n: usize = 0;
    for line in pkl.lines() {
        if line.starts_with("local fast") || line.starts_with("local all") {
            inside = true;
        } else if line.starts_with('}') {
            inside = false;
        } else if inside && line.starts_with("  [\"") {
            n = n.saturating_add(1);
        }
    }
    n
}

/// The platforms CI actually gates, from the workflow's matrix.
///
/// Read from `ci.yml` rather than from `flake.nix` on purpose. The flake
/// declares four systems and CI runs three: generating from the flake would
/// badge `x86_64-darwin` as supported when no runner has ever built it,
/// which is a platform claim nothing checks.
pub fn ci_platforms(workflow: &str) -> Vec<(String, String)> {
    let Some(list) = workflow
        .lines()
        .find_map(|l| l.trim().strip_prefix("os: [")?.strip_suffix(']'))
    else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for runner in list.split(',').map(str::trim) {
        let (os, vendors): (&str, &[&str]) = if runner.starts_with("macos") {
            ("macos", &["arm"])
        } else if runner.contains("arm") {
            ("linux", &["arm"])
        } else if runner.starts_with("ubuntu") {
            ("linux", &["intel", "amd"])
        } else {
            continue;
        };
        for v in vendors {
            out.push(((*v).to_string(), os.to_string()));
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Everything the block reports, gathered by the caller from real files.
pub struct Facts {
    pub edition: String,
    pub msrv: String,
    pub deps: usize,
    pub gate_steps: usize,
    pub coverage_floor: String,
    pub lint_debt: String,
    pub nodes: usize,
    pub nixpkgs: String,
    pub platforms: Vec<(String, String)>,
}

const SHIELD: &str = "https://img.shields.io/badge";

/// Render the block. No CI, crates.io or docs.rs badge: `bbx-cli` is
/// unpublished and the GitHub repository does not exist yet, so each would
/// render a broken image or a green tick for a run nobody made. They land
/// with the publish.
pub fn render(f: &Facts) -> String {
    let mut s = String::new();
    let (ed, msrv, deps) = (&f.edition, &f.msrv, f.deps);
    s.push_str(&format!(
        "[![License: MIT]({SHIELD}/license-MIT-blue.svg)](LICENSE)\n\
         [![edition {ed}]({SHIELD}/edition-{ed}-000000?logo=rust&logoColor=white)](Cargo.toml)\n\
         [![MSRV {msrv}]({SHIELD}/MSRV-{msrv}-000000?logo=rust&logoColor=white)](Cargo.toml)\n\
         [![direct dependencies {deps}]({SHIELD}/direct_dependencies-{deps}-brightgreen)](docs/THIRD-PARTY-NOTICES.md)\n\
         [![unsafe forbidden]({SHIELD}/unsafe-forbidden-brightgreen)](Cargo.toml)\n\n"
    ));
    let (steps, cov, debt, nodes) =
        (f.gate_steps, &f.coverage_floor, &f.lint_debt, f.nodes);
    s.push_str(&format!(
        "[![gate hk]({SHIELD}/gate-hk-6E4AFF)](hk.pkl)\n\
         [![gate steps {steps}]({SHIELD}/gate_steps-{steps}-6E4AFF)](hk.pkl)\n\
         [![coverage floor {cov}%]({SHIELD}/coverage_floor-%E2%89%A5{cov}%25-brightgreen)](.coverage)\n\
         [![lint debt {debt}]({SHIELD}/lint_debt-%E2%89%A4{debt}-orange)](.lint-debt)\n\
         [![federated nodes {nodes}]({SHIELD}/federated_nodes-{nodes}-6E4AFF)](SPEC.md)\n\n"
    ));
    let rev = &f.nixpkgs;
    s.push_str(&format!(
        "[![nix flake]({SHIELD}/nix-flake-5277C3?logo=nixos&logoColor=white)](flake.nix)\n\
         [![nixpkgs {rev}]({SHIELD}/nixpkgs-{rev}-5277C3?logo=nixos&logoColor=white)](flake.lock)\n"
    ));
    for (vendor, os) in &f.platforms {
        s.push_str(&format!(
            "[![{vendor} {os}]({SHIELD}/{os}-5277C3?logo={vendor}&logoColor=white)](.github/workflows/ci.yml)\n"
        ));
    }
    s.push_str(&format!(
        "\n[![built with Claude Code]({SHIELD}/built_with-Claude_Code-D97757)](https://claude.com/claude-code)\n\
         [![built with Opus 5]({SHIELD}/built_with-Opus_5-D97757)](https://www.anthropic.com/claude)\n\
         [![built with SDD]({SHIELD}/built_with-spec--driven_development-D97757)](SPEC.md)\n"
    ));
    s
}

/// The markers around one generated block. NAMED, because the README carries
/// four of them now -- the badges and the three `bbx graph` renderings -- and
/// a single unnamed pair could only ever guard one.
#[must_use]
pub fn markers(name: &str) -> (String, String) {
    (
        format!("<!-- BEGIN {name} -->"),
        format!("<!-- END {name} -->"),
    )
}

/// What the README currently carries between one block's markers, if both are
/// there.
pub fn current_named(readme: &str, name: &str) -> Option<String> {
    let (begin, end) = markers(name);
    let after = readme.split_once(&begin)?.1;
    let (block, _) = after.split_once(&end)?;
    Some(block.trim_start_matches('\n').to_string())
}

/// The README with one block replaced. `None` when a marker is missing --
/// which is a document that has not opted in, not a failure to report as
/// staleness.
pub fn splice_named(readme: &str, name: &str, block: &str) -> Option<String> {
    let (begin, end) = markers(name);
    let (head, rest) = readme.split_once(&begin)?;
    let (_, tail) = rest.split_once(&end)?;
    Some(format!("{head}{begin}\n{block}{end}{tail}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "\
[package]
name = \"bbx-cli\"
edition = \"2024\"
rust-version = \"1.95\"

[[bin]]
name = \"bbx\"
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
        assert_eq!(
            manifest_value(MANIFEST, "edition").as_deref(),
            Some("2024")
        );
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

    #[test]
    fn the_named_lock_node_is_the_one_read() {
        let lock = "\
{
  \"nodes\": {
    \"nix-hk\": {
      \"locked\": {
        \"rev\": \"aaaaaaaaaaaaaaaaaaaa\"
      }
    },
    \"nixpkgs\": {
      \"locked\": {
        \"rev\": \"9f78f44bbbbbbbbbbbbb\"
      }
    }
  }
}
";
        assert_eq!(locked_rev(lock, "nixpkgs").as_deref(), Some("9f78f44"));
        assert_eq!(locked_rev(lock, "nix-hk").as_deref(), Some("aaaaaaa"));
        assert_eq!(locked_rev(lock, "absent"), None);
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
        let yml =
            "        os: [ubuntu-latest, ubuntu-24.04-arm, macos-latest]\n";
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
            lint_debt: "270".to_string(),
            nodes: 16,
            nixpkgs: "9f78f44".to_string(),
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
            "270",
            "federated_nodes-16",
            "9f78f44",
            "logo=arm",
            "unsafe-forbidden",
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
            current_named(out.as_deref().unwrap_or_default(), "badges")
                .as_deref(),
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
}

/// Every file the block is rendered from, read by the CALLER.
///
/// The reason this struct exists is `src/cli:V6`: a function that opens its
/// own inputs can only be tested against a real repository, and the only
/// repository lying around is the one under development. Handed the text, the
/// whole decision is testable from a string literal.
pub struct Sources {
    pub manifest: String,
    pub coverage: String,
    pub debt: String,
    pub pkl: String,
    pub lock: String,
    pub workflow: String,
}

/// What running the generator concluded. `Stale` carries the difference so a
/// refusal names what is wrong rather than only that something is.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Fresh,
    Wrote(String),
    Stale(Vec<String>),
    NoMarkers,
}

/// Gather every fact, or name the owner that had nothing to say. A value with
/// no owner is an ERROR rather than a default (V1).
///
/// # Errors
/// Any owning file missing the value it owns.
pub fn facts(s: &Sources, nodes: usize) -> Result<Facts, String> {
    let missing = |what: &str| format!("bbx-dev: no {what} to read");
    let lines = ratchet(&s.coverage, "lines")
        .ok_or_else(|| missing("`lines` row in .coverage"))?;
    Ok(Facts {
        edition: manifest_value(&s.manifest, "edition")
            .ok_or_else(|| missing("`edition` in Cargo.toml"))?,
        msrv: manifest_value(&s.manifest, "rust-version")
            .ok_or_else(|| missing("`rust-version` in Cargo.toml"))?,
        deps: direct_dependencies(&s.manifest),
        gate_steps: gate_steps(&s.pkl),
        coverage_floor: truncate_tenth(&lines)
            .ok_or_else(|| missing("a number in .coverage's `lines` row"))?,
        lint_debt: ratchet(&s.debt, "total")
            .ok_or_else(|| missing("`total` row in .lint-debt"))?,
        nodes,
        nixpkgs: locked_rev(&s.lock, "nixpkgs")
            .ok_or_else(|| missing("a `nixpkgs` node in flake.lock"))?,
        platforms: ci_platforms(&s.workflow),
    })
}

/// Every generated block in the document, by marker name.
pub type Blocks = Vec<(String, String)>;

/// Compare each generated block against what the README carries, and say what
/// should happen for the document as a whole.
///
/// One pass over all blocks rather than one call per block, because a
/// document is either current or it is not: reporting the badges fresh while
/// the diagram is four nodes behind is the half-truth that let the
/// Architecture section claim it could not drift while it had (`.:B16`).
///
/// # Errors
/// Never -- the signature mirrors [`run`] so a caller handles one shape.
pub fn apply(readme: &str, blocks: &Blocks, check_only: bool) -> Outcome {
    let mut next = readme.to_string();
    let mut diff = Vec::new();
    let mut missing = false;
    for (name, want) in blocks {
        let Some(have) = current_named(&next, name) else {
            missing = true;
            continue;
        };
        if &have == want {
            continue;
        }
        if check_only {
            diff.push(format!("stale: {name}"));
            diff.extend(
                want.lines()
                    .filter(|l| !have.contains(*l) && !l.trim().is_empty())
                    .take(3)
                    .map(|l| format!("  want: {l}")),
            );
            diff.extend(
                have.lines()
                    .filter(|l| !want.contains(*l) && !l.trim().is_empty())
                    .take(3)
                    .map(|l| format!("  have: {l}")),
            );
            continue;
        }
        match splice_named(&next, name, want) {
            Some(s) => next = s,
            None => missing = true,
        }
    }
    if missing {
        return Outcome::NoMarkers;
    }
    if !diff.is_empty() {
        return Outcome::Stale(diff);
    }
    if next == readme {
        return Outcome::Fresh;
    }
    Outcome::Wrote(next)
}

#[cfg(test)]
mod apply_tests {
    use super::*;

    fn doc(badges: &str, graph: &str) -> String {
        format!(
            "# t\n\n<!-- BEGIN badges -->\n{badges}<!-- END badges -->\n\ntext\n\n<!-- BEGIN graph -->\n{graph}<!-- END graph -->\n"
        )
    }

    fn blocks(badges: &str, graph: &str) -> Blocks {
        vec![
            ("badges".to_string(), badges.to_string()),
            ("graph".to_string(), graph.to_string()),
        ]
    }

    #[test]
    fn a_document_where_every_block_matches_is_fresh() {
        let d = doc("A\n", "B\n");
        assert_eq!(apply(&d, &blocks("A\n", "B\n"), true), Outcome::Fresh);
        assert_eq!(apply(&d, &blocks("A\n", "B\n"), false), Outcome::Fresh);
    }

    /// The failure `.:B16` records: one block current, another four nodes
    /// behind. A per-block checker that stopped at the first fresh one would
    /// have reported this document clean for weeks.
    #[test]
    fn a_stale_second_block_is_reported_even_when_the_first_is_fresh() {
        let d = doc("A\n", "OLD\n");
        let Outcome::Stale(diff) = apply(&d, &blocks("A\n", "NEW\n"), true)
        else {
            unreachable!("a stale block must be reported")
        };
        assert!(diff.iter().any(|l| l == "stale: graph"));
        assert!(!diff.iter().any(|l| l == "stale: badges"));
    }

    #[test]
    fn writing_replaces_every_stale_block_in_one_pass() {
        let d = doc("OLD\n", "OLD\n");
        let Outcome::Wrote(next) = apply(&d, &blocks("A\n", "B\n"), false)
        else {
            unreachable!("two stale blocks must be rewritten")
        };
        assert_eq!(next, doc("A\n", "B\n"));
        assert_eq!(apply(&next, &blocks("A\n", "B\n"), false), Outcome::Fresh);
    }

    #[test]
    fn a_block_whose_markers_are_absent_is_named_as_such() {
        let d = "# t\n\n<!-- BEGIN badges -->\nA\n<!-- END badges -->\n";
        assert_eq!(apply(d, &blocks("A\n", "B\n"), true), Outcome::NoMarkers);
    }
}
