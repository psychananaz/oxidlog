use crate::{
    error::AppResult,
    storage::{config::Config, Entry, Journal, Tag},
    utils::{self, DateRange, DisplayOptions, EntryFilter, TagMatch},
};
use colored::Colorize;
use std::{
    borrow::Cow,
    io::{self, Write},
    ops::Range,
};

// TODO: add regex search
#[derive(clap::Args)]
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

pub struct TextQuery {
    term: String,
    case_sensitive: bool,
    fuzzy: bool,
}

impl TextQuery {
    pub fn new(term: String, case_sensitive: bool, fuzzy: bool) -> Self {
        let term = if case_sensitive {
            term
        } else {
            term.to_lowercase()
        };
        Self {
            term,
            case_sensitive,
            fuzzy,
        }
    }

    pub fn find(&self, body: &str) -> Option<Vec<Range<usize>>> {
        if self.term.is_empty() {
            return Some(Vec::new());
        }
        let text = if self.case_sensitive {
            Cow::Borrowed(body)
        } else {
            Cow::Owned(body.to_lowercase())
        };
        let ranges: Vec<Range<usize>> = if self.fuzzy {
            let mut chars = text.char_indices();
            let mut ranges = Vec::new();
            for wanted in self.term.chars() {
                let (offset, character) = chars.find(|(_, character)| *character == wanted)?;
                ranges.push(offset..offset + character.len_utf8());
            }
            ranges
        } else {
            let ranges: Vec<_> = text
                .match_indices(&self.term)
                .map(|(offset, matched)| offset..offset + matched.len())
                .collect();
            if ranges.is_empty() {
                return None;
            }
            ranges
        };
        if self.case_sensitive {
            return Some(ranges);
        }

        // Lowercasing can expand a character (İ -> i + combining dot).
        // Map every normalized byte back to the whole original character.
        let mut original = Vec::with_capacity(text.len());
        for (offset, character) in body.char_indices() {
            let width: usize = character.to_lowercase().map(char::len_utf8).sum();
            original.extend(std::iter::repeat_n(
                offset..offset + character.len_utf8(),
                width,
            ));
        }
        let mut mapped: Vec<Range<usize>> = Vec::new();
        for range in ranges {
            let range = original[range.start].start..original[range.end - 1].end;
            if let Some(last) = mapped.last_mut() {
                if range.start <= last.end {
                    last.end = last.end.max(range.end);
                    continue;
                }
            }
            mapped.push(range);
        }
        Some(mapped)
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

fn search_entries<'a>(entries: &'a [Entry], query: &SearchQuery) -> Vec<SearchMatch<'a>> {
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

fn highlight(body: &str, ranges: &[Range<usize>]) -> String {
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

fn print_results(
    output: &mut impl Write,
    found: &[SearchMatch<'_>],
    options: DisplayOptions,
) -> io::Result<()> {
    utils::write_list(
        output,
        found.len(),
        found.iter().map(|result| {
            let body = highlight(&result.entry.body, &result.ranges);
            utils::format_with_body(result.entry, options, &body)
        }),
    )
}

pub fn execute(journal: &Journal, args: SearchArgs, config: &Config) -> AppResult<()> {
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
    let found = search_entries(journal.entries(), &query);
    print_results(
        &mut std::io::stdout().lock(),
        &found,
        DisplayOptions {
            show_time: config.journal_cfg.show_time,
            inline_tags: config.journal_cfg.inline_tags,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn fuzzy_requires_every_character_in_order() {
        let query = |term: &str| TextQuery::new(term.into(), true, true);
        assert!(query("z").find("abc").is_none());
        assert!(query("az").find("abc").is_none());
        assert!(query("ba").find("abc").is_none());
        assert_eq!(query("ac").find("abc"), Some(vec![0..1, 2..3]));
        assert_eq!(query("").find(""), Some(vec![]));
    }

    #[test]
    fn repeated_matches_use_original_offsets() {
        assert_eq!(
            TextQuery::new("cat".into(), false, false).find("CAT cat"),
            Some(vec![0..3, 4..7])
        );
        assert_eq!(
            TextQuery::new("CAT".into(), true, false).find("CAT cat"),
            Some(std::iter::once(0..3).collect())
        );
    }

    #[test]
    fn unicode_offsets_stay_on_original_character_boundaries() {
        let body = "İ café CAFÉ";
        assert_eq!(
            TextQuery::new("i".into(), false, false).find(body),
            Some(std::iter::once(0..2).collect())
        );
        assert_eq!(
            TextQuery::new("café".into(), false, false).find(body),
            Some(vec![3..8, 9..14])
        );
        assert_eq!(
            TextQuery::new("i\u{307}".into(), false, true).find(body),
            Some(std::iter::once(0..2).collect())
        );
        assert_eq!(
            TextQuery::new("ος".into(), false, false).find("ΟΣ"),
            Some(std::iter::once(0..4).collect())
        );
    }

    #[test]
    fn search_selects_borrowed_entries_after_shared_filters() {
        let day = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let mut first = Entry::new(
            0,
            "hello world".into(),
            vec![Tag::new("work".into()), Tag::new("rust".into())],
        );
        first.date = day;
        let mut second = Entry::new(1, "hello again".into(), vec![Tag::new("work".into())]);
        second.date = day;
        let entries = [first, second];
        let query = SearchQuery {
            filter: EntryFilter {
                dates: DateRange::new(Some(day), Some(day)).unwrap(),
                tags: vec![Tag::new("work".into()), Tag::new("rust".into())],
                tag_match: TagMatch::All,
            },
            text: TextQuery::new("world".into(), false, false),
        };
        let results = search_entries(&entries, &query);
        assert_eq!(results.len(), 1);
        assert!(std::ptr::eq(results[0].entry, &entries[0]));
        assert_eq!(results[0].ranges, vec![6..11]);
    }
}
