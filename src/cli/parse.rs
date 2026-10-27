//! The argv GRAMMAR, declared once (`src/cli:V21`).
//!
//! This replaces the hand-rolled scans -- `position(|a| a == "--format")`,
//! `args.get(1)`, `starts_with("--")` -- that each verb used to carry its own
//! copy of. A flag is now a field: where it may sit, whether it takes a value
//! and what values it accepts are the type's business, and a mistyped flag
//! gets a "did you mean" instead of being read as a directory.
//!
//! Exit codes are unchanged: clap's own usage errors exit 2, which is the
//! code `src/cli:V1` reserves for a malformed invocation.

use crate::lens;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};

/// `--format text|json`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Format {
    #[default]
    Text,
    Json,
}

impl Format {
    pub(super) fn is_json(self) -> bool {
        self == Self::Json
    }
}

/// `--depth rule|why|all`; [`lens::Depth`] without a clap dependency in `lens`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum Depth {
    #[default]
    Rule,
    Why,
    All,
}

impl From<Depth> for lens::Depth {
    fn from(d: Depth) -> Self {
        match d {
            Depth::Rule => Self::Rule,
            Depth::Why => Self::Why,
            Depth::All => Self::All,
        }
    }
}

/// The three words `outcome` records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Verdict {
    Kept,
    Reverted,
    Failed,
}

impl Verdict {
    pub(super) fn is_kept(self) -> bool {
        self == Self::Kept
    }
}

/// `--format`, shared by every verb with a JSON form.
#[derive(Args, Debug, Default)]
pub struct FormatArg {
    /// output form
    #[arg(long, value_enum, default_value_t)]
    pub format: Format,
}

/// The repository a verb acts on, for verbs that take no `[dir]` of their own.
///
/// `root_for` resolves the root from ANY directory argument (`src/cli:B7`), so
/// `sherd land ../other` and `sherd plan --triage <dir>` always named a
/// repository. The hand parser ignored the extra word; a grammar has to say
/// it is accepted.
#[derive(Args, Debug, Default)]
pub struct At {
    /// a directory inside the repository to act on
    pub at: Option<String>,
}

/// The shells `completions` can write for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Elvish,
    Powershell,
}

impl From<Shell> for clap_complete::Shell {
    fn from(s: Shell) -> Self {
        match s {
            Shell::Bash => Self::Bash,
            Shell::Zsh => Self::Zsh,
            Shell::Fish => Self::Fish,
            Shell::Elvish => Self::Elvish,
            Shell::Powershell => Self::PowerShell,
        }
    }
}

/// The whole command line. `override_help` keeps `--help` printing
/// [`super::USAGE`], the text the README's Commands table is rendered from;
/// each verb's own `--help` is generated from the fields below.
#[derive(Parser, Debug)]
#[command(
    name = "sherd",
    version,
    override_help = super::USAGE,
    disable_help_subcommand = false,
)]
pub struct Cli {
    /// dump every prompt and stream every reply
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

/// One variant per verb; the lowercased name IS the verb, and `sherd-dev`
/// reads this enum to check usage and `§I` name every one (`src/cli:V7`).
#[derive(Subcommand, Debug)]
pub enum Cmd {
    /// scaffold a SPEC.md, §F rows from child dirs
    Init {
        dir: Option<String>,
        /// print instead of writing
        #[arg(long)]
        stdout: bool,
    },
    /// token cost of every node, against the working budget
    Budget {
        dir: Option<String>,
        #[command(flatten)]
        format: FormatArg,
    },
    /// the context pack for one node
    Lens {
        dir: String,
        #[arg(long, value_enum, default_value_t)]
        depth: Depth,
    },
    /// the federation edges declared by a node
    Fed { dir: Option<String> },
    /// microlith structural check of every node
    Check {
        dir: Option<String>,
        #[command(flatten)]
        format: FormatArg,
    },
    /// lint ratchet: density & shape vs .lint-debt
    Debt {
        #[command(flatten)]
        at: At,
        #[arg(long, conflicts_with = "record")]
        check: bool,
        #[arg(long)]
        record: bool,
    },
    /// coverage floor vs .coverage
    Coverage {
        #[command(flatten)]
        at: At,
        #[arg(long, conflicts_with = "record")]
        check: bool,
        #[arg(long)]
        record: bool,
    },
    /// DAG + ids + ceilings + slice drift, one verdict
    Validate {
        #[command(flatten)]
        at: At,
        #[command(flatten)]
        format: FormatArg,
    },
    /// propose a federation split. writes nothing
    Split {
        dir: Option<String>,
        #[arg(long)]
        apply: bool,
    },
    /// the public types each node declares. writes nothing
    Seam { dir: Option<String> },
    /// the rounds a parallel build would run. writes nothing
    Wave { dir: Option<String> },
    /// migrate a single-file SPEC.md onto a federation
    Adopt {
        dir: String,
        /// the map file to apply
        #[arg(long, value_name = "FILE")]
        map: Option<String>,
        #[arg(long)]
        check: bool,
    },
    /// regenerate §N from §F. exit 1 if it wrote
    Sync {
        dir: Option<String>,
        #[arg(long)]
        check: bool,
    },
    /// which node owns a question. 0 hit · 2 miss · 3 ambiguous
    Route {
        query: String,
        #[command(flatten)]
        at: At,
    },
    /// mechanical checks on what a commit added (default HEAD)
    Review { rev: Option<String> },
    /// regenerate distilled slices from their sources
    Slice {
        #[command(flatten)]
        at: At,
        #[arg(long, conflicts_with = "list")]
        check: bool,
        #[arg(long)]
        list: bool,
    },
    /// record whether a node's work survived review
    Outcome {
        node: String,
        #[arg(value_enum)]
        verdict: Verdict,
        #[command(flatten)]
        at: At,
    },
    /// federation DAG, generated from §F
    Graph {
        #[arg(long, group = "form")]
        tree: bool,
        #[arg(long, group = "form")]
        table: bool,
        #[arg(long, group = "form")]
        dot: bool,
        /// the default rendering, accepted by name
        #[arg(long, group = "form")]
        mermaid: bool,
        #[command(flatten)]
        at: At,
    },
    /// next 3 steps, with what would invalidate each
    Plan {
        #[command(flatten)]
        at: At,
        #[command(flatten)]
        format: FormatArg,
        /// unmanaged rows, with a proposed home for each
        #[arg(long, conflicts_with = "milestone")]
        triage: bool,
        /// only the rows milestone M claims
        #[arg(long, value_name = "M")]
        milestone: Option<String>,
    },
    /// execute step 1 only, commit it to a run branch, stop
    Apply {
        #[command(flatten)]
        at: At,
        #[arg(long)]
        land: bool,
        #[arg(long)]
        push: bool,
    },
    /// fast-forward main to this run branch, if it earned it
    Land {
        #[command(flatten)]
        at: At,
        #[arg(long)]
        push: bool,
    },
    /// ask the endpoint from a node's lens pack
    Ask { dir: String, question: String },
    /// red -> judge -> green -> gate -> repair
    Tdd {
        dir: String,
        invariant: String,
        task: String,
    },
    /// the monolith arm: one call, whole repo
    Oneshot {
        dir: String,
        invariant: String,
        task: String,
    },
    /// print a shell completion script to stdout
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// print the man page (roff) to stdout
    Man,
}

/// The command tree, for completions and the man page.
#[must_use]
pub fn command() -> clap::Command {
    Cli::command()
}

#[cfg(test)]
#[path = "tests/parse.rs"]
mod tests;
