//! `sherd` -- entry point only. Everything is in `sherd::cli` (`.:V41`).

fn main() -> std::process::ExitCode {
    sherd::cli::run()
}
