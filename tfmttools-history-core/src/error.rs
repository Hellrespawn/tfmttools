use camino::Utf8PathBuf;
use thiserror::Error;

pub type Result<T = (), E = HistoryError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum HistoryError {
    #[error("Unable to load history: {0}")]
    LoadError(String),

    #[error("Unable to save history: {0}")]
    SaveError(String),

    #[error("Unable to remove history: {0}")]
    RemoveError(String),

    #[error("Unable to save history: {0}. Saved backup to {1}.")]
    SaveErrorWithBackup(String, Utf8PathBuf),

    #[error("{0}")]
    MiscError(String),
}
