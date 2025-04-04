use camino::{Utf8Path, Utf8PathBuf};

use crate::error::{TFMTError, TFMTResult};

#[must_use]
pub fn normalize_separators(string: &str) -> String {
    string
        .split(['\\', '/'])
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR)
}

#[derive(Debug, Default, Copy, Clone)]
pub enum ActionMode {
    #[default]
    Default,
    DryRun,
}
#[derive(Debug, Default, Copy, Clone)]
pub enum MoveMode {
    #[default]
    Auto,
    AlwaysCopy,
}

#[derive(Debug, Clone)]
pub struct Utf8Directory(Utf8PathBuf);

impl Utf8Directory {
    pub fn new(path: impl AsRef<Utf8Path>) -> TFMTResult<Self> {
        if path.as_ref().is_dir() || !path.as_ref().exists() {
            Ok(Self(path.as_ref().to_owned()))
        } else {
            Err(TFMTError::NotADirectory(path.as_ref().to_owned()))
        }
    }

    #[must_use]
    pub fn as_path(&self) -> &Utf8Path {
        &self.0
    }

    #[must_use]
    pub fn into_path(self) -> Utf8PathBuf {
        self.0
    }
}

impl std::fmt::Display for Utf8Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Debug)]
pub struct Utf8File(Utf8PathBuf);

impl Utf8File {
    pub fn new(path: impl AsRef<Utf8Path>) -> TFMTResult<Self> {
        if path.as_ref().is_file() || !path.as_ref().exists() {
            Ok(Self(path.as_ref().to_owned()))
        } else {
            Err(TFMTError::NotAFile(path.as_ref().to_owned()))
        }
    }

    #[must_use]
    pub fn as_path(&self) -> &Utf8Path {
        &self.0
    }

    #[must_use]
    pub fn into_path(self) -> Utf8PathBuf {
        self.0
    }
}

impl AsRef<Utf8Path> for Utf8File {
    fn as_ref(&self) -> &Utf8Path {
        self.0.as_ref()
    }
}

impl std::fmt::Display for Utf8File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
