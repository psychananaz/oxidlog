use crate::{
    content::parse_content,
    error::JotResult,
    storage::{self, config::Config, Journal},
};
use colored::Colorize;

#[derive(clap::Args)]
pub struct AddArgs {
    pub content: String,
}

pub fn execute(journal: &mut Journal, args: AddArgs, config: &Config) -> JotResult<()> {
    let content = parse_content(args.content, config.journal_cfg.body_tags)?;
    let id = journal.add_entry(content.body, content.tags)?;
    storage::save_journal(journal)?;
    println!("Entry {} added!", format!("#{id}").bold().green());
    Ok(())
}
