use colored::*;
use storage::load_config;

mod cli;
mod commands;
mod error;
mod storage;
mod utils;

fn main() {
    let config = match load_config() {
        Ok(config) => config,
        Err(e) => {
            eprintln!(
                "{} {}",
                "Error:".red().bold(),
                format!("Failed to load config - {}", e)
            );
            eprintln!(
                "{} Run 'xlog init' to create a new configuration",
                "Tip:".cyan().bold()
            );
            std::process::exit(1);
        }
    };

    if let Err(e) = cli::run(&config) {
        let error_type = match e {
            error::AppError::_InitError(_) => "Initialization",
            error::AppError::AddError(_) => "Add Entry",
            error::AppError::RemoveError(_) => "Remove Entry",
            error::AppError::EditError(_) => "Edit Entry",
            error::AppError::IoError(_) => "File System",
            error::AppError::Json(_) => "Data Format",
            error::AppError::TomlParseError(_) => "Config Parse",
            error::AppError::TomlSerializeError(_) => "Config Save",
            error::AppError::ExportError(_) => "Export",
            error::AppError::CommandError(_) => "Command",
            error::AppError::Other(_) => "Unknown",
        };

        eprintln!("\n{} {} Error", "Error:".red().bold(), error_type);
        eprintln!("{} {}", "Details:".yellow().bold(), e);

        // Provide helpful tips based on error type
        match e {
            error::AppError::IoError(_) => {
                eprintln!(
                    "\n{} Check file permissions and disk space",
                    "Tip:".cyan().bold()
                );
            }
            error::AppError::_InitError(_) => {
                eprintln!("\n{} Try running 'xlog init' again", "Tip:".cyan().bold());
            }
            error::AppError::Json(_) | error::AppError::TomlParseError(_) => {
                eprintln!(
                    "\n{} The journal file may be corrupted. Try backing up and reinitializing",
                    "Tip:".cyan().bold()
                );
            }
            _ => {}
        }

        std::process::exit(1);
    }
}
