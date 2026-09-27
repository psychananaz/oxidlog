use crate::error::{AppError, AppResult};
use crate::query::{self, DateRange};
use crate::storage::{self, Journal};
use chrono::NaiveDate;
use std::io::Write as _;
use std::ops::RangeInclusive;

#[derive(clap::Args, Debug)]
pub struct RemoveArgs {
    pub id: Option<usize>,
    /// Inclusive ID range, written start..end
    #[clap(short, long, value_parser = parse_id_range)]
    pub range: Option<RangeInclusive<usize>>,
    #[clap(short, long, value_parser = query::parse_date)]
    pub from: Option<NaiveDate>,
    #[clap(short, long, value_parser = query::parse_date)]
    pub to: Option<NaiveDate>,
}

fn parse_id_range(value: &str) -> Result<RangeInclusive<usize>, String> {
    let (start, end) = value
        .split_once("..")
        .ok_or("Invalid range format. Use start..end")?;
    let start = start.parse::<usize>().map_err(|_| "Invalid range start")?;
    let end = end.parse::<usize>().map_err(|_| "Invalid range end")?;
    if start > end {
        return Err("Range start must not exceed end".into());
    }
    Ok(start..=end)
}

fn select_ids(journal: &Journal, args: &RemoveArgs) -> AppResult<Vec<usize>> {
    let dates = DateRange::new(args.from, args.to)?;
    if let Some(id) = args.id {
        if journal.get_entry(id).is_none() {
            return Err(AppError::EntryNotFound(id));
        }
    }
    let has_dates = args.from.is_some() || args.to.is_some();
    let mut ids: Vec<_> = journal
        .entries()
        .iter()
        .filter(|entry| {
            Some(entry.id) == args.id
                || args
                    .range
                    .as_ref()
                    .is_some_and(|range| range.contains(&entry.id))
                || (has_dates && dates.contains(entry.date))
        })
        .map(|entry| entry.id)
        .collect();
    ids.sort_unstable();
    if ids.is_empty() {
        return Err(AppError::NoEntriesSelected);
    }
    Ok(ids)
}

pub fn execute(journal: &mut Journal, args: RemoveArgs) -> AppResult<()> {
    let ids = select_ids(journal, &args)?;
    for &id in &ids {
        journal.remove_entry(id);
    }
    storage::save_journal(journal)?;
    for id in ids {
        writeln!(std::io::stdout().lock(), "Entry {id} removed")?;
    }
    Ok(())
}
