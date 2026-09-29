// TODO: maybe move this to a separate module rather than keeping it in utils
z
use crate::{
    error::{AppError, AppResult},
    storage::{Entry, Tag},
};
use colored::Colorize;
use std::{
    borrow::Cow,
    fmt::Write as _,
    io::{self, Write},
};

pub struct EntryContent {
    pub body: String,
    pub tags: Vec<Tag>,
}

pub fn parse_tags(input: &str) -> Vec<Tag> {
    let mut tags = Vec::new();
    for word in input.split_whitespace() {
        let tag = Tag::from_hash(word);
        if !tag.name.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    tags
}

pub fn parse_content(mut content: String) -> AppResult<EntryContent> {
    content.truncate(content.trim_end().len());
    let leading = content.len() - content.trim_start().len();
    content.drain(..leading);
    if content.is_empty() {
        return Err(AppError::EmptyBody);
    }
    let mut tags = Vec::new();
    for word in content
        .split_whitespace()
        .filter(|word| word.starts_with('#'))
    {
        let tag = Tag::from_hash(word);
        if !tag.name.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }
    let body = content
        .split_inclusive(char::is_whitespace)
        .filter(|part| !part.starts_with('#'))
        .collect::<String>()
        .trim()
        .to_owned();
    if body.is_empty() {
        return Err(AppError::EmptyBody);
    }
    Ok(EntryContent { body, tags })
}

pub fn get_input(prompt: &str) -> io::Result<String> {
    let mut output = io::stdout().lock();
    write!(output, "{prompt}")?;
    output.flush()?;
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Input ended; edit cancelled",
        ));
    }
    Ok(input.trim().to_owned())
}

#[derive(Clone, Copy, Default)]
pub enum TagMatch {
    #[default]
    Any, // OR operation
    All, // AND operation
}

pub fn do_tags_match(query_tags: &[Tag], entry_tags: &[Tag], match_type: TagMatch) -> bool {
    if query_tags.is_empty() {
        return true;
    }

    match match_type {
        TagMatch::Any => query_tags.iter().any(|tag| entry_tags.contains(tag)),
        TagMatch::All => query_tags.iter().all(|tag| entry_tags.contains(tag)),
    }
}

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

#[derive(Clone, Copy, Default)]
pub struct DisplayOptions {
    pub show_time: bool,
    pub inline_tags: bool,
}

pub fn format_entry(entry: &Entry, options: DisplayOptions) -> String {
    format_with_body(entry, options, &entry.body)
}

fn displayed_body<'a>(entry: &Entry, body: &'a str, options: DisplayOptions) -> Cow<'a, str> {
    if !options.inline_tags || entry.tags.is_empty() {
        return Cow::Borrowed(body);
    }
    let mut body = body.to_owned();
    for tag in &entry.tags {
        write!(body, " {}", format!("#{}", tag.name).bright_green()).unwrap();
    }
    Cow::Owned(body)
}

pub fn format_with_body(entry: &Entry, options: DisplayOptions, body: &str) -> String {
    let mut result = format!("[{:>3}] {}", entry.id, entry.date.to_string().bright_blue());
    if options.show_time {
        write!(
            result,
            " {}",
            entry
                .timestamp
                .format("%H:%M")
                .to_string()
                .dimmed()
                .underline()
        )
        .unwrap();
    }
    if !options.inline_tags {
        for tag in &entry.tags {
            write!(result, " {}", tag.name.bright_yellow()).unwrap();
        }
    }
    let body = displayed_body(entry, body, options);
    write!(result, "\n{body}\n").unwrap();
    result
}

pub fn write_list(
    output: &mut impl Write,
    count: usize,
    entries: impl Iterator<Item = String>,
) -> io::Result<()> {
    if count == 0 {
        writeln!(output, "No entries found.")?;
    } else {
        writeln!(output, "{count} entries found")?;
    }
    for entry in entries {
        writeln!(output, "{entry}")?;
    }
    Ok(())
}

pub fn write_detail(
    output: &mut impl Write,
    entry: &Entry,
    options: DisplayOptions,
) -> io::Result<()> {
    writeln!(output, "\n{}", "=".repeat(50))?;
    writeln!(output, "Entry #{}", entry.id)?;
    writeln!(output, "Date: {}", entry.date)?;
    writeln!(
        output,
        "\n{}\n",
        displayed_body(entry, &entry.body, options)
    )?;
    if !options.inline_tags && !entry.tags.is_empty() {
        write!(output, "Tags:")?;
        for tag in &entry.tags {
            write!(output, " #{}", tag.name)?;
        }
        writeln!(output)?;
    }
    writeln!(output, "{}", "=".repeat(50))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_and_deduplicates_tags() {
        assert_eq!(
            parse_tags("#work work ##home #"),
            vec![Tag::new("work".into()), Tag::new("home".into())]
        );
    }

    #[test]
    fn content_extracts_tags_and_preserves_other_internal_whitespace() {
        let parsed = parse_content("  first\n  second #work #work  ".into()).unwrap();
        assert_eq!(parsed.body, "first\n  second");
        assert_eq!(parsed.tags, vec![Tag::new("work".into())]);
        assert!(parse_content(" #work ".into()).is_err());
    }

    #[test]
    fn empty_filters_match_all_and_invalid_date_ranges_fail() {
        let entry = Entry::new(0, "".into(), vec![]);
        assert!(EntryFilter::default().matches(&entry));
        let day = parse_date("2025-01-02").unwrap();
        assert!(DateRange::new(Some(day), Some(parse_date("2025-01-01").unwrap())).is_err());
        assert!(parse_date("2025-02-30").is_err());
    }

    #[test]
    fn preserves_body_whitespace_and_includes_metadata() {
        let entry = Entry::new(
            4,
            "first\n  second".into(),
            vec![Tag::new("unique_tag".into())],
        );
        let formatted = format_entry(
            &entry,
            DisplayOptions {
                show_time: true,
                ..DisplayOptions::default()
            },
        );
        assert!(formatted.contains("first\n  second"));
        assert!(formatted.lines().next().unwrap().contains("unique_tag"));
        let mut output = Vec::new();
        write_detail(&mut output, &entry, DisplayOptions::default()).unwrap();
        let detail = String::from_utf8(output).unwrap();
        assert!(detail.contains("Entry #4"));
        assert!(detail.contains("Tags: #unique_tag"));
    }

    #[test]
    fn output_errors_are_returned() {
        struct FailedOutput;
        impl Write for FailedOutput {
            fn write(&mut self, _: &[u8]) -> io::Result<usize> {
                Err(io::ErrorKind::BrokenPipe.into())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        assert_eq!(
            write_list(&mut FailedOutput, 0, std::iter::empty())
                .unwrap_err()
                .kind(),
            io::ErrorKind::BrokenPipe
        );
    }
}
