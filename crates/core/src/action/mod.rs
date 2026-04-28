use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

mod case_insensitive_path;
mod rename_action;
mod validation;

pub use case_insensitive_path::{
    CaseInsensitivePathKey, CaseInsensitivePathSet,
};
pub use rename_action::RenameAction;
pub use validation::{FORBIDDEN_CHARACTERS, validate_rename_actions};

use crate::util::Utf8PathExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    MoveFile { source: Utf8PathBuf, target: Utf8PathBuf },
    CopyFile { source: Utf8PathBuf, target: Utf8PathBuf },
    RemoveFile(Utf8PathBuf),
    MakeDir(Utf8PathBuf),
    RemoveDir(Utf8PathBuf),
    EditTagValues { path: Utf8PathBuf, changes: Vec<TagValueChange> },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TagValueKind {
    Text,
    Locator,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TagValueChange {
    key: String,
    kind: TagValueKind,
    old_value: String,
    new_value: String,
}

impl TagValueChange {
    #[must_use]
    pub fn new(
        key: String,
        kind: TagValueKind,
        old_value: String,
        new_value: String,
    ) -> Self {
        Self { key, kind, old_value, new_value }
    }

    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    #[must_use]
    pub fn kind(&self) -> &TagValueKind {
        &self.kind
    }

    #[must_use]
    pub fn old_value(&self) -> &str {
        &self.old_value
    }

    #[must_use]
    pub fn new_value(&self) -> &str {
        &self.new_value
    }
}

impl Action {
    #[must_use]
    pub fn copy_from_rename_action(rename_action: &RenameAction) -> Self {
        Action::CopyFile {
            source: rename_action.source().to_owned().into_path_buf(),
            target: rename_action.target().to_owned().into_path_buf(),
        }
    }

    #[must_use]
    pub fn move_from_rename_action(rename_action: &RenameAction) -> Self {
        Action::MoveFile {
            source: rename_action.source().to_owned().into_path_buf(),
            target: rename_action.target().to_owned().into_path_buf(),
        }
    }

    #[must_use]
    pub fn is_rename_action(&self) -> bool {
        matches!(self, Self::MoveFile { .. } | Self::CopyFile { .. })
    }

    #[must_use]
    pub fn source(&self) -> Option<&Utf8Path> {
        match self {
            Action::CopyFile { source, .. }
            | Action::MoveFile { source, .. } => Some(source.as_path()),
            Action::RemoveFile(_)
            | Action::MakeDir(_)
            | Action::RemoveDir(_)
            | Action::EditTagValues { .. } => None,
        }
    }

    #[must_use]
    pub fn target(&self) -> &Utf8Path {
        match self {
            Action::CopyFile { target, .. }
            | Action::MoveFile { target, .. } => target.as_path(),
            Action::RemoveFile(path)
            | Action::MakeDir(path)
            | Action::RemoveDir(path)
            | Action::EditTagValues { path, .. } => path,
        }
    }
}
