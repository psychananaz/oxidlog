use crate::{
    error::JotResult,
    matching::TextQuery,
    presentation::{self, DisplayOptions},
    query::{self, DateRange, EntryFilter, SearchQuery, TagMatch},
    storage::{config::Config, Journal, Tag},
};

// TODO: add regex search
#[derive(clap::Args)]
pub struct SearchArgs {
    pub query: String,
    #[clap(long, value_delimiter = ' ')]
    pub tags: Vec<String>,
    #[clap(long, value_parser = query::parse_date)]
    pub from: Option<chrono::NaiveDate>,
    #[clap(long, value_parser = query::parse_date)]
    pub to: Option<chrono::NaiveDate>,
    #[clap(short, long)]
    pub fuzzy: bool,
    #[clap(short, long)]
    pub all: bool,
    #[clap(short, long)]
    pub case_sensitive: bool,
}

pub fn execute(journal: &Journal, args: SearchArgs, config: &Config) -> JotResult<()> {
    let query = SearchQuery {
        filter: EntryFilter {
            dates: DateRange::new(args.from, args.to)?,
            tags: args.tags.into_iter().map(Tag::new).collect(),
            tag_match: if args.all {
                TagMatch::All
            } else {
                TagMatch::Any
            },
        },
        text: TextQuery::new(args.query, args.case_sensitive, args.fuzzy),
    };
    let found = query::search(journal.entries(), &query);
    presentation::write_search_results(
        &mut std::io::stdout().lock(),
        &found,
        DisplayOptions {
            show_time: config.journal_cfg.show_time,
        },
    )?;
    Ok(())
}
