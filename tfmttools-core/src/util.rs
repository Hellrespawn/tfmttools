use std::path::Path;

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::{TFMTError, TFMTResult};

#[must_use]
pub fn normalize_separators(string: &str) -> String {
    string
        .split(['\\', '/'])
        .collect::<Vec<&str>>()
        .join(std::path::MAIN_SEPARATOR_STR)
}

#[derive(Debug, Default, Copy, Clone)]
pub enum FSMode {
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

pub trait Utf8PathExt: Sized {
    fn as_path(&self) -> &Utf8Path;

    fn into_path_buf(self) -> Utf8PathBuf;

    fn exists(&self) -> bool;
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Utf8Directory(Utf8PathBuf);

impl Utf8Directory {
    pub fn new(path: impl AsRef<Utf8Path>) -> TFMTResult<Self> {
        if path.as_ref().is_dir() || !path.as_ref().exists() {
            Ok(Self(path.as_ref().to_owned()))
        } else {
            Err(TFMTError::NotADirectory(path.as_ref().to_owned()))
        }
    }

    pub fn new_unchecked(path: impl AsRef<Utf8Path>) -> Self {
        Self(path.as_ref().to_owned())
    }

    #[must_use]
    pub fn ancestors(self) -> Vec<Utf8Directory> {
        self.0
            .ancestors()
            .map(|path| Utf8Directory::new(path).expect("msg"))
            .collect()
    }

    pub fn join(
        &self,
        path: impl AsRef<Utf8Path>,
    ) -> TFMTResult<Utf8Directory> {
        let joined_path = self.as_path().join(path);

        Self::new(joined_path)
    }

    pub fn join_file(
        &self,
        path: impl AsRef<Utf8Path>,
    ) -> TFMTResult<Utf8File> {
        let joined_path = self.as_path().join(path);

        Utf8File::new(joined_path)
    }
}

impl Utf8PathExt for Utf8Directory {
    fn as_path(&self) -> &Utf8Path {
        &self.0
    }

    fn into_path_buf(self) -> Utf8PathBuf {
        self.0
    }

    fn exists(&self) -> bool {
        self.0.exists()
    }
}

impl AsRef<Utf8Path> for Utf8Directory {
    fn as_ref(&self) -> &Utf8Path {
        self.0.as_ref()
    }
}

impl AsRef<Path> for Utf8Directory {
    fn as_ref(&self) -> &Path {
        self.0.as_path().as_ref()
    }
}

impl std::fmt::Display for Utf8Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(
    Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct Utf8File(Utf8PathBuf);

impl Utf8File {
    pub fn new(path: impl AsRef<Utf8Path>) -> TFMTResult<Self> {
        if path.as_ref().is_file() || !path.as_ref().exists() {
            Ok(Self(path.as_ref().to_owned()))
        } else {
            Err(TFMTError::NotAFile(path.as_ref().to_owned()))
        }
    }

    pub fn new_unchecked(path: impl AsRef<Utf8Path>) -> Self {
        Self(path.as_ref().to_owned())
    }

    #[must_use]
    pub fn parent(&self) -> Utf8Directory {
        let path = self.0.parent().expect("Utf8File should have parent");

        Utf8Directory::new(path).expect("Utf8File::parent should directory.")
    }

    #[must_use]
    pub fn components(&'_ self) -> Vec<Utf8Component<'_>> {
        self.0.components().collect()
    }

    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.0.extension()
    }

    #[must_use]
    pub fn file_name(&self) -> &str {
        self.0.file_name().expect("Utf8File should always have a file name")
    }
}

impl Utf8PathExt for Utf8File {
    fn as_path(&self) -> &Utf8Path {
        &self.0
    }

    fn into_path_buf(self) -> Utf8PathBuf {
        self.0
    }

    fn exists(&self) -> bool {
        self.0.exists()
    }
}

impl AsRef<Utf8Path> for Utf8File {
    fn as_ref(&self) -> &Utf8Path {
        self.0.as_ref()
    }
}

impl AsRef<Path> for Utf8File {
    fn as_ref(&self) -> &Path {
        self.0.as_path().as_ref()
    }
}

impl std::fmt::Display for Utf8File {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
