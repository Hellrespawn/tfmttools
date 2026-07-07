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

    #[error("File is too big for checksum: {0}")]
    FileTooLargeError(Utf8PathBuf),

    #[error("Interpolated value contains a forbidden character: '{0}'")]
    ForbiddenCharacterError(String),

    #[error("Failed to parse frontmatter TOML in template '{0}': {1}")]
    FrontmatterParse(String, toml::de::Error),

    #[error(
        "Unterminated frontmatter block in template '{0}': missing closing '+++'"
    )]
    UnterminatedFrontmatter(String),

    #[error(
        "Duplicate argument name '{1}' declared in frontmatter of template '{0}'"
    )]
    DuplicateArgumentName(String, String),

    #[error("Missing required argument '{1}' for template '{0}': {2}")]
    MissingRequiredArgument(String, String, String),

    #[error(
        "Template '{0}' accepts at most {1} argument(s), but {2} were supplied"
    )]
    TooManyArguments(String, usize, usize),

    #[error(
        "Argument '{1}' for template '{0}' has an invalid value '{3}': {2}"
    )]
    InvalidArgumentValue(String, String, String, String),

    #[error(
        "Template '{0}' uses indexed `args[N]` access, which is not allowed once a frontmatter block is present"
    )]
    IndexedArgsWithFrontmatter(String),

    // Passthrough errors
    #[error(transparent)]
    Camino(#[from] camino::FromPathBufError),

    #[error(transparent)]
    Ignore(#[from] ignore::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error("Error while reading file: {0}\n{1}")]
    Lofty(Utf8PathBuf, lofty::error::LoftyError),

    #[error(transparent)]
    Minijinja(#[from] minijinja::Error),
}
