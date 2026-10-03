//! `wave`'s reading of shell scripts (`src/wave:V5`): which scripts a
//! script INVOKES, resolved lexically from what the file itself says.
//!
//! An invocation is `source X`, `. X`, `bash X`, `sh X`, `exec X`, `sh <X`,
//! or `X` at command position, where `X` ends in `.sh`. A path is resolved
//! from a literal, or from an expression the file itself makes:
//! `$(dirname "$0")` and `BASH_SOURCE` (the script's own dir),
//! `$(git rev-parse --show-toplevel)` (the root), `$(cd X && pwd)` (X), a
//! variable the same file assigned one of those, and `${VAR:-default}` (VAR
//! if known, else the default the file wrote). Anything else -- an argument,
//! the environment, a name built at run time -- is UNRESOLVED and handed back
//! as written, never guessed.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

/// What one invocation resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Target {
    /// A tracked script.
    Script(PathBuf),
    /// The call as written, which names no tracked script.
    Unresolved(String),
}

/// Where a script sits and what it may resolve against.
pub(super) struct Context<'a> {
    pub script: &'a Path,
    pub root: &'a Path,
    pub all: &'a [PathBuf],
}

type Vars = BTreeMap<String, PathBuf>;

/// Every invocation in a script's text, in order.
pub(super) fn calls(text: &str, cx: &Context<'_>) -> Vec<Target> {
    let mut vars = Vars::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let words = words(line);
        assign(&words, cx, &mut vars);
        for call in invoked(&words) {
            out.push(resolve(&call, cx, &vars));
        }
    }
    out
}

/// Record each leading `NAME=value` whose value names a path this can
/// resolve. Later assignments win, as they do when the script runs.
fn assign(words: &[String], cx: &Context<'_>, vars: &mut Vars) {
    let prefix = ["export", "readonly", "local"];
    for w in words.iter().filter(|w| !prefix.contains(&w.as_str())) {
        let Some((name, value)) = w.split_once('=') else {
            break;
        };
        if !is_name(name) {
            break;
        }
        match eval(&unquote(value), cx, vars) {
            Some(dir) => vars.insert(name.to_string(), dir),
            None => vars.remove(name),
        };
    }
}

fn is_name(s: &str) -> bool {
    let mut cs = s.chars();
    cs.next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && cs.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn is_assignment(w: &str) -> bool {
    w.split_once('=').is_some_and(|(n, _)| is_name(n))
}

/// The `.sh` words a line invokes, including inside `$( … )`.
fn invoked(words: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut at_command = true;
    let mut runner = false;
    // A shell anywhere in this command reads a `<script` redirect as its
    // program: `ssh host sh <x.sh` runs x.sh, `cat <x.sh` does not.
    let mut shell_in_command = false;
    for w in words {
        for inner in substitutions(w) {
            out.extend(invoked(&self::words(&inner)));
        }
        let bare = unquote(w);
        shell_in_command |= matches!(bare.as_str(), "sh" | "bash");
        if is_separator(&bare) {
            at_command = true;
            runner = false;
            shell_in_command = false;
            continue;
        }
        let script = bare.ends_with(".sh");
        if shell_in_command && script && bare.starts_with('<') {
            out.push(bare);
            continue;
        }
        let flag = runner && bare.starts_with('-');
        let prefix = at_command && is_assignment(w);
        if flag || prefix {
            continue;
        }
        let next_runner = at_command && !script && is_runner(&bare);
        if (at_command || runner) && script {
            out.push(bare);
        }
        runner = next_runner;
        at_command = false;
    }
    out
}

fn is_runner(w: &str) -> bool {
    matches!(w, "source" | "." | "bash" | "sh" | "exec")
}

fn is_separator(w: &str) -> bool {
    matches!(
        w,
        ";" | "&&"
            | "||"
            | "|"
            | "&"
            | "then"
            | "do"
            | "else"
            | "elif"
            | "if"
            | "while"
            | "until"
            | "!"
            | "time"
            | "{"
            | "("
    )
}

/// The bodies of the `$( … )` substitutions in one word.
fn substitutions(word: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = word;
    while let Some(i) = rest.find("$(") {
        let after = rest.get(i.saturating_add(2)..).unwrap_or("");
        match balanced(after, '(', ')') {
            Some((inner, tail)) => {
                out.push(inner.to_string());
                rest = tail;
            }
            None => break,
        }
    }
    out
}

/// Shell words: split on unquoted blanks outside `$( … )`, with `;`, `|`
/// and `&` runs as words of their own.
fn words(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    let mut depth = 0usize;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match (quote, c) {
            (Some(q), _) if c == q => {
                quote = None;
                cur.push(c);
            }
            (Some(_), _) => cur.push(c),
            (None, '\'' | '"') => {
                quote = Some(c);
                cur.push(c);
            }
            (None, '(') => {
                depth = depth.saturating_add(1);
                cur.push(c);
            }
            (None, ')') => {
                depth = depth.saturating_sub(1);
                cur.push(c);
            }
            (None, ' ' | '\t') if depth == 0 => flush(&mut cur, &mut out),
            (None, ';' | '|' | '&') if depth == 0 => {
                flush(&mut cur, &mut out);
                let mut op = c.to_string();
                while chars.next_if_eq(&c).is_some() {
                    op.push(c);
                }
                out.push(op);
            }
            _ => cur.push(c),
        }
    }
    flush(&mut cur, &mut out);
    out
}

fn flush(cur: &mut String, out: &mut Vec<String>) {
    if !cur.is_empty() {
        out.push(std::mem::take(cur));
    }
}

fn unquote(w: &str) -> String {
    w.chars().filter(|c| *c != '"' && *c != '\'').collect()
}

/// The path an expression names, if the file itself says: `$( … )` and
/// `$VAR` / `${VAR}` / `${VAR:-default}` at the front, then any literal
/// tail joined on. A tail that still holds a `$` is built at run time, and
/// is not guessed.
fn eval(expr: &str, cx: &Context<'_>, vars: &Vars) -> Option<PathBuf> {
    if expr.starts_with('/') && !expr.contains('$') {
        return Some(normalize(Path::new(expr)));
    }
    let (base, tail) = if let Some(rest) = expr.strip_prefix("$(") {
        let (inner, tail) = balanced(rest, '(', ')')?;
        (command_dir(inner.trim(), cx, vars)?, tail)
    } else if let Some(rest) = expr.strip_prefix("${") {
        let (inner, tail) = balanced(rest, '{', '}')?;
        (braced(inner, cx, vars)?, tail)
    } else {
        let rest = expr.strip_prefix('$')?;
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let (name, tail) = rest.split_at(end);
        (vars.get(name).cloned()?, tail)
    };
    if tail.contains('$') {
        return None;
    }
    Some(normalize(&base.join(tail.trim_start_matches('/'))))
}

/// `${NAME}`, or `${NAME:-default}`: NAME when this file assigned it, else
/// the default the file wrote.
fn braced(inner: &str, cx: &Context<'_>, vars: &Vars) -> Option<PathBuf> {
    match inner.split_once(":-") {
        Some((name, default)) => {
            vars.get(name).cloned().or_else(|| eval(default, cx, vars))
        }
        None => vars.get(inner).cloned(),
    }
}

/// The directory a command substitution prints, for the commands a script
/// uses to find itself: `dirname "$0"`, `cd X && pwd`, and the repo root.
fn command_dir(cmd: &str, cx: &Context<'_>, vars: &Vars) -> Option<PathBuf> {
    if let Some(arg) = cmd.strip_prefix("dirname ") {
        let own = arg.contains("$0") || arg.contains("BASH_SOURCE");
        return own.then(|| cx.script.parent().map(Path::to_path_buf))?;
    }
    if cmd.starts_with("git rev-parse --show-toplevel") {
        return Some(cx.root.to_path_buf());
    }
    // `cd "$X/.." 2>/dev/null && pwd`: the path is the first word only.
    let rest = cmd.strip_prefix("cd ")?;
    let arg = words(rest).into_iter().next()?;
    eval(&unquote(&arg), cx, vars)
}

/// The text up to the bracket that closes an already-open one, and what
/// follows it.
fn balanced(s: &str, open: char, close: char) -> Option<(&str, &str)> {
    let mut depth = 1usize;
    for (i, c) in s.char_indices() {
        if c == open {
            depth = depth.saturating_add(1);
        } else if c == close {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some((s.get(..i)?, s.get(i.saturating_add(1)..)?));
            }
        }
    }
    None
}

/// Lexical normalisation: `.` dropped, `..` pops. No filesystem access, so
/// the answer does not depend on what exists or where a symlink points.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// One invoked word, onto a tracked script or back as written.
fn resolve(call: &str, cx: &Context<'_>, vars: &Vars) -> Target {
    let call = call.trim_start_matches('<');
    let candidates: Vec<PathBuf> = if call.starts_with('$') {
        eval(call, cx, vars).into_iter().collect()
    } else if call.contains('$') {
        Vec::new()
    } else {
        let here = cx.script.parent().unwrap_or(cx.root);
        vec![normalize(&here.join(call)), normalize(&cx.root.join(call))]
    };
    candidates
        .into_iter()
        .find(|c| cx.all.contains(c))
        .map_or_else(|| Target::Unresolved(call.to_string()), Target::Script)
}
