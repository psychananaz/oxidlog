use crate::error::{JotError, JotResult};
use crate::storage;
use crate::storage::config::{Config, JournalConfig};
use dialoguer::{Confirm, Input, Select};
use std::path::PathBuf;

#[derive(clap::Args)]
pub struct InitArgs {
    /// Custom export directory path
    #[arg(short, long)]
    export_dir: Option<PathBuf>,
}

pub fn execute(args: InitArgs) -> JotResult<()> {
    if storage::journal_exists() {
        let proceed = Confirm::new()
            .with_prompt("A journal already exists. Do you want to overwrite it?")
            .default(false)
            .interact()
            .map_err(|e| JotError::_InitError(format!("Failed to get user confirmation: {}", e)))?;

        if !proceed {
            println!("Operation cancelled.");
            return Ok(());
        }
    }

    // Get export directory
    let export_dir = if let Some(dir) = args.export_dir {
        dir.to_string_lossy().into_owned()
    } else {
        let default_dir = "exports".to_string();
        Input::<String>::new()
            .with_prompt("Export directory path")
            .default(default_dir)
            .interact()
            .map_err(|e| JotError::_InitError(format!("Failed to get export directory: {}", e)))?
    };

    // Configure timestamp display
    let show_time = Confirm::new()
        .with_prompt("Show timestamps in entries?")
        .default(true)
        .interact()
        .map_err(|e| JotError::_InitError(format!("Failed to get timestamp preference: {}", e)))?;

    // Configure tag style
    let tag_options = vec!["Body tags (#tag in content)", "Separate tag field"];
    let tag_selection = Select::new()
        .with_prompt("How would you like to handle tags?")
        .items(&tag_options)
        .default(0)
        .interact()
        .map_err(|e| JotError::_InitError(format!("Failed to get tag preference: {}", e)))?;

    let new_config = Config {
        journal_cfg: JournalConfig {
            show_time,
            body_tags: tag_selection == 0,
            export_dir,
        },
    };

    storage::init_journal(&new_config)?;

    // Show success message with journal location
    let journal_path = storage::get_journal_path()?;
    println!("\n✨ Journal initialized successfully!");
    println!("📝 Location: {}", journal_path.display());
    println!("🚀 Run 'xlog add' to create your first entry");

    Ok(())
}
