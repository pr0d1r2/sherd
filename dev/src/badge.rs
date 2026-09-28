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

/// What `flake.lock` records for one input: the rev it pins, when that rev
/// was last modified, and the release branch it was taken from.
pub struct Locked {
    pub rev: String,
    pub date: String,
    pub release: Option<String>,
}

/// Read one NAMED node out of `flake.lock`.
///
/// The node HEADER is matched, not any line beginning with the name, and that
/// distinction is the whole function. `nix-hk`'s `inputs` block contains the
/// line `"nixpkgs": [`, which begins with `"nixpkgs":` exactly as its own node
/// header does -- so a prefix match entered the wrong block and returned the
/// first `rev` it then found, which belonged to `nix-hk`. The badge named
/// nix-hk's commit as nixpkgs' for as long as the generator existed
/// (`dev:B1`).
///
/// A header is `"name": {` and an inputs entry is `"name": [` or
/// `"name": "..."`, so the trailing brace is what tells them apart. The block
/// ends at the next line indented as a sibling header, which bounds the
/// search to the node actually asked for.
pub fn locked(lock: &str, node: &str) -> Option<Locked> {
    let header = format!("\"{node}\": {{");
    let mut inside = false;
    let mut indent = 0;
    let (mut rev, mut stamp, mut release) = (None, None, None);
    for line in lock.lines() {
        let t = line.trim();
        if !inside {
            if t == header {
                inside = true;
                indent = line.len().saturating_sub(t.len());
            }
            continue;
        }
        // A sibling header at the same indent ends this node's block.
        let this_indent = line.len().saturating_sub(t.len());
        if this_indent == indent && t.ends_with("{") && t != header {
            break;
        }
        if let Some(v) = field(t, "rev") {
            rev.get_or_insert(v.chars().take(7).collect::<String>());
        }
        if let Some(v) = field(t, "lastModified") {
            stamp.get_or_insert(v);
        }
        if let Some(v) = field(t, "ref") {
            release.get_or_insert(v);
        }
    }
    Some(Locked {
        rev: rev?,
        date: stamp.and_then(|s| s.parse().ok()).map(civil_date)?,
        // `nixos-26.05` is a branch name; the release is what a reader knows
        // it by. `None` when the input is not pinned to a branch at all,
        // which is every input taken from a bare repository.
        release: release.map(|r| {
            r.trim_start_matches("nixos-")
                .trim_start_matches("release-")
                .to_string()
        }),
    })
}

fn field(line: &str, key: &str) -> Option<String> {
    let rest = line.trim().strip_prefix(&format!("\"{key}\":"))?;
    Some(
        rest.trim()
            .trim_end_matches(',')
            .trim_matches('"')
            .to_string(),
    )
}

/// A unix timestamp as `YYYY-MM-DD`, UTC.
///
/// Hinnant's civil-from-days, which is exact for every date this will ever
/// see and is twenty lines. A date crate for one format string would be a
/// runtime dependency in a repository whose §C counts them -- and this crate
/// ships to nobody, so the dependency would be pure cost.
#[must_use]
#[allow(
    clippy::arithmetic_side_effects,
    reason = "Hinnant's algorithm is exact integer arithmetic over a bounded \
              domain: every intermediate is derived from a day count, and an \
              i64 epoch cannot drive any of them near an i64 bound -- the \
              largest, `z * 400`, stays under 2^40 for every timestamp a \
              flake.lock can hold. Rewriting nine operations as checked ones \
              would obscure a published algorithm to satisfy a lint about \
              overflow that cannot occur here."
)]
pub fn civil_date(epoch_secs: i64) -> String {
    let days = epoch_secs.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
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
    pub nixpkgs: Locked,
    pub platforms: Vec<(String, String)>,
}

const SHIELD: &str = "https://img.shields.io/badge";

/// One field of a shields.io static badge, escaped.
///
/// The path is `/badge/<label>-<message>-<colour>`, so a literal dash inside
/// a field is a FIELD SEPARATOR unless doubled, and a literal underscore is a
/// space unless doubled. `2026-08-12` would render as three broken fields
/// without this, and the URL looks correct in the source either way.
#[must_use]
pub fn shield_text(raw: &str) -> String {
    raw.replace('_', "__").replace('-', "--")
}

/// Render the block. No CI, crates.io or docs.rs badge: `sherd` is
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
         [![lint debt {debt}/KLoC]({SHIELD}/lint_debt-%E2%89%A4{debt}%2FKLoC-orange)](.lint-debt)\n\
         [![federated nodes {nodes}]({SHIELD}/federated_nodes-{nodes}-6E4AFF)](SPEC.md)\n\n"
    ));
    // `26.05 (2026-08-12 - 9f78f44)`: the release a reader knows it by, the
    // day that rev was last modified, and the rev itself. A bare short sha
    // says which commit and nothing about which nixpkgs.
    //
    // shields.io reads `-` as a field separator, so every literal dash in a
    // label is doubled and spaces are underscores. The parentheses survive
    // as they are.
    let np = &f.nixpkgs;
    let release = np.release.as_deref().unwrap_or("unpinned");
    let label = format!(
        "{}_({}_--_{})",
        shield_text(release),
        shield_text(&np.date),
        np.rev
    );
    let shown = format!("{release} ({} - {})", np.date, np.rev);
    s.push_str(&format!(
        "[![nix flake]({SHIELD}/nix-flake-5277C3?logo=nixos&logoColor=white)](flake.nix)\n\
         [![nixpkgs {shown}]({SHIELD}/nixpkgs-{label}-5277C3?logo=nixos&logoColor=white)](flake.lock)\n"
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
/// four of them now -- the badges and the three `sherd graph` renderings -- and
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
#[path = "tests/badge.rs"]
mod tests;

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
    let missing = |what: &str| format!("sherd-dev: no {what} to read");
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
        lint_debt: ratchet(&s.debt, "density")
            .ok_or_else(|| missing("`density` row in .lint-debt"))?,
        nodes,
        nixpkgs: locked(&s.lock, "nixpkgs")
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
#[path = "tests/badge_apply.rs"]
mod apply_tests;
