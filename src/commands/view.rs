use crate::{
    error::{JotError, JotResult},
    storage::{config::Config, Entry, Journal, Tag},
    utils::{self, TagMatch},
};

#[derive(clap::Args, Clone)]
pub struct ViewArgs {
    /// ID of the specific entry to view
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

fn print_formatted_entries(entries: &[&Entry], config: &Config) {
    if entries.is_empty() {
        println!("No entries found.");
    } else {
        println!("{} entries found", entries.len());
        entries
            .iter()
            .for_each(|e| println!("{}", utils::format_entry(e, config.journal_cfg.clone())));
    }
}

fn view_recent(journal: &Journal, config: &Config) {
    let entries = journal.get_entries();
    if let Some(last) = entries.iter().last() {
        print_formatted_entries(&[last], config);
    }
}

fn filter_entries<'a>(
    entries: &'a [Entry],
    args: &ViewArgs,
    dates: &utils::DateRange,
) -> Vec<&'a Entry> {
    entries
        .iter()
        .filter(|e| {
            if !dates.contains(e.date) {
                return false;
            }
            let match_type = if args.all {
                TagMatch::All
            } else {
                TagMatch::Any
            };

            utils::do_tags_match(
                &args
                    .tags
                    .iter()
                    .map(|t| Tag::new(t.to_string()))
                    .collect::<Vec<_>>(),
                &e.tags,
                match_type,
            )
        })
        .collect()
}

pub fn execute(journal: &Journal, args: ViewArgs, config: &Config) -> JotResult<()> {
    let dates = utils::DateRange::new(args.from, args.to)?;
    if let Some(id) = args.id {
        if args.recent {
            return Err(JotError::CommandError(
                "Cannot specify both an ID and --recent".to_string(),
            ));
        }

        utils::view_by_id(journal, id);
    } else if args.recent {
        view_recent(journal, config);
    } else {
        let entries = filter_entries(journal.get_entries(), &args, &dates);
        print_formatted_entries(&entries, config);
    }

    Ok(())
}
