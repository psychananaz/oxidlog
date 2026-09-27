use crate::{
    error::{AppError, AppResult},
    storage::Tag,
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

pub fn parse_content(mut content: String, body_tags: bool) -> AppResult<EntryContent> {
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
    let body = if body_tags {
        content
            .split_inclusive(char::is_whitespace)
            .filter(|part| !part.starts_with('#'))
            .collect::<String>()
            .trim()
            .to_owned()
    } else {
        content
    };
    if body.is_empty() {
        return Err(AppError::EmptyBody);
    }
    Ok(EntryContent { body, tags })
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
    fn content_preserves_internal_whitespace_in_both_modes() {
        for mode in [false, true] {
            let parsed = parse_content("  first\n  second #work #work  ".into(), mode).unwrap();
            assert!(parsed.body.starts_with("first\n  second"));
            assert_eq!(parsed.tags, vec![Tag::new("work".into())]);
            assert_eq!(parsed.body.contains("#work"), !mode);
        }
        assert!(parse_content(" #work ".into(), true).is_err());
    }
}
