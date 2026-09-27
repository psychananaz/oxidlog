pub mod config;
pub mod journal;

pub use journal::{Entry, Journal, Tag};

use crate::error::{JotError, JotResult};
use config::Config;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const CONFIG_FILE: &str = "config.toml";
const JOURNAL_DIR: &str = ".oxidlog";
const JOURNAL_FILE: &str = "journal.json";
const BACKUP_EXTENSION: &str = ".bak";

pub struct Backup {
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
    pub old_backup_path: PathBuf,
}

impl Backup {
    pub fn from_journal(journal: &Journal) -> Self {
        let source_path = journal.path().to_owned();
        let backup_path = source_path.with_extension(format!("json{}", BACKUP_EXTENSION));
        let old_backup_path = source_path.with_extension(format!("json{}.old", BACKUP_EXTENSION));

        Self {
            source_path,
            backup_path,
            old_backup_path,
        }
    }

    pub fn create(&self) -> JotResult<()> {
        if self.backup_path.exists() {
            fs::rename(&self.backup_path, &self.old_backup_path)
                .map_err(|e| JotError::Other(format!("Failed to rename backup: {}", e).into()))?;
        }

        fs::copy(&self.source_path, &self.backup_path)
            .map_err(|e| JotError::Other(format!("Failed to create backup: {}", e).into()))?;

        Ok(())
    }

    pub fn restore(&self) -> JotResult<()> {
        // Validate before replacing the current file. Restore must work even
        // when the current journal is corrupt or missing.
        let content = fs::read(&self.backup_path)?;
        let entries: Vec<Entry> = serde_json::from_slice(&content)?;
        Journal::from_entries(self.source_path.clone(), entries)?;
        atomic_write(&self.source_path, &content)
    }
}

/// Load the journal from the default location
pub fn load_journal() -> JotResult<Journal> {
    let journal_path = get_journal_path()
        .map_err(|e| JotError::Other(format!("Failed to get journal path: {}", e).into()))?;

    if !journal_path.exists() {
        return Err(JotError::Other(
            "Journal not found. Run 'xlog init' to create one.".into(),
        ));
    }

    load_from_path(journal_path)
}

/// Load a journal from a specific path
pub fn load_from_path(path: PathBuf) -> JotResult<Journal> {
    match fs::read_to_string(&path) {
        Ok(content) => {
            // Validate JSON structure before parsing
            if !content.trim().starts_with('[') || !content.trim().ends_with(']') {
                return Err(JotError::Other("Invalid journal file format".into()));
            }

            let entries: Vec<Entry> =
                serde_json::from_str(&content).map_err(JotError::SerdeError)?;
            Journal::from_entries(path, entries)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Journal::new(path)),
        Err(e) => Err(JotError::IoError(e)),
    }
}

pub fn save_journal(journal: &Journal) -> JotResult<()> {
    let content = serde_json::to_vec_pretty(journal.entries())?;
    if journal.path().exists() {
        Backup::from_journal(journal).create()?;
    }
    atomic_write(journal.path(), &content)
}

// A unique, exclusively created sibling file keeps failed writes from
// truncating the destination and prevents temporary-file name collisions.
fn atomic_write(path: &Path, content: &[u8]) -> JotResult<()> {
    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (temp_path, mut file) = loop {
        let sequence = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
        let temp_path = parent.join(format!(".xlog-{}-{sequence}.tmp", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => break (temp_path, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    };
    let result = (|| {
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp_path, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    result.map_err(Into::into)
}

/// Get the directory where the journal is stored
pub fn get_journal_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Some(path) = std::env::var_os("XLOG_HOME") {
        return Ok(PathBuf::from(path));
    }

    let mut path;
    if cfg!(debug_assertions) {
        path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    } else {
        path = dirs::home_dir().ok_or("Could not find home directory")?;
    }

    path.push(JOURNAL_DIR);
    Ok(path)
}

/// Get the path to the journal file
pub fn get_journal_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = get_journal_dir()?;
    path.push(JOURNAL_FILE);
    Ok(path)
}

pub fn init_journal(config: &Config) -> JotResult<()> {
    init_at(&get_journal_dir()?, config)
}

fn init_at(dir: &Path, config: &Config) -> JotResult<()> {
    fs::create_dir_all(dir)?;
    let journal = Journal::new(dir.join(JOURNAL_FILE));
    // Preserve existing journal bytes, including a corrupt journal, before
    // resetting it. Failure to save configuration must not erase entries.
    if journal.path().exists() {
        Backup::from_journal(&journal).create()?;
    }
    save_config_to(&dir.join(CONFIG_FILE), config)?;
    atomic_write(journal.path(), b"[]")
}

pub fn journal_exists() -> bool {
    get_journal_path().is_ok_and(|path| path.is_file())
}

// ! Config Related

/// Get the path to the config file
pub fn get_config_path() -> JotResult<PathBuf> {
    let mut path = get_journal_dir()
        .map_err(|e| JotError::Other(format!("Failed to get journal directory: {}", e).into()))?;
    path.push(CONFIG_FILE);
    Ok(path)
}

/// Load configuration without creating directories or changing files.
pub fn load_config() -> JotResult<Config> {
    load_config_from(&get_config_path()?)
}

fn load_config_from(path: &Path) -> JotResult<Config> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(toml::from_str(&content)?),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(error.into()),
    }
}

fn save_config_to(path: &Path, config: &Config) -> JotResult<()> {
    let content = toml::to_string_pretty(config)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if path.exists() {
        let backup_path = path.with_extension(format!("toml{BACKUP_EXTENSION}"));
        atomic_write(&backup_path, &fs::read(path)?)?;
    }
    atomic_write(path, content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn initialization_preserves_original_when_config_save_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(JOURNAL_FILE);
        fs::write(&path, "original journal").unwrap();
        fs::create_dir(dir.path().join(CONFIG_FILE)).unwrap();
        assert!(init_at(dir.path(), &Config::default()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original journal");
        assert_eq!(
            fs::read_to_string(path.with_extension("json.bak")).unwrap(),
            "original journal"
        );
    }

    #[test]
    fn initialization_creates_files_and_backs_up_before_reset() {
        let dir = TempDir::new().unwrap();
        init_at(dir.path(), &Config::default()).unwrap();
        assert!(load_config_from(&dir.path().join(CONFIG_FILE)).is_ok());
        let path = dir.path().join(JOURNAL_FILE);
        let mut journal = load_from_path(path.clone()).unwrap();
        journal.add_entry("keep a backup".into(), vec![]).unwrap();
        save_journal(&journal).unwrap();
        init_at(dir.path(), &Config::default()).unwrap();
        assert!(load_from_path(path.clone()).unwrap().entries().is_empty());
        assert_eq!(
            load_from_path(path.with_extension("json.bak"))
                .unwrap()
                .entries()[0]
                .body,
            "keep a backup"
        );
    }

    fn setup_test_env() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let journal_path = temp_dir.path().join("journal.json");
        let config_path = temp_dir.path().join("config.toml");
        (temp_dir, journal_path, config_path)
    }

    #[test]
    fn test_config_operations() {
        let (_temp_dir, _, config_path) = setup_test_env();

        // Create test config
        let mut config = Config::default();
        config.journal_cfg.body_tags = true;

        // Test saving
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        save_config_to(&config_path, &config).unwrap();

        // Test loading
        let loaded_config = load_config_from(&config_path).unwrap();
        assert!(loaded_config.journal_cfg.body_tags);
    }

    fn setup_temp_journal() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let journal_path = temp_dir.path().join("test_journal.json");
        (temp_dir, journal_path)
    }

    #[test]
    fn test_load_empty_journal() {
        let (_temp_dir, path) = setup_temp_journal();
        let journal = load_from_path(path.clone()).unwrap();
        assert!(journal.entries().is_empty());
        assert_eq!(*journal.path(), path);
    }

    #[test]
    fn test_save_and_load_journal() {
        let (_temp_dir, path) = setup_temp_journal();

        // Create and save a journal with one entry
        let mut journal = Journal::new(path.clone());
        journal
            .add_entry("Test entry".to_string(), vec![Tag::new("test".to_string())])
            .unwrap();
        save_journal(&journal).unwrap();

        // Load the journal and verify contents
        let loaded_journal = load_from_path(path).unwrap();
        assert_eq!(loaded_journal.entries().len(), 1);
        assert_eq!(loaded_journal.entries()[0].body, "Test entry");
        assert_eq!(loaded_journal.entries()[0].tags[0].name, "test");
    }

    #[test]
    fn test_get_journal_dir() {
        let dir = get_journal_dir().unwrap();
        if let Some(path) = std::env::var_os("XLOG_HOME") {
            assert_eq!(dir, PathBuf::from(path));
        } else if cfg!(debug_assertions) {
            assert_eq!(
                dir,
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(JOURNAL_DIR)
            );
        } else {
            assert_eq!(dir, dirs::home_dir().unwrap().join(JOURNAL_DIR));
        }
    }
}
