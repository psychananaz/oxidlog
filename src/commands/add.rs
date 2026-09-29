use crate::{
    error::AppResult,
    storage::{self, Journal},
    utils::parse_content,
};
use colored::Colorize;
use std::io::Write as _;

#[derive(clap::Args)]
pub struct AddArgs {
    pub content: String,
}

pub fn execute(journal: &mut Journal, args: AddArgs) -> AppResult<()> {
    let content = parse_content(args.content)?;
    let id = journal.add_entry(content.body, content.tags)?;
    storage::save_journal(journal)?;
    writeln!(
        std::io::stdout().lock(),
        "Entry {} added!",
        format!("#{id}").bold().green()
    )?;
    Ok(())
}
