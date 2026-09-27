use crate::{
    error::{JotError, JotResult},
    storage::{config::Config, Entry, Journal},
};
use chrono::Local;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub struct ExportArgs {
    #[clap(value_enum)]
    /// The format to export the journal in (json, csv, plain)
    pub format: ExportFormat,
    #[clap(short, long)]
    /// Open the exported file with the default program
    pub open: bool,
}

#[derive(clap::ValueEnum, Clone, Copy)]
pub enum ExportFormat {
    Json,
    Csv,
    Plain,
}

pub fn execute(journal: &Journal, args: ExportArgs, config: &Config) -> JotResult<()> {
    let entries = journal.entries();
    let export_dir = journal
        .path()
        .parent()
        .unwrap_or(journal.path())
        .join(&config.journal_cfg.export_dir);
    fs::create_dir_all(&export_dir)?;

    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
    let filename = generate_filename(args.format, timestamp);

    let content = match args.format {
        ExportFormat::Json => export_to_json(entries)?,
        ExportFormat::Csv => export_to_csv(entries),
        ExportFormat::Plain => export_to_plain(entries),
    };

    let export_path = write_export(&export_dir, &filename, &content)?;

    if args.open {
        open_exported_file(&export_path)?;
    }

    println!("Journal exported successfully to {}", export_path.display());
    Ok(())
}

fn generate_filename(format: ExportFormat, timestamp: impl std::fmt::Display) -> String {
    match format {
        ExportFormat::Json => format!("journal_{}.json", timestamp),
        ExportFormat::Csv => format!("journal_{}.csv", timestamp),
        ExportFormat::Plain => format!("journal_{}.txt", timestamp),
    }
}

fn export_to_json(entries: &[Entry]) -> JotResult<String> {
    serde_json::to_string_pretty(&entries).map_err(JotError::SerdeError)
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\r', '\n']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn export_to_csv(entries: &[Entry]) -> String {
    let mut csv = String::from("date,body,tags\r\n");
    for entry in entries {
        let tags = entry
            .tags
            .iter()
            .map(|tag| tag.name.as_str())
            .collect::<Vec<_>>()
            .join(",");
        csv.push_str(&format!(
            "{},{},{}\r\n",
            entry.date,
            csv_field(&entry.body),
            csv_field(&tags)
        ));
    }
    csv
}

fn write_export(dir: &Path, filename: &str, content: &str) -> JotResult<PathBuf> {
    let filename = Path::new(filename);
    let stem = filename.file_stem().unwrap().to_string_lossy();
    let extension = filename.extension().unwrap().to_string_lossy();
    for suffix in 0_u64.. {
        let name = if suffix == 0 {
            filename.to_path_buf()
        } else {
            PathBuf::from(format!("{stem}_{suffix}.{extension}"))
        };
        let path = dir.join(name);
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                let result = file
                    .write_all(content.as_bytes())
                    .and_then(|()| file.sync_all());
                drop(file);
                if let Err(error) = result {
                    let _ = fs::remove_file(&path);
                    return Err(error.into());
                }
                return Ok(path);
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err(JotError::ExportError("No available export filename".into()))
}

fn export_to_plain(entries: &[Entry]) -> String {
    let mut text = String::new();
    for entry in entries {
        text.push_str(&format!("Date: {}\n", entry.date));
        if !entry.tags.is_empty() {
            let tags_str = entry
                .tags
                .iter()
                .map(|t| t.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            text.push_str(&format!("Tags: {}\n", tags_str));
        }
        text.push_str(&format!("\n{}\n", entry.body));
        text.push_str("\n---\n\n");
    }
    text
}

fn open_exported_file(export_path: &Path) -> JotResult<()> {
    let platform = std::env::consts::OS;
    let command = match platform {
        "linux" => "xdg-open",
        "macos" => "open",
        "windows" => "explorer.exe",
        _ => {
            return Err(JotError::ExportError(format!(
                "Cannot open exported file: unsupported platform '{}'",
                platform
            )));
        }
    };

    let status = std::process::Command::new(command)
        .arg(export_path)
        .status()?;

    if !status.success() {
        return Err(JotError::ExportError(format!(
            "Failed to open exported file '{}' with system command '{}'",
            export_path.display(),
            command
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::Tag;

    #[test]
    fn csv_preserves_quotes_commas_and_line_breaks() {
        let mut entry = Entry::new(
            0,
            "one, \"two\"\nthree".into(),
            vec![Tag::new("a".into()), Tag::new("b".into())],
        );
        entry.date = chrono::NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        assert_eq!(
            export_to_csv(&[entry]),
            "date,body,tags\r\n2025-01-02,\"one, \"\"two\"\"\nthree\",\"a,b\"\r\n"
        );
    }

    #[test]
    fn repeated_exports_never_overwrite_previous_content() {
        let dir = tempfile::tempdir().unwrap();
        let first = write_export(dir.path(), "journal_20250101.json", "first").unwrap();
        let second = write_export(dir.path(), "journal_20250101.json", "second").unwrap();
        assert_ne!(first, second);
        assert_eq!(fs::read_to_string(first).unwrap(), "first");
        assert_eq!(fs::read_to_string(second).unwrap(), "second");
    }
}
