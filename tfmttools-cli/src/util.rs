use std::io::Write;

use camino::Utf8PathBuf;
use color_eyre::Result;

use crate::TERM;

#[derive(Debug, Clone, PartialEq)]
pub enum PathOrString {
    Path(Utf8PathBuf, String),
    String(String),
}

impl From<String> for PathOrString {
    fn from(string: String) -> Self {
        let path = Utf8PathBuf::from(&string);

        if path.is_file() {
            Self::Path(path, string)
        } else {
            Self::String(string)
        }
    }
}

impl PathOrString {
    pub fn as_str(&self) -> &str {
        match self {
            PathOrString::String(s) | PathOrString::Path(_, s) => s,
        }
    }
}
