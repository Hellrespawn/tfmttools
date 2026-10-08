use camino::Utf8PathBuf;
use tfmttools_core::error::TFMTError;
use thiserror::Error;

pub type FsResult<T = (), E = FsError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum FsError {
    #[error("File recovery error: {0}")]
    Recovery(String),

    #[error("Path exists but is not a directory: {0}")]
    NotADirectory(Utf8PathBuf),

    #[error("Path exists but is not a file: {0}")]
    NotAFile(Utf8PathBuf),

    #[error("Unexpected error while trying to move {0} to {1}: {2} ")]
    UnexpectedMoveError(Utf8PathBuf, Utf8PathBuf, String),

    #[error("File is too big for checksum: {0}")]
    FileTooLargeError(Utf8PathBuf),

    #[error("Error while reading file: {0}\n{1}")]
    Lofty(Utf8PathBuf, #[source] lofty::error::FileParseError),

    #[error("Error while writing file: {0}\n{1}")]
    LoftyFileWrite(Utf8PathBuf, #[source] lofty::error::FileEncodingError),

    #[error("Error while writing tag: {0}\n{1}")]
    LoftyTagWrite(Utf8PathBuf, #[source] lofty::error::FileEncodingError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    Camino(#[from] camino::FromPathBufError),

    #[error(transparent)]
    Core(#[from] TFMTError),
}
