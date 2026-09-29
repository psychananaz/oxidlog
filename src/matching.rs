use std::{borrow::Cow, ops::Range};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
