use std::{
    io,
    path::{Path, PathBuf},
    process::ExitStatus,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Failed to {operation} '{}': {source}", path.display())]
    File {
        operation: &'static str,
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("Invalid journal '{}': {source}", path.display())]
    JournalData {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Failed to encode JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Invalid configuration '{}': {source}", path.display())]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },
    #[error("Failed to encode configuration: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),
    #[error("Entry with ID {0} not found")]
    EntryNotFound(usize),
    #[error("Duplicate entry ID {0} in journal")]
    DuplicateId(usize),
    #[error("Entry IDs exhausted")]
    IdExhausted,
    #[error("Entry body cannot be empty")]
    EmptyBody,
    #[error("Start date must not be after end date")]
    InvalidDateRange,
    #[error("No entries to remove")]
    NoEntriesSelected,
    #[error("Journal not found at '{}'. Run 'xlog init' to create one.", .0.display())]
    JournalNotFound(PathBuf),
    #[error("Could not find home directory")]
    HomeUnavailable,
    #[error("Failed to get {prompt}: {source}")]
    Prompt {
        prompt: &'static str,
        #[source]
        source: dialoguer::Error,
    },
    #[error("Cannot open exported file on unsupported platform '{0}'")]
    UnsupportedPlatform(&'static str),
    #[error("Opening '{}' with '{command}' failed: {status}", path.display())]
    OpenExport {
        path: PathBuf,
        command: &'static str,
        status: ExitStatus,
    },
    #[error("No available export filename")]
    ExportNamesExhausted,
    #[error("{command}: {source}")]
    Command {
        command: &'static str,
        #[source]
        source: Box<AppError>,
    },
}

impl AppError {
    pub fn file(operation: &'static str, path: &Path, source: io::Error) -> Self {
        Self::File {
            operation,
            path: path.to_owned(),
            source,
        }
    }

    pub fn in_command(self, command: &'static str) -> Self {
        Self::Command {
            command,
            source: Box::new(self),
        }
    }

    pub fn is_broken_pipe(&self) -> bool {
        match self {
            Self::Io(error) => error.kind() == io::ErrorKind::BrokenPipe,
            Self::Command { source, .. } => source.is_broken_pipe(),
            _ => false,
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn command_context_preserves_structured_causes() {
        let error = AppError::file(
            "read",
            Path::new("journal.json"),
            io::ErrorKind::PermissionDenied.into(),
        )
        .in_command("search");
        assert!(error.source().is_some());
        let AppError::Command { source, .. } = &error else {
            panic!("missing command context")
        };
        let cause = source.as_ref();
        assert!(
            matches!(cause, AppError::File { source, .. } if source.kind() == io::ErrorKind::PermissionDenied)
        );
        assert!(error
            .to_string()
            .contains("search: Failed to read 'journal.json'"));
        assert!(AppError::Io(io::ErrorKind::BrokenPipe.into())
            .in_command("view")
            .is_broken_pipe());
    }
}
