use camino::Utf8PathBuf;
use thiserror::Error;

pub type TFMTResult<T = (), E = TFMTError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum TFMTError {
    #[error("No primary tag")]
    NoPrimaryTag(Utf8PathBuf),

    #[error("Unknown tag: '{0}'")]
    UnknownTag(String),

    #[error("Path exists but is not a directory: {0}")]
    NotADirectory(Utf8PathBuf),

    #[error("Path exists but is not a file: {0}")]
    NotAFile(Utf8PathBuf),

    #[error("Unexpected error while trying to move {0} to {1}: {2} ")]
    UnexpectedMoveError(Utf8PathBuf, Utf8PathBuf, String),

    // Passthrough errors
    #[error(transparent)]
    Camino(#[from] camino::FromPathBufError),

    #[error(transparent)]
    Id3(#[from] id3::Error),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Error while reading file: {0}\n{1}")]
    Lofty(Utf8PathBuf, lofty::error::LoftyError),

    #[error(transparent)]
    Minijinja(#[from] minijinja::Error),
}
