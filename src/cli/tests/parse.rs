use super::*;
use clap::Parser;

fn parse(a: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("sherd").chain(a.iter().copied()))
}

fn cmd(a: &[&str]) -> Option<Cmd> {
    parse(a).ok().and_then(|c| c.cmd)
}

/// The grammar is internally consistent: clap asserts this at build time in
/// debug, and a conflicting short flag or group would panic on first use.
#[test]
fn the_grammar_is_well_formed() {
    command().debug_assert();
}

/// Every verb `USAGE` names is a subcommand, and the reverse -- the clap tree
/// and the usage text are two spellings of one list (`src/cli:V7`).
#[test]
fn every_usage_verb_is_a_subcommand_and_back() {
    let usage: Vec<&str> = super::super::USAGE
        .lines()
        .filter_map(|l| l.strip_prefix("  sherd "))
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    let tree = command();
    let names: Vec<&str> = tree
        .get_subcommands()
        .map(clap::Command::get_name)
        .filter(|n| *n != "help")
        .collect();
    for v in &usage {
        assert!(names.contains(v), "usage names `{v}`, grammar does not");
    }
    for n in &names {
        assert!(usage.contains(n), "grammar has `{n}`, usage does not");
    }
}

#[test]
fn flags_sit_anywhere_and_dirs_are_optional() {
    assert!(matches!(
        cmd(&["sync", "--check", "."]),
        Some(Cmd::Sync {
            check: true,
            dir: Some(_)
        })
    ));
    assert!(matches!(
        cmd(&["sync", ".", "--check"]),
        Some(Cmd::Sync {
            check: true,
            dir: Some(_)
        })
    ));
    assert!(matches!(
        cmd(&["sync"]),
        Some(Cmd::Sync {
            check: false,
            dir: None
        })
    ));
    assert!(matches!(
        cmd(&["init", "--stdout", "x"]),
        Some(Cmd::Init {
            stdout: true,
            dir: Some(_)
        })
    ));
}

#[test]
fn verbose_is_global_and_positional_agnostic() {
    assert!(parse(&["-v", "check"]).is_ok_and(|c| c.verbose));
    assert!(parse(&["check", "--verbose"]).is_ok_and(|c| c.verbose));
    assert!(parse(&["check"]).is_ok_and(|c| !c.verbose));
}

#[test]
fn mistakes_are_parse_errors_not_silent_fallbacks() {
    for bad in [
        &["lens"][..],
        &["outcome", "src/fed"],
        &["outcome", "src/fed", "probably"],
        &["graph", "--dot", "--tree"],
        &["debt", "--check", "--record"],
        &["plan", "--triage", "--milestone", "M1"],
        &["budget", "--formta", "json"],
        &["nope"],
    ] {
        assert!(parse(bad).is_err(), "{bad:?} must not parse");
    }
}

#[test]
fn a_mistyped_flag_is_suggested() {
    let Err(e) = parse(&["budget", "--formta", "json"]) else {
        unreachable!("a mistyped flag does not parse")
    };
    assert!(e.to_string().contains("--format"), "{e}");
}

#[test]
fn version_and_help_are_success_paths_with_the_crate_version() {
    for a in [&["--version"][..], &["-V"], &["--help"], &["-h"], &["help"]] {
        let Err(e) = parse(a) else {
            unreachable!("{a:?} is answered by clap, not parsed")
        };
        assert!(!e.use_stderr(), "{a:?} must print to stdout and exit 0");
    }
    let Err(e) = parse(&["--version"]) else {
        unreachable!("--version is answered by clap")
    };
    assert_eq!(
        e.to_string().trim(),
        format!("sherd {}", super::super::VERSION)
    );
}

#[test]
fn completions_and_man_page_can_be_generated() {
    for shell in ["bash", "zsh", "fish", "elvish", "powershell"] {
        assert_eq!(
            super::super::run_args(vec!["completions".into(), shell.into()]),
            std::process::ExitCode::SUCCESS,
            "{shell}"
        );
    }
    let mut script = Vec::new();
    clap_complete::generate(
        clap_complete::Shell::Bash,
        &mut command(),
        "sherd",
        &mut script,
    );
    let script = String::from_utf8_lossy(&script);
    assert!(script.contains("sherd"), "{script}");
    assert!(script.contains("budget"), "completes the verbs");

    let mut page = Vec::new();
    assert!(clap_mangen::Man::new(command()).render(&mut page).is_ok());
    let page = String::from_utf8_lossy(&page);
    assert!(page.contains(".TH sherd"), "{page}");
    assert_eq!(
        super::super::run_args(vec!["man".into()]),
        std::process::ExitCode::SUCCESS
    );
    assert_eq!(
        super::super::run_args(vec!["completions".into(), "tcsh".into()]),
        std::process::ExitCode::from(2)
    );
}
