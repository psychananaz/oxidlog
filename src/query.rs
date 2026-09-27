use crate::{
    error::AppResult,
    matching::TextQuery,
    storage::{Entry, Tag},
};
use std::ops::Range;

#[derive(Clone, Copy, Default)]
pub enum TagMatch {
    #[default]
    Any, // OR operation
    All, // AND operation
}

/// Checks if the tags in the query match the tags in the entry based on the match type.
///
/// # Arguments
///
/// * `query_tags` - A slice of `Tag` representing the tags to query.
/// * `entry_tags` - A slice of `Tag` representing the tags in the entry.
/// * `match_type` - A `TagMatch` enum indicating whether to match any or all tags.
///
/// # Returns
///
/// A boolean indicating whether the tags match.
pub fn do_tags_match(query_tags: &[Tag], entry_tags: &[Tag], match_type: TagMatch) -> bool {
    if query_tags.is_empty() {
        return true;
    }

    match match_type {
        TagMatch::Any => query_tags.iter().any(|tag| entry_tags.contains(tag)),
        TagMatch::All => query_tags.iter().all(|tag| entry_tags.contains(tag)),
    }
}

/// Parses a date string in the format "YYYY-MM-DD" into a `NaiveDate` struct.
///
/// # Arguments
///
/// * `date` - A string slice containing the date in "YYYY-MM-DD" format.
///
/// # Returns
///
/// A `NaiveDate` struct representing the parsed date.
pub fn parse_date(date: &str) -> Result<chrono::NaiveDate, chrono::ParseError> {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
}

#[derive(Default)]
pub struct DateRange {
    from: Option<chrono::NaiveDate>,
    to: Option<chrono::NaiveDate>,
}

impl DateRange {
    pub fn new(from: Option<chrono::NaiveDate>, to: Option<chrono::NaiveDate>) -> AppResult<Self> {
        if matches!((from, to), (Some(from), Some(to)) if from > to) {
            return Err(crate::error::AppError::InvalidDateRange);
        }
        Ok(Self { from, to })
    }

    pub fn contains(&self, date: chrono::NaiveDate) -> bool {
        self.from.is_none_or(|from| date >= from) && self.to.is_none_or(|to| date <= to)
    }
}

#[derive(Default)]
pub struct EntryFilter {
    pub dates: DateRange,
    pub tags: Vec<Tag>,
    pub tag_match: TagMatch,
}

impl EntryFilter {
    pub fn matches(&self, entry: &Entry) -> bool {
        self.dates.contains(entry.date) && do_tags_match(&self.tags, &entry.tags, self.tag_match)
    }
}

pub struct SearchQuery {
    pub filter: EntryFilter,
    pub text: TextQuery,
}

pub struct SearchMatch<'a> {
    pub entry: &'a Entry,
    pub ranges: Vec<Range<usize>>,
}

pub fn select<'a>(entries: &'a [Entry], filter: &EntryFilter) -> Vec<&'a Entry> {
    entries
        .iter()
        .filter(|entry| filter.matches(entry))
        .collect()
}

pub fn search<'a>(entries: &'a [Entry], query: &SearchQuery) -> Vec<SearchMatch<'a>> {
    entries
        .iter()
        .filter(|entry| query.filter.matches(entry))
        .filter_map(|entry| {
            Some(SearchMatch {
                entry,
                ranges: query.text.find(&entry.body)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn selection_and_search_share_inclusive_date_and_tag_filters() {
        let day = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let mut a = Entry::new(
            0,
            "hello world".into(),
            vec![Tag::new("work".into()), Tag::new("rust".into())],
        );
        a.date = day;
        let mut b = Entry::new(1, "hello again".into(), vec![Tag::new("work".into())]);
        b.date = day;
        let entries = [a, b];
        let mut filter = EntryFilter {
            dates: DateRange::new(Some(day), Some(day)).unwrap(),
            tags: vec![Tag::new("work".into()), Tag::new("rust".into())],
            tag_match: TagMatch::All,
        };
        assert_eq!(select(&entries, &filter).len(), 1);
        filter.tag_match = TagMatch::Any;
        assert_eq!(select(&entries, &filter).len(), 2);
        let query = SearchQuery {
            filter,
            text: TextQuery::new("world".into(), false, false),
        };
        let results = search(&entries, &query);
        assert_eq!(results.len(), 1);
        assert!(std::ptr::eq(results[0].entry, &entries[0]));
        assert_eq!(results[0].ranges, vec![6..11]);
    }

    #[test]
    fn empty_filters_match_all_and_invalid_date_ranges_fail() {
        let entry = Entry::new(0, "".into(), vec![]);
        assert!(EntryFilter::default().matches(&entry));
        let day = parse_date("2025-01-02").unwrap();
        assert!(DateRange::new(Some(day), Some(parse_date("2025-01-01").unwrap())).is_err());
        assert!(parse_date("2025-02-30").is_err());
    }
}
