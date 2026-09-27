use colored::Colorize;

use crate::error::{JotError, JotResult};
use crate::storage::config::Config;
use crate::storage::{self, Entry, Journal, Tag};

#[derive(clap::Args, Clone)]
pub struct AddArgs {
    pub content: String,
}

pub fn execute(journal: &mut Journal, args: AddArgs, config: &Config) -> JotResult<()> {
    let content = args.content.trim();
    if content.is_empty() {
        return Err(JotError::AddError("Entry cannot be empty".to_string()));
    }

    let tags = extract_tags(content);
    let body = extract_body(content, config);

    let entry = Entry::new(0, body, tags);
    let id = journal.add_entry(entry)?;
    storage::save_journal(journal)?;

    println!("Entry {} added!", format!("#{id}").bold().green());

    Ok(())
}

fn extract_tags(content: &str) -> Vec<Tag> {
    content
        .split_whitespace()
        .filter(|w| w.starts_with('#'))
        .map(|w| Tag::new(w[1..].to_string()))
        .collect()
}

fn extract_body(content: &str, config: &Config) -> String {
    if config.journal_cfg.body_tags {
        content
            .split_whitespace()
            .filter(|w| !w.starts_with('#'))
            .collect::<Vec<&str>>()
            .join(" ")
    } else {
        content.to_string()
    }
}
