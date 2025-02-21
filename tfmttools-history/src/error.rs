use camino::Utf8PathBuf;
use thiserror::Error;

pub type Result<T = (), E = HistoryError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum HistoryError {
    #[error("History file path exists, but is not a file: {0}")]
    PathIsNotAFile(Utf8PathBuf),
    #[error("Unable to save history to {expected}. Saved backup to {actual}")]
    SaveErrorWithBackup { expected: Utf8PathBuf, actual: Utf8PathBuf },
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
    #[error("Unable to serialize history: {source}")]
    Serialize { source: serde_json::Error },
    #[error("Unable to deserialize history: {source}")]
    Deserialize { source: serde_json::Error },
}
