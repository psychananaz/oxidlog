pub mod input;

use crate::commands::{backup, init};
use crate::error::AppResult;
use crate::{commands, storage};
use clap::{Parser, Subcommand};

use commands::{add, edit, export, remove, search, view};

/// A command-line journaling tool for quick note-taking and organization
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new journal or reconfigure an existing one
    #[command(visible_alias = "i")]
    Init {
        #[clap(flatten)]
        args: init::InitArgs,
    },

    /// Add a new entry to your journal
    #[command(visible_alias = "new", visible_alias = "a")]
    Add {
        #[clap(flatten)]
        args: add::AddArgs,
    },

    /// Remove an entry from your journal
    #[command(visible_alias = "delete", visible_alias = "rm")]
    Remove {
        #[clap(flatten)]
        args: remove::RemoveArgs,
    },

    /// View journal entries
    #[command(visible_alias = "list", visible_alias = "ls")]
    View {
        #[clap(flatten)]
        args: view::ViewArgs,
    },

    /// Edit an existing journal entry
    #[command(visible_alias = "modify", visible_alias = "e")]
    Edit {
        #[clap(flatten)]
        args: edit::EditArgs,
    },

    /// Search through journal entries
    #[command(visible_alias = "find", visible_alias = "s")]
    Search {
        #[clap(flatten)]
        args: search::SearchArgs,
    },

    /// Export journal entries to various formats
    #[command(visible_alias = "dump", visible_alias = "ex")]
    Export {
        #[clap(flatten)]
        args: export::ExportArgs,
    },

    #[command(visible_alias = "b")]
    Backup {
        #[clap(flatten)]
        args: backup::BackupArgs,
    },
}

/// Parse CLI arguments before loading any files, so help and init work even
/// when the saved configuration is invalid.
pub fn run() -> AppResult<()> {
    let cli = Cli::parse();

    let name = match &cli.command {
        Commands::Init { .. } => "init",
        Commands::Add { .. } => "add",
        Commands::Remove { .. } => "remove",
        Commands::View { .. } => "view",
        Commands::Edit { .. } => "edit",
        Commands::Search { .. } => "search",
        Commands::Export { .. } => "export",
        Commands::Backup { .. } => "backup",
    };
    dispatch(cli.command).map_err(|error| error.in_command(name))
}

fn dispatch(command: Commands) -> AppResult<()> {
    match command {
        Commands::Init { args } => commands::init::execute(args),
        Commands::Add { args } => {
            let mut journal = storage::load_journal()?;
            commands::add::execute(&mut journal, args)
        }
        Commands::Remove { args } => {
            let mut journal = storage::load_journal()?;
            commands::remove::execute(&mut journal, args)
        }
        Commands::View { args } => {
            let journal = storage::load_journal()?;
            commands::view::execute(&journal, args, &storage::load_config()?)
        }
        Commands::Edit { args } => {
            let mut journal = storage::load_journal()?;
            commands::edit::execute(&mut journal, args)
        }
        Commands::Search { args } => {
            let journal = storage::load_journal()?;
            commands::search::execute(&journal, args, &storage::load_config()?)
        }
        Commands::Export { args } => {
            let journal = storage::load_journal()?;
            commands::export::execute(&journal, args, &storage::load_config()?)
        }
        Commands::Backup { args } => {
            let journal = storage::Journal::new(storage::get_journal_path()?);
            commands::backup::execute(&journal, args)
        }
    }
}
