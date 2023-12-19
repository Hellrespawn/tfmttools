use camino::Utf8PathBuf;

pub fn rows_required_for_string(string: &str, width: usize) -> usize {
    string.lines().fold(0, |acc, el| {
        acc + console::measure_text_width(el).div_ceil(width)
    })
}

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
