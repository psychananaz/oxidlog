pub mod config;
pub mod journal;

pub use journal::{Entry, Journal, Tag};
use serde::de::Error;

use crate::error::{JotError, JotResult};
use config::Config;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::PathBuf;

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
        if self.backup_path.exists() {
            fs::copy(&self.backup_path, &self.source_path)
                .map_err(|e| JotError::Other(format!("Failed to restore backup: {}", e).into()))?;
        } else {
            return Err(JotError::Other("Backup not found".into()));
        }

        Ok(())
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
            Ok(Journal::from_entries(path, entries))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Journal::new(path)),
        Err(e) => Err(JotError::IoError(e)),
    }
}

pub fn save_journal(journal: &Journal) -> JotResult<()> {
    let backup = Backup::from_journal(journal);
    if journal.path().exists() {
        backup.create()?;
    }

    // Serialize entries
    let serialized_entries =
        serde_json::to_string_pretty(journal.entries()).map_err(JotError::SerdeError)?;

    // Write to temporary file first
    let temp_path = journal.path().with_extension("json.tmp");
    {
        let mut temp_file = File::create(&temp_path).map_err(JotError::IoError)?;
        temp_file
            .write_all(serialized_entries.as_bytes())
            .map_err(JotError::IoError)?;
        temp_file.sync_all().map_err(JotError::IoError)?;
    }

    // Atomically rename temporary file to actual journal file
    fs::rename(&temp_path, journal.path()).map_err(JotError::IoError)?;

    Ok(())
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

// Update init_journal to take config
pub fn init_journal(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let journal_path = if get_journal_path()?.exists() {
        get_journal_path()?
    } else {
        let path = get_journal_path()?;
        std::fs::create_dir_all(path.parent().unwrap())?;
        path
    };

    // create all parent directories if they don't exist
    fs::create_dir_all(journal_path.parent().unwrap())?;
    fs::write(journal_path, "[]")?;

    // Initialize the config file
    save_config(config)?;

    Ok(())
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

/// Load configuration from the config file with validation
pub fn load_config() -> JotResult<Config> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&config_path).map_err(JotError::IoError)?;

    // Basic TOML validation
    if !content.trim().starts_with('[') {
        return Err(JotError::TomlParseError(toml::de::Error::custom(
            "Invalid TOML format: must start with a table header",
        )));
    }

    let config: Config = toml::from_str(&content).map_err(JotError::TomlParseError)?;

    // Validate export directory path
    if !config.journal_cfg.export_dir.is_empty() {
        let export_path = PathBuf::from(&config.journal_cfg.export_dir);
        if export_path.is_absolute() && !export_path.exists() {
            fs::create_dir_all(&export_path).map_err(|e| {
                JotError::Other(format!("Failed to create export directory: {}", e).into())
            })?;
        }
    }

    Ok(config)
}

/// Save configuration to the config file with atomic write
pub fn save_config(config: &Config) -> JotResult<()> {
    let config_path = get_config_path()?;
    let temp_path = config_path.with_extension("toml.tmp");

    // Create parent directories if they don't exist
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            JotError::Other(format!("Failed to create config directory: {}", e).into())
        })?;
    }

    // Serialize config with pretty formatting
    let content = toml::to_string_pretty(config).map_err(JotError::TomlSerializeError)?;

    // Write to temporary file first
    {
        let mut temp_file = File::create(&temp_path).map_err(JotError::IoError)?;
        temp_file
            .write_all(content.as_bytes())
            .map_err(JotError::IoError)?;
        temp_file.sync_all().map_err(JotError::IoError)?;
    }

    // Backup existing config if it exists
    if config_path.exists() {
        let backup_path = config_path.with_extension(format!("toml{}", BACKUP_EXTENSION));
        fs::copy(&config_path, &backup_path)
            .map_err(|e| JotError::Other(format!("Failed to backup config: {}", e).into()))?;
    }

    // Atomically rename temporary file to actual config file
    fs::rename(&temp_path, &config_path).map_err(JotError::IoError)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
        fs::write(&config_path, toml::to_string(&config).unwrap()).unwrap();

        // Test loading
        let loaded_config: Config =
            toml::from_str(&fs::read_to_string(config_path).unwrap()).unwrap();
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
        journal.add_entry(Entry::new(
            0,
            "Test entry".to_string(),
            vec![Tag::new("test".to_string())],
        ));
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
