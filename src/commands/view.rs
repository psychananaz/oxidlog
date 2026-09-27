use crate::{
    error::{AppError, AppResult},
    presentation::{self, DisplayOptions},
    query::{self, DateRange, EntryFilter, TagMatch},
    storage::{config::Config, Journal, Tag},
};

#[derive(clap::Args)]
pub struct ViewArgs {
    /// ID of the specific entry to view
    #[arg(conflicts_with_all = ["recent", "from", "to", "tags", "all"])]
    pub id: Option<usize>,
    /// View entries starting from this date
    #[clap(short, long, value_parser = query::parse_date)]
    pub from: Option<chrono::NaiveDate>,
    /// View entries up to this date
    #[clap(short, long, value_parser = query::parse_date)]
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
        presentation::write_detail(&mut output, entry, options)?;
    } else {
        let mut entries = query::select(journal.entries(), &filter);
        if args.recent {
            entries = entries.last().copied().into_iter().collect();
        }
        presentation::write_entries(&mut output, &entries, options)?;
    }
    Ok(())
}
