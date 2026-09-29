pub mod config;
pub mod journal;

pub use journal::{Entry, Journal, Tag};

use crate::error::{AppError, AppResult};
use config::Config;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

const CONFIG_FILE: &str = "config.toml";
const APP_DIR: &str = "oxidlog";
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

    pub fn create(&self) -> AppResult<()> {
        // Read the current journal content to prepare for backup.
        let content = fs::read(&self.source_path).map_err(|source| {
            AppError::file("read journal for backup", &self.source_path, source)
        })?;
        if self.backup_path.exists() {
            let previous = fs::read(&self.backup_path)
                .map_err(|source| AppError::file("read backup", &self.backup_path, source))?;
            atomic_write(&self.old_backup_path, &previous)?;
        }
        atomic_write(&self.backup_path, &content)
    }

    pub fn restore(&self) -> AppResult<()> {
        let content = fs::read(&self.backup_path)
            .map_err(|source| AppError::file("read backup", &self.backup_path, source))?;
        let entries: Vec<Entry> =
            serde_json::from_slice(&content).map_err(|source| AppError::JournalData {
                path: self.backup_path.clone(),
                source,
            })?;
        Journal::from_entries(self.source_path.clone(), entries)?;
        atomic_write(&self.source_path, &content)
    }
}

pub fn load_journal() -> AppResult<Journal> {
    load_from_path(get_journal_path()?)
}

pub fn load_from_path(path: PathBuf) -> AppResult<Journal> {
    let content = fs::read(&path).map_err(|source| {
        if source.kind() == io::ErrorKind::NotFound {
            AppError::JournalNotFound(path.clone())
        } else {
            AppError::file("read journal", &path, source)
        }
    })?;
    let entries = serde_json::from_slice(&content).map_err(|source| AppError::JournalData {
        path: path.clone(),
        source,
    })?;
    Journal::from_entries(path, entries)
}

pub fn save_journal(journal: &Journal) -> AppResult<()> {
    let content = serde_json::to_vec_pretty(journal.entries())?;
    if journal.path().exists() {
        Backup::from_journal(journal).create()?;
    }
    atomic_write(journal.path(), &content)
}

/// Atomically write content to a file (make a temporary file and then rename/"move" it to the target path)
fn atomic_write(path: &Path, content: &[u8]) -> AppResult<()> {
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
            Err(error) => return Err(AppError::file("create temporary file for", path, error)),
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
    result.map_err(|source| AppError::file("write", path, source))
}

// Directory stuff for xdg based locations and overrides

fn override_dir() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XLOG_HOME") {
        return Some(PathBuf::from(path));
    }
    if cfg!(debug_assertions) {
        return Some(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".oxidlog"));
    }
    None
}

pub fn get_journal_dir() -> AppResult<PathBuf> {
    if let Some(path) = override_dir() {
        return Ok(path);
    }
    Ok(dirs::data_dir()
        .ok_or(AppError::HomeUnavailable)?
        .join(APP_DIR))
}

pub fn get_config_dir() -> AppResult<PathBuf> {
    if let Some(path) = override_dir() {
        return Ok(path);
    }
    Ok(dirs::config_dir()
        .ok_or(AppError::HomeUnavailable)?
        .join(APP_DIR))
}

pub fn get_journal_path() -> AppResult<PathBuf> {
    let mut path = get_journal_dir()?;
    path.push(JOURNAL_FILE);
    Ok(path)
}

pub fn init_journal(config: &Config) -> AppResult<()> {
    init_at(&get_journal_dir()?, &get_config_dir()?, config)
}

fn init_at(dir: &Path, config_dir: &Path, config: &Config) -> AppResult<()> {
    for d in [dir, config_dir] {
        fs::create_dir_all(d).map_err(|source| AppError::file("create directory", d, source))?;
    }
    let journal = Journal::new(dir.join(JOURNAL_FILE));
    if journal.path().exists() {
        Backup::from_journal(&journal).create()?;
    }
    save_config_to(&config_dir.join(CONFIG_FILE), config)?;
    atomic_write(journal.path(), b"[]")
}

pub fn journal_exists() -> AppResult<bool> {
    let path = get_journal_path()?;
    match fs::metadata(&path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(AppError::file("inspect journal", &path, source)),
    }
}

pub fn get_config_path() -> AppResult<PathBuf> {
    let mut path = get_config_dir()?;
    path.push(CONFIG_FILE);
    Ok(path)
}

pub fn load_config() -> AppResult<Config> {
    load_config_from(&get_config_path()?)
}

fn load_config_from(path: &Path) -> AppResult<Config> {
    match fs::read_to_string(path) {
        Ok(content) => toml::from_str(&content).map_err(|source| AppError::ConfigParse {
            path: path.to_owned(),
            source,
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Config::default()),
        Err(error) => Err(AppError::file("read configuration", path, error)),
    }
}

fn save_config_to(path: &Path, config: &Config) -> AppResult<()> {
    let content = toml::to_string_pretty(config)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|source| AppError::file("create directory", parent, source))?;
    }
    if path.exists() {
        let backup_path = path.with_extension(format!("toml{BACKUP_EXTENSION}"));
        let previous = fs::read(path)
            .map_err(|source| AppError::file("read configuration for backup", path, source))?;
        atomic_write(&backup_path, &previous)?;
    }
    atomic_write(path, content.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn failed_backup_keeps_the_existing_backup() {
        let dir = TempDir::new().unwrap();
        let backup = Backup::from_journal(&Journal::new(dir.path().join(JOURNAL_FILE)));
        fs::write(&backup.backup_path, "previous backup").unwrap();
        assert!(backup.create().is_err());
        assert_eq!(
            fs::read_to_string(&backup.backup_path).unwrap(),
            "previous backup"
        );
        assert!(!backup.old_backup_path.exists());
    }

    #[test]
    fn initialization_preserves_original_when_config_save_fails() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(JOURNAL_FILE);
        fs::write(&path, "original journal").unwrap();
        fs::create_dir(dir.path().join(CONFIG_FILE)).unwrap();
        assert!(init_at(dir.path(), dir.path(), &Config::default()).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "original journal");
        assert_eq!(
            fs::read_to_string(path.with_extension("json.bak")).unwrap(),
            "original journal"
        );
    }

    #[test]
    fn initialization_creates_files_and_backs_up_before_reset() {
        let dir = TempDir::new().unwrap();
        init_at(dir.path(), dir.path(), &Config::default()).unwrap();
        assert!(load_config_from(&dir.path().join(CONFIG_FILE)).is_ok());
        let path = dir.path().join(JOURNAL_FILE);
        let mut journal = load_from_path(path.clone()).unwrap();
        journal.add_entry("keep a backup".into(), vec![]).unwrap();
        save_journal(&journal).unwrap();
        init_at(dir.path(), dir.path(), &Config::default()).unwrap();
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
        config.journal_cfg.inline_tags = true;

        // Test saving
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        save_config_to(&config_path, &config).unwrap();

        // Test loading
        let loaded_config = load_config_from(&config_path).unwrap();
        assert!(loaded_config.journal_cfg.inline_tags);
    }

    fn setup_temp_journal() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let journal_path = temp_dir.path().join("test_journal.json");
        (temp_dir, journal_path)
    }

    #[test]
    fn test_load_empty_journal() {
        let (_temp_dir, path) = setup_temp_journal();
        assert!(matches!(
            load_from_path(path.clone()),
            Err(AppError::JournalNotFound(_))
        ));
        fs::write(&path, "[]").unwrap();
        let journal = load_from_path(path.clone()).unwrap();
        assert!(journal.entries().is_empty());
        assert_eq!(*journal.path(), path);
    }

    #[test]
    fn test_save_and_load_journal() {
        let (_temp_dir, path) = setup_temp_journal();

        let mut journal = Journal::new(path.clone());
        journal
            .add_entry("Test entry".to_string(), vec![Tag::new("test".to_string())])
            .unwrap();
        save_journal(&journal).unwrap();

        let loaded_journal = load_from_path(path).unwrap();
        assert_eq!(loaded_journal.entries().len(), 1);
        assert_eq!(loaded_journal.entries()[0].body, "Test entry");
        assert_eq!(loaded_journal.entries()[0].tags[0].name, "test");
    }

    #[test]
    fn test_xdg_dirs() {
        let data = get_journal_dir().unwrap();
        let config = get_config_dir().unwrap();
        if let Some(path) = std::env::var_os("XLOG_HOME") {
            assert_eq!(data, PathBuf::from(&path));
            assert_eq!(config, PathBuf::from(path));
        } else if cfg!(debug_assertions) {
            let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".oxidlog");
            assert_eq!(data, dev);
            assert_eq!(config, dev);
        } else {
            assert_eq!(data, dirs::data_dir().unwrap().join(APP_DIR));
            assert_eq!(config, dirs::config_dir().unwrap().join(APP_DIR));
        }
    }
}
