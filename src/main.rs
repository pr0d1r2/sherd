//! `bbx` -- entry point only. Everything is in `bbx::cli` (`.:V41`).

fn main() -> std::process::ExitCode {
    bbx::cli::run()
}
