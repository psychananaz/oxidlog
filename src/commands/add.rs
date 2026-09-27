use crate::{
    content::parse_content,
    error::AppResult,
    storage::{self, config::Config, Journal},
};
use colored::Colorize;
use std::io::Write as _;

#[derive(clap::Args)]
pub struct AddArgs {
    pub content: String,
}

pub fn execute(journal: &mut Journal, args: AddArgs, config: &Config) -> AppResult<()> {
    let content = parse_content(args.content, config.journal_cfg.body_tags)?;
    let id = journal.add_entry(content.body, content.tags)?;
    storage::save_journal(journal)?;
    writeln!(
        std::io::stdout().lock(),
        "Entry {} added!",
        format!("#{id}").bold().green()
    )?;
    Ok(())
}
