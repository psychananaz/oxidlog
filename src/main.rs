use colored::*;

mod cli;
mod commands;
mod error;
mod storage;
mod utils;

fn main() -> std::process::ExitCode {
    use std::io::Write;
    match cli::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) if error.is_broken_pipe() => std::process::ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(
                std::io::stderr().lock(),
                "{} {error}",
                "Error:".red().bold()
            );
            std::process::ExitCode::FAILURE
        }
    }
}
