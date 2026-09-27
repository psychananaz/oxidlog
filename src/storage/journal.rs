use crate::{
    content::EntryContent,
    error::{AppError, AppResult},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Hash, Eq)]
pub struct Tag {
    pub name: String,
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn from_hash(s: &str) -> Self {
        Self::new(s.trim_start_matches('#').to_string())
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl FromStr for Tag {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Tag::new(s.to_string()))
    }
}

impl AsRef<str> for Tag {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Entry {
    pub id: usize,
    pub timestamp: DateTime<Utc>,
    pub date: NaiveDate,
    pub body: String,
    pub tags: Vec<Tag>,
}

impl Entry {
    pub fn new(id: usize, body: String, tags: Vec<Tag>) -> Self {
        let timestamp = Utc::now();
        Self {
            id,
            timestamp,
            date: timestamp.date_naive(),
            body,
            tags,
        }
    }
}

pub struct Journal {
    path: PathBuf,
    entries: Vec<Entry>,
}

impl Journal {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            entries: Vec::new(),
        }
    }

    pub fn from_entries(path: PathBuf, entries: Vec<Entry>) -> AppResult<Self> {
        let mut ids = HashSet::with_capacity(entries.len());
        for entry in &entries {
            if !ids.insert(entry.id) {
                return Err(AppError::DuplicateId(entry.id));
            }
        }
        Ok(Self { path, entries })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn add_entry(&mut self, body: String, tags: Vec<Tag>) -> AppResult<usize> {
        if body.trim().is_empty() {
            return Err(AppError::EmptyBody);
        }
        let id = self.next_id()?;
        let entry = Entry::new(id, body, tags);

        self.entries.push(entry);
        Ok(id)
    }

    pub fn remove_entry(&mut self, id: usize) -> Option<Entry> {
        if let Some(index) = self.entries.iter().position(|e| e.id == id) {
            Some(self.entries.remove(index))
        } else {
            None
        }
    }

    /// Apply editable fields without allowing callers to replace IDs or timestamps.
    pub fn edit_entry(
        &mut self,
        id: usize,
        content: Option<EntryContent>,
        tags: Option<Vec<Tag>>,
    ) -> AppResult<()> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or(AppError::EntryNotFound(id))?;
        if let Some(content) = &content {
            if content.body.trim().is_empty() {
                return Err(AppError::EmptyBody);
            }
        }
        if let Some(tags) = tags {
            entry.tags = tags;
        }
        if let Some(content) = content {
            entry.body = content.body;
            for tag in content.tags {
                if !entry.tags.contains(&tag) {
                    entry.tags.push(tag);
                }
            }
        }
        Ok(())
    }

    pub fn get_entry(&self, id: usize) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    fn next_id(&self) -> AppResult<usize> {
        match self.entries.iter().map(|entry| entry.id).max() {
            Some(id) => id.checked_add(1).ok_or(AppError::IdExhausted),
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entry_creation() {
        let entry = Entry::new(
            1,
            "Test entry".to_string(),
            vec![Tag::new("test".to_string())],
        );
        assert_eq!(entry.id, 1);
        assert_eq!(entry.body, "Test entry");
        assert_eq!(entry.tags[0].name, "test");
        assert!(entry.timestamp <= Utc::now());
    }

    #[test]
    fn test_entry_creation_with_details() {
        let body = "Test entry".to_string();
        let tags = vec![
            Tag::new("test".to_string()),
            Tag::new("important".to_string()),
        ];
        let entry = Entry::new(1, body.clone(), tags.clone());

        assert_eq!(entry.id, 1);
        assert_eq!(entry.body, body);
        assert_eq!(entry.tags, tags);
        assert!(entry.timestamp <= Utc::now());
        assert_eq!(entry.date, Utc::now().naive_utc().date());
    }

    #[test]
    fn test_journal_operations() {
        let path = PathBuf::from("test_journal.json");
        let mut journal = Journal::new(path);

        // Test adding entries
        journal
            .add_entry("First entry".to_string(), vec![])
            .unwrap();
        journal
            .add_entry(
                "Second entry".to_string(),
                vec![Tag::new("tag1".to_string())],
            )
            .unwrap();

        assert_eq!(journal.entries().len(), 2);
        assert_eq!(journal.get_entry(0).unwrap().body, "First entry");
        assert_eq!(journal.get_entry(1).unwrap().body, "Second entry");

        // Test removing entries
        let removed = journal.remove_entry(0);
        assert!(removed.is_some());
        assert_eq!(journal.entries().len(), 1);

        // Test updating entries
        journal
            .edit_entry(
                1,
                Some(EntryContent {
                    body: "Updated entry".into(),
                    tags: vec![],
                }),
                None,
            )
            .unwrap();
        assert_eq!(journal.get_entry(1).unwrap().body, "Updated entry");
    }

    #[test]
    fn test_journal_comprehensive_operations() {
        let path = PathBuf::from("test_journal.json");
        let mut journal = Journal::new(path);

        // Test sequential adding and ID assignment
        for i in 0..3 {
            journal
                .add_entry(format!("Entry {}", i), vec![Tag::new(format!("tag{}", i))])
                .unwrap();
        }

        // Verify correct ID assignment
        assert_eq!(journal.entries().len(), 3);
        for i in 0..3 {
            let entry = journal.get_entry(i).unwrap();
            assert_eq!(entry.id, i);
            assert_eq!(entry.body, format!("Entry {}", i));
            assert_eq!(entry.tags, vec![Tag::new(format!("tag{}", i))]);
        }

        // Test entry removal from middle
        let removed = journal.remove_entry(1).unwrap();
        assert_eq!(removed.body, "Entry 1");
        assert_eq!(journal.entries().len(), 2);
        assert!(journal.get_entry(1).is_none());

        // Test entry update
        journal
            .edit_entry(
                2,
                Some(EntryContent {
                    body: "Updated content".into(),
                    tags: vec![],
                }),
                Some(vec![Tag::new("updated".into())]),
            )
            .unwrap();

        let updated = journal.get_entry(2).unwrap();
        assert_eq!(updated.body, "Updated content");
        assert_eq!(updated.tags, vec![Tag::new("updated".to_string())]);
    }

    #[test]
    fn test_journal_edge_cases() {
        let path = PathBuf::from("test_journal.json");
        let mut journal = Journal::new(path);

        // Test empty journal behaviors
        assert_eq!(journal.next_id().unwrap(), 0);
        assert!(journal.get_entry(0).is_none());
        assert!(journal.remove_entry(0).is_none());
        assert_eq!(journal.entries().len(), 0);

        assert!(journal.add_entry(String::new(), vec![]).is_err());
        assert!(journal.edit_entry(999, None, None).is_err());
        assert!(journal.entries().is_empty());
    }

    #[test]
    fn test_multiple_entries_same_day() {
        let path = PathBuf::from("test_journal.json");
        let mut journal = Journal::new(path);

        // Add multiple entries for the same day
        for i in 0..3 {
            journal
                .add_entry(
                    format!("Same day entry {}", i),
                    vec![Tag::new("same_day".to_string())],
                )
                .unwrap();
        }

        let entries = journal.entries();
        let first_date = entries[0].date;

        // Verify all entries have the same date
        assert!(entries.iter().all(|e| e.date == first_date));
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn test_journal_next_id() {
        let path = PathBuf::from("test_journal.json");
        let mut journal = Journal::new(path);

        assert_eq!(journal.next_id().unwrap(), 0);
        journal.add_entry("Entry".to_string(), vec![]).unwrap();
        assert_eq!(journal.next_id().unwrap(), 1);
    }
}
