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
}
