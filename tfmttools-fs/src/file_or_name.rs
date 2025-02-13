use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum FileOrName {
    File(Utf8PathBuf, String),
    Name(String),
}

impl From<String> for FileOrName {
    fn from(string: String) -> Self {
        let path = Utf8PathBuf::from(&string);

        if path.is_file() {
            Self::File(path, string)
        } else {
            Self::Name(string)
        }
    }
}

impl From<&str> for FileOrName {
    fn from(string: &str) -> Self {
        FileOrName::from(string.to_owned())
    }
}

impl FileOrName {
    pub fn as_str(&self) -> &str {
        match self {
            FileOrName::Name(s) | FileOrName::File(_, s) => s,
        }
    }
}

impl std::fmt::Display for FileOrName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
