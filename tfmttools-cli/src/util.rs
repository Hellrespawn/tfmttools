use camino::Utf8PathBuf;

#[derive(Debug, Clone, PartialEq)]
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

impl FileOrName {
    pub fn as_str(&self) -> &str {
        match self {
            FileOrName::Name(s) | FileOrName::File(_, s) => s,
        }
    }
}
