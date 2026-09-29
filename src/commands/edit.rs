use crate::{
    error::{AppError, AppResult},
    storage::{self, Journal},
    utils::{self, parse_content, parse_tags},
};
use colored::Colorize;
use std::io::Write as _;

#[derive(clap::Args)]
pub struct EditArgs {
    pub id: usize,
}

pub fn execute(journal: &mut Journal, args: EditArgs) -> AppResult<()> {
    let entry = journal
        .get_entry(args.id)
        .ok_or(AppError::EntryNotFound(args.id))?;
    writeln!(std::io::stdout().lock(), "Editing entry: {}", entry.body)?;
    let body_input = utils::get_input(&format!("Enter new content [{}]: ", entry.body))?;
    let tags = entry
        .tags
        .iter()
        .map(|tag| tag.name.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    let tags_input = utils::get_input(&format!(
        "Enter new tags [{tags}] (blank keeps, '-' clears): "
    ))?;
    let content = if body_input.is_empty() {
        None
    } else {
        Some(parse_content(body_input)?)
    };
    let tags = if tags_input == "-" {
        Some(Vec::new())
    } else if tags_input.is_empty() {
        None
    } else {
        Some(parse_tags(&tags_input))
    };
    journal.edit_entry(args.id, content, tags)?;
    storage::save_journal(journal)?;
    writeln!(std::io::stdout().lock(), "{}", "Entry updated!".green())?;
    Ok(())
}
