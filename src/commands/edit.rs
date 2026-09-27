use crate::{
    content::{parse_content, parse_tags},
    error::{JotError, JotResult},
    storage::{self, config::Config, Journal},
    utils,
};
use colored::Colorize;

#[derive(clap::Args)]
pub struct EditArgs {
    pub id: usize,
}

pub fn execute(journal: &mut Journal, args: EditArgs, config: &Config) -> JotResult<()> {
    let entry = journal
        .get_entry(args.id)
        .ok_or_else(|| JotError::EditError(format!("Entry with ID {} not found", args.id)))?;
    println!("Editing entry: {}", entry.body);
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
        Some(parse_content(body_input, config.journal_cfg.body_tags)?)
    };
    let entry = journal
        .get_entry_mut(args.id)
        .ok_or_else(|| JotError::EditError(format!("Entry with ID {} not found", args.id)))?;
    if tags_input == "-" {
        entry.tags.clear();
    } else if !tags_input.is_empty() {
        entry.tags = parse_tags(&tags_input);
    }
    if let Some(content) = content {
        entry.body = content.body;
        for tag in content.tags {
            if !entry.tags.contains(&tag) {
                entry.tags.push(tag);
            }
        }
    }
    storage::save_journal(journal)?;
    println!("{}", "Entry updated!".green());
    Ok(())
}
