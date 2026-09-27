use crate::{
    error::JotResult,
    storage::{config::Config, Journal, Tag},
    utils::{self, TagMatch},
};

// TODO: add regex search
#[derive(clap::Args, Clone)]
pub struct SearchArgs {
    pub query: String,
    #[clap(long, value_delimiter = ' ')]
    pub tags: Vec<String>,
    #[clap(long, value_parser = utils::parse_date)]
    pub from: Option<chrono::NaiveDate>,
    #[clap(long, value_parser = utils::parse_date)]
    pub to: Option<chrono::NaiveDate>,
    #[clap(short, long)]
    pub fuzzy: bool,
    #[clap(short, long)]
    pub all: bool,
    #[clap(short, long)]
    pub case_sensitive: bool,
}

fn highlight(body: &str, ranges: &[std::ops::Range<usize>]) -> String {
    use colored::Colorize;
    let mut result = String::new();
    let mut cursor = 0;
    for range in ranges {
        result.push_str(&body[cursor..range.start]);
        result.push_str(&body[range.clone()].on_green().to_string());
        cursor = range.end;
    }
    result.push_str(&body[cursor..]);
    result
}

pub fn execute(journal: &Journal, args: SearchArgs, config: &Config) -> JotResult<()> {
    let dates = utils::DateRange::new(args.from, args.to)?;
    let query = crate::matching::TextQuery::new(args.query, args.case_sensitive, args.fuzzy);
    let tags: Vec<_> = args.tags.into_iter().map(Tag::new).collect();
    let found: Vec<String> = journal
        .get_entries()
        .iter()
        .filter_map(|entry| {
            let mode = if args.all {
                TagMatch::All
            } else {
                TagMatch::Any
            };
            if !dates.contains(entry.date) || !utils::do_tags_match(&tags, &entry.tags, mode) {
                return None;
            }
            let ranges = query.find(&entry.body)?;
            let body = highlight(&entry.body, &ranges);
            Some(utils::format_entry_with_body(
                entry,
                config.journal_cfg.clone(),
                &body,
            ))
        })
        .collect();
    if found.is_empty() {
        println!("No entries found.");
    } else {
        println!("{} entries found", found.len());
        for entry in found {
            println!("{entry}");
        }
    }
    Ok(())
}
