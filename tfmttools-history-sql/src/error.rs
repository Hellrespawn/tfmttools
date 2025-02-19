use camino::Utf8PathBuf;
use thiserror::Error;

pub type Result<T = (), E = HistoryError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum HistoryError {
    #[error("History file path exists, but is not a file: {0}")]
    PathIsNotAFile(Utf8PathBuf),
    #[error("Unable to read temporary directory: {source}")]
    FromPathBufError {
        #[from]
        source: camino::FromPathBufError,
    },
    #[error("Unable to read file: {source}")]
    ReadFile {
        #[from]
        source: std::io::Error,
    },
    #[error("SQL Error: {source}")]
    SQLError {
        #[from]
        source: rusqlite::Error,
    },
    #[error("Migration error: {source}")]
    MigrationError {
        #[from]
        source: rusqlite_migration::Error,
    },
}
