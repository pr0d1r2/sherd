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

/// Verbs that dispatch and appear in no usage line (`src/cli:V7`).
///
/// `help` and its flag spellings are excluded: they are how usage is REACHED,
/// so a usage text listing itself would be noise rather than a contract.
pub fn undocumented(usage: &str, source: &str) -> Vec<String> {
    let named: Vec<String> =
        usage_lines(usage).into_iter().map(|(v, _)| v).collect();
    dispatched(source)
        .into_iter()
        .filter(|v| !matches!(v.as_str(), "help" | "--help" | "-h"))
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
mod tests {
    use super::*;

    const USAGE: &str = "\
sherd -- a tool

  sherd budget [dir]     token cost of every node
  sherd lens <dir> [--depth rule|why|all]  the context pack for one node

  -v, --verbose        dump every prompt

exit: 0 clean · 1 violation · 2 usage";

    const DISPATCH: &str = r#"
    let x = match depth {
        Some("rule") => Depth::Rule,
    };
    match args.first().map(String::as_str) {
        Some("budget") => budget(&root, arg_dir(&args, &root)),
        Some("lens") => match (args.get(1), depth_arg(&args)) {
        Some("oneshot") => match (args.get(1), args.get(2)) {
            Some("--nested") => nested(),
        },
        Some("help" | "--help" | "-h") => { print!("{USAGE}"); }
    }
    match graph_flag {
        Some("--dot") => dot(),
    }
"#;

    #[test]
    fn a_usage_line_yields_its_verb_and_a_flag_line_does_not() {
        let got: Vec<String> =
            usage_lines(USAGE).into_iter().map(|(v, _)| v).collect();
        assert_eq!(got, vec!["budget", "lens"]);
    }

    /// SCOPED: the arms before and after the dispatch match are argument
    /// matches spelled identically, and the unscoped version reported
    /// `rule` and `--dot` as undocumented verbs.
    #[test]
    fn dispatch_verbs_come_from_the_dispatch_match_only() {
        let got = dispatched(DISPATCH);
        assert!(got.contains(&"budget".to_string()));
        assert!(got.contains(&"lens".to_string()));
        assert!(got.contains(&"oneshot".to_string()));
        assert!(!got.contains(&"rule".to_string()), "an argument arm before");
        assert!(!got.contains(&"--dot".to_string()), "and one after");
        assert!(
            !got.contains(&"--nested".to_string()),
            "and one nested INSIDE a dispatch arm, which scoping to the \
             match alone did not exclude"
        );
    }

    /// B2 exactly: a verb that dispatches and is in no usage line. This is
    /// the assertion that would have caught it the day it landed.
    #[test]
    fn a_verb_that_dispatches_and_is_not_documented_is_reported() {
        assert_eq!(undocumented(USAGE, DISPATCH), vec!["oneshot"]);
    }

    #[test]
    fn help_is_not_reported_as_undocumented() {
        let dispatch = "        Some(\"help\" | \"--help\" | \"-h\") => {}\n";
        assert!(undocumented(USAGE, dispatch).is_empty());
    }

    #[test]
    fn every_documented_verb_reaches_the_table() {
        let table = render(USAGE);
        assert!(table.starts_with("| command | what it does |\n|---|---|\n"));
        assert!(
            table.contains(
                "| `sherd budget [dir]` | token cost of every node |"
            )
        );
        assert!(
            table.contains(
                "| `sherd lens <dir> [--depth rule\\|why\\|all]` | the context pack for one node |"
            ),
            "a pipe inside a cell is escaped, or the table renders broken: {table}"
        );
        assert!(!table.contains("--verbose"));
    }
}
