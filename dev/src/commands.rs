//! The README's Commands section, and the check that it names every verb.
//!
//! Two rules, one source. `sherd::cli::USAGE` is the text the binary prints, so
//! the section is rendered FROM it rather than beside it -- `.:B4`'s shape
//! applied to a list instead of a number. And the dispatch arms in
//! `src/cli/mod.rs` are compared AGAINST it, which is `src/cli:V7`: usage
//! names every verb that dispatches. `src/cli:B2` is what the missing half
//! cost -- `oneshot` was reachable, documented in the README and measured in
//! `§R30`, and absent from the one place a user looks.

/// One command line of the usage text: the verb, and the whole line.
fn usage_lines(usage: &str) -> Vec<(String, String)> {
    usage
        .lines()
        .filter_map(|l| {
            let t = l.strip_prefix("  sherd ")?;
            let verb = t.split_whitespace().next()?;
            (!verb.starts_with('-')).then(|| (verb.to_string(), l.to_string()))
        })
        .collect()
}

/// The verbs `run_args` actually dispatches, read from the source.
///
/// SCOPED to the top-level `match args.first()` and its closing brace, not
/// matched across the file. The unscoped version reported `--dot`, `--tree`,
/// `rule`, `why` and `all` as undocumented verbs: those are ARGUMENT arms of
/// nested matches, spelled identically. Same lesson as `V4` one file over --
/// scope tells things apart where a name list cannot, and the unscoped
/// version looked right until it ran.
///
/// A HEURISTIC over text either way, and it says so: a registry the
/// dispatcher iterates is the right answer and a bigger change than the
/// check that motivates it.
pub fn dispatched(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for line in source.lines() {
        if line.contains("match args.first()") {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line == "    }" {
            break;
        }
        // The outer match's own arms sit at EXACTLY this indent. A nested
        // match inside one of them -- `graph`'s `--dot`/`--table`/`--tree`
        // -- is indented further and is an ARGUMENT, not a verb. Scoping to
        // the match was not enough; the arms had to be scoped to its level.
        const ARM: &str = "        Some(";
        if !line.starts_with(ARM) {
            continue;
        }
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.split_once("Some(").map(|(_, r)| r) else {
            continue;
        };
        let rest = rest.trim_start_matches('&');
        let Some(inner) = rest.strip_prefix('"') else {
            continue;
        };
        let Some((verb, _)) = inner.split_once('"') else {
            continue;
        };
        if !verb.is_empty() && !out.contains(&verb.to_string()) {
            out.push(verb.to_string());
        }
    }
    out
}

/// A dispatch arm that is a FLAG about the binary, not a verb acting on a
/// repository.
///
/// `help` and its spellings are how usage is REACHED, so a usage text listing
/// itself would be noise rather than a contract. `--version` is the same
/// thing one field over: it answers what this binary IS. Both are excluded
/// from the two contracts below -- `usage_lines` and `specced` read VERB
/// lines, and neither the README's Commands table nor `§I` is a place a flag
/// belongs. `-v`/`--verbose` is absent from both for the same reason and
/// never reached here, because it is removed from argv before dispatch.
fn is_flag_not_verb(v: &str) -> bool {
    matches!(v, "help" | "--help" | "-h" | "--version" | "-V")
}

/// Verbs that dispatch and appear in no usage line (`src/cli:V7`).
pub fn undocumented(usage: &str, source: &str) -> Vec<String> {
    let named: Vec<String> =
        usage_lines(usage).into_iter().map(|(v, _)| v).collect();
    dispatched(source)
        .into_iter()
        .filter(|v| !is_flag_not_verb(v))
        .filter(|v| !named.contains(v))
        .collect()
}

/// The Commands section, rendered from the usage text.
#[must_use]
pub fn render(usage: &str) -> String {
    let mut s = String::from("| command | what it does |\n|---|---|\n");
    for (_, line) in usage_lines(usage) {
        let body = line.trim();
        // The usage text separates the invocation from its gloss with two or
        // more spaces. One space is inside an invocation (`sherd lens <dir>`),
        // so the split has to be on the RUN, not on the first space.
        let (inv, gloss) = body.split_once("  ").unwrap_or((body, ""));
        // A `|` inside a cell ENDS the cell. `sherd lens <dir> [--depth
        // rule|why|all]` renders as three broken columns unescaped, and the
        // table looked fine in the source and wrong on the page.
        s.push_str(&format!(
            "| `{}` | {} |\n",
            inv.trim().replace('|', "\\|"),
            gloss.trim().replace('|', "\\|")
        ));
    }
    s
}

#[cfg(test)]
#[path = "tests/commands.rs"]
mod tests;

/// A `§I` command line: the verb, and the rung it is promised for.
///
/// `- cmd: `sherd route "<query>"` → ... (0.3)` is a promise; the same line
/// without a rung is a claim that the verb ships today.
fn specced(spec: &str) -> Vec<(String, Option<String>)> {
    spec.lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("- cmd: `sherd ")?;
            let verb = rest.split([' ', '`']).next()?;
            let rung = l
                .rsplit_once(" (0.")
                .and_then(|(_, r)| r.strip_suffix(')'))
                .map(|r| format!("0.{r}"));
            (!verb.is_empty()).then(|| (verb.to_string(), rung))
        })
        .collect()
}

/// Where `§I` and the binary disagree, in both directions (`.:V115`).
///
/// `.:B17` is what one direction alone misses: usage-vs-dispatch had a runner
/// the same day this did not, and ten verbs shipped while the interface
/// section never named them. A reader trusts `§I`, so an interface promising
/// five absent verbs and hiding ten present ones is worse than none.
#[must_use]
pub fn interface_drift(spec: &str, source: &str) -> Vec<String> {
    let rows = specced(spec);
    let named: Vec<&String> = rows
        .iter()
        .filter(|(_, r)| r.is_none())
        .map(|(v, _)| v)
        .collect();
    let mut out: Vec<String> = dispatched(source)
        .into_iter()
        .filter(|v| !is_flag_not_verb(v))
        .filter(|v| !named.contains(&v))
        .map(|v| format!("`{v}` dispatches and §I does not name it"))
        .collect();
    let live = dispatched(source);
    out.extend(
        rows.iter()
            .filter(|(v, rung)| rung.is_none() && !live.contains(v))
            .map(|(v, _)| {
                format!("`{v}` is in §I, does not dispatch, and names no rung")
            }),
    );
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
#[path = "tests/interface.rs"]
mod interface_tests;
