use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

mod rename_action;
mod validation;

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
            | Action::RemoveDir(_) => None,
        }
    }

    #[must_use]
    pub fn target(&self) -> &Utf8Path {
        match self {
            Action::CopyFile { target, .. }
            | Action::MoveFile { target, .. } => target.as_path(),
            Action::RemoveFile(path)
            | Action::MakeDir(path)
            | Action::RemoveDir(path) => path,
        }
    }
}
