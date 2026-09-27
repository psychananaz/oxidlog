use crate::{query::SearchMatch, storage::Entry};
use colored::Colorize;
use std::{
    fmt::Write as _,
    io::{self, Write},
    ops::Range,
};

#[derive(Clone, Copy, Default)]
pub struct DisplayOptions {
    pub show_time: bool,
}

pub fn format_entry(entry: &Entry, options: DisplayOptions) -> String {
    let mut body = String::new();
    for part in entry.body.split_inclusive(char::is_whitespace) {
        if part.starts_with('#') {
            body.push_str(&part.bright_green().to_string());
        } else {
            body.push_str(part);
        }
    }
    format_with_body(entry, options, &body)
}

fn format_with_body(entry: &Entry, options: DisplayOptions, body: &str) -> String {
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
    for tag in &entry.tags {
        write!(result, " {}", tag.name.bright_yellow()).unwrap();
    }
    write!(result, "\n{body}\n{}", "-".repeat(40)).unwrap();
    result
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

fn write_list(
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

pub fn write_entries(
    output: &mut impl Write,
    entries: &[&Entry],
    options: DisplayOptions,
) -> io::Result<()> {
    write_list(
        output,
        entries.len(),
        entries.iter().map(|entry| format_entry(entry, options)),
    )
}

pub fn write_search_results(
    output: &mut impl Write,
    found: &[SearchMatch<'_>],
    options: DisplayOptions,
) -> io::Result<()> {
    write_list(
        output,
        found.len(),
        found.iter().map(|result| {
            let body = highlight(&result.entry.body, &result.ranges);
            format_with_body(result.entry, options, &body)
        }),
    )
}

pub fn write_detail(output: &mut impl Write, entry: &Entry) -> io::Result<()> {
    writeln!(output, "\n{}", "=".repeat(50))?;
    writeln!(output, "Entry #{}", entry.id)?;
    writeln!(output, "Date: {}", entry.date)?;
    writeln!(output, "\n{}\n", entry.body)?;
    if !entry.tags.is_empty() {
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
    use crate::storage::Tag;

    #[test]
    fn preserves_body_whitespace_and_includes_metadata() {
        let entry = Entry::new(
            4,
            "first\n  second".into(),
            vec![Tag::new("unique_tag".into())],
        );
        let formatted = format_entry(&entry, DisplayOptions { show_time: true });
        assert!(formatted.contains("first\n  second"));
        assert!(formatted.lines().next().unwrap().contains("unique_tag"));
        let mut output = Vec::new();
        write_detail(&mut output, &entry).unwrap();
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
            write_entries(&mut FailedOutput, &[], DisplayOptions::default())
                .unwrap_err()
                .kind(),
            io::ErrorKind::BrokenPipe
        );
    }
}
