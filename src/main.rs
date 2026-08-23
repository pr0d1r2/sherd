//! `sherd` -- entry point only. Everything is in `sherd::cli` (`src/cli:V14`).

fn main() -> std::process::ExitCode {
    sherd::cli::run()
}
