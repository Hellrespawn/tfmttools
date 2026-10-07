use camino::Utf8PathBuf;
use thiserror::Error;

pub type TFMTResult<T = (), E = TFMTError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum TFMTError {
    #[error("{0}")]
    Template(String),

    #[error("Failed to render '{file}': {source}")]
    TemplateRender { file: Utf8PathBuf, source: Box<TFMTError> },

    #[error("No primary tag")]
    NoPrimaryTag(Utf8PathBuf),

    #[error("Unknown tag: '{0}'")]
    UnknownTag(String),

    #[error("Interpolated value contains a forbidden character: '{0}'")]
    ForbiddenCharacterError(String),
}
