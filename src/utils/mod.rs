use std::io::{self, Write};

use colored::Colorize;

use crate::storage::{config::JournalConfig, Entry, Journal, Tag};

/// Prompts the user for input and returns the trimmed input as a String.
///
/// # Arguments
///
/// * `prompt` - A string slice that holds the prompt message to display to the user.
///
/// # Returns
///
/// A `String` containing the user's input.
pub fn get_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

pub enum TagMatch {
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

/// Parses a string of tags separated by whitespace into a vector of `Tag` structs.
///
/// # Arguments
///
/// * `tags` - A string slice containing the tags separated by whitespace.
///
/// # Returns
///
/// A vector of `Tag` structs.
pub fn parse_tags(tags: &str) -> Vec<Tag> {
    tags.split_whitespace()
        .map(|t| t.parse::<Tag>().unwrap())
        .collect()
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
pub fn parse_date(date: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap()
}

/// Formats a journal entry into a string for display.
///
/// # Arguments
///
/// * `entry` - A reference to the `Entry` struct to format.
/// * `cfg` - A `JournalConfig` struct containing configuration settings.
///
/// # Returns
///
/// A `String` containing the formatted entry.
pub fn format_entry(entry: &Entry, cfg: JournalConfig) -> String {
    let mut formatted = String::new();
    formatted.push_str(
        &format!(
            "[{:>3}] {}",
            entry.id,
            entry.date.format("%Y-%m-%d").to_string().bright_blue()
        )
        .to_string(),
    );

    if cfg.show_time {
        formatted.push_str(&format!(
            " {}",
            entry
                .timestamp
                .format("%H:%M")
                .to_string()
                .dimmed()
                .underline()
        ));
    }

    if !entry.tags.is_empty() {
        formatted.push_str(&format!(
            " {}",
            entry
                .tags
                .iter()
                .map(|t| t.to_string())
                .collect::<Vec<String>>()
                .join(" ")
                .bright_yellow()
        ));
    }

    let body_colored = entry
        .body
        .split_whitespace()
        .map(|word| {
            if word.starts_with('#') {
                word.bright_green().to_string()
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<String>>();

    formatted.push_str(&format!("\n{}\n", body_colored.join(" ")));
    formatted.push_str(&"-".repeat(40));

    formatted
}

/// Performs a fuzzy match of the needle string within the haystack string.
///
/// # Arguments
///
/// * `haystack` - A string slice representing the text to search within.
/// * `needle` - A string slice representing the text to search for.
///
/// # Returns
///
/// A boolean indicating whether the needle was found in the haystack.
pub fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let needle_chars = needle.chars();
    let mut haystack_chars = haystack.chars();

    for needle_char in needle_chars {
        if let Some(haystack_char) = haystack_chars.next() {
            if needle_char == haystack_char {
                continue;
            } else {
                for next_haystack_char in haystack_chars.by_ref() {
                    if needle_char == next_haystack_char {
                        break;
                    }
                }
            }
        } else {
            return false;
        }
    }

    true
}

/// Views a journal entry by its ID.
///
/// # Arguments
///
/// * `journal` - A reference to the `Journal` struct containing the entries.
/// * `id` - The ID of the entry to view.
pub fn view_by_id(journal: &Journal, id: usize) {
    if let Some(entry) = journal.entries().iter().find(|e| e.id == id) {
        print_single_entry(entry);
    } else {
        println!("Entry with id {id} not found");
    }
}

/// Prints a single journal entry.
///
/// # Arguments
///
/// * `entry` - A reference to the `Entry` struct to print.
pub fn print_single_entry(entry: &Entry) {
    println!("\n{}", "=".repeat(50));
    println!("Entry #{}", entry.id);
    println!("Date: {}", entry.date);
    println!("\n{}\n", entry.body);

    let tags = entry
        .tags
        .iter()
        .map(|t| format!("#{}", t.name))
        .collect::<Vec<_>>()
        .join(" ");

    if !tags.is_empty() {
        println!("Tags: {}", tags);
    }
    println!("{}", "=".repeat(50));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_do_tags_match_any() {
        let query_tags = vec![
            Tag::new("test".to_string()),
            Tag::new("example".to_string()),
        ];
        let entry_tags = vec![
            Tag::new("example".to_string()),
            Tag::new("sample".to_string()),
        ];
        assert!(do_tags_match(&query_tags, &entry_tags, TagMatch::Any));
    }

    #[test]
    fn test_do_tags_match_all() {
        let query_tags = vec![
            Tag::new("test".to_string()),
            Tag::new("example".to_string()),
        ];
        let entry_tags = vec![
            Tag::new("test".to_string()),
            Tag::new("example".to_string()),
            Tag::new("sample".to_string()),
        ];
        assert!(do_tags_match(&query_tags, &entry_tags, TagMatch::All));
    }

    #[test]
    fn test_parse_tags() {
        let tags_str = "test example sample";
        let tags = parse_tags(tags_str);
        assert_eq!(tags.len(), 3);
        assert_eq!(tags[0].name, "test");
        assert_eq!(tags[1].name, "example");
        assert_eq!(tags[2].name, "sample");
    }

    #[test]
    fn test_parse_date() {
        let date_str = "2023-09-15";
        let date = parse_date(date_str);
        assert_eq!(date, chrono::NaiveDate::from_ymd_opt(2023, 9, 15).unwrap());
    }

    #[test]
    fn test_format_entry() {
        let entry = Entry::new(
            1,
            "Test entry".to_string(),
            vec![Tag::new("unique_tag".to_string())],
        );
        let config = JournalConfig {
            body_tags: true,
            show_time: true,
            export_dir: "exports".to_string(),
        };
        let formatted = format_entry(&entry, config);
        assert!(formatted.contains("Test entry"));
        assert!(formatted.lines().next().unwrap().contains("unique_tag"));
    }

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("fuzzy matching", "fuz mat"));
        assert!(!fuzzy_match("fuzzy matching", "fuzzy not matching"));
    }
}
