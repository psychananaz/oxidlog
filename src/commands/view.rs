use std::io::{self, Write};

use crate::{
    error::{AppError, AppResult},
    storage::{config::Config, Entry, Journal, Tag},
    utils::{self, DateRange, DisplayOptions, EntryFilter, TagMatch},
};

#[derive(clap::Args)]
pub struct ViewArgs {
    /// ID of the specific entry to view
    #[arg(conflicts_with_all = ["recent", "from", "to", "tags", "all"])]
    pub id: Option<usize>,
    /// View entries starting from this date
    #[clap(short, long, value_parser = utils::parse_date)]
    pub from: Option<chrono::NaiveDate>,
    /// View entries up to this date
    #[clap(short, long, value_parser = utils::parse_date)]
    pub to: Option<chrono::NaiveDate>,
    /// Tags to filter entries by
    #[clap(long, value_delimiter = ' ', num_args = 1)]
    pub tags: Vec<String>,
    /// Show only the most recent entry
    #[clap(short, long)]
    pub recent: bool,
    /// Whether all tags should match or any tag should match
    #[clap(short, long)]
    pub all: bool,
}

fn select_entries<'a>(entries: &'a [Entry], filter: &EntryFilter) -> Vec<&'a Entry> {
    entries
        .iter()
        .filter(|entry| filter.matches(entry))
        .collect()
}

fn print_entries(
    output: &mut impl Write,
    entries: &[&Entry],
    options: DisplayOptions,
) -> io::Result<()> {
    utils::write_list(
        output,
        entries.len(),
        entries
            .iter()
            .map(|entry| utils::format_entry(entry, options)),
    )
}

pub fn execute(journal: &Journal, args: ViewArgs, config: &Config) -> AppResult<()> {
    let filter = EntryFilter {
        dates: DateRange::new(args.from, args.to)?,
        tags: args.tags.into_iter().map(Tag::new).collect(),
        tag_match: if args.all {
            TagMatch::All
        } else {
            TagMatch::Any
        },
    };
    let options = DisplayOptions {
        show_time: config.journal_cfg.show_time,
        inline_tags: config.journal_cfg.inline_tags,
    };
    let mut output = std::io::stdout().lock();
    if let Some(id) = args.id {
        let entry = journal.get_entry(id).ok_or(AppError::EntryNotFound(id))?;
        utils::write_detail(&mut output, entry, options)?;
    } else {
        let mut entries = select_entries(journal.entries(), &filter);
        if args.recent {
            entries = entries.last().copied().into_iter().collect();
        }
        print_entries(&mut output, &entries, options)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_borrows_and_respects_tag_mode() {
        let entries = [
            Entry::new(
                0,
                "one".into(),
                vec![Tag::new("work".into()), Tag::new("rust".into())],
            ),
            Entry::new(1, "two".into(), vec![Tag::new("work".into())]),
        ];
        let mut filter = EntryFilter {
            tags: vec![Tag::new("work".into()), Tag::new("rust".into())],
            tag_match: TagMatch::All,
            ..EntryFilter::default()
        };
        assert_eq!(select_entries(&entries, &filter).len(), 1);
        filter.tag_match = TagMatch::Any;
        assert_eq!(select_entries(&entries, &filter).len(), 2);
    }
}
