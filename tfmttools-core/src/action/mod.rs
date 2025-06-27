use std::collections::HashSet;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

mod validation;

pub use validation::{FORBIDDEN_CHARACTERS, validate_rename_actions};

use crate::error::TFMTResult;
use crate::util::{Utf8Directory, Utf8File, Utf8PathExt};

#[derive(PartialEq, Serialize, Deserialize, Debug, Clone)]
pub struct RenameAction {
    source: Utf8File,
    target: Utf8File,
}

impl RenameAction {
    #[must_use]
    pub fn new(source: Utf8File, target: Utf8File) -> Self {
        Self { source, target }
    }

    pub fn from_path_bufs(
        source: Utf8PathBuf,
        target: Utf8PathBuf,
    ) -> TFMTResult<Self> {
        Ok(Self {
            source: Utf8File::new(source)?,
            target: Utf8File::new(target)?,
        })
    }

    pub fn separate_unchanged_destinations(
        rename_actions: Vec<RenameAction>,
    ) -> (Vec<RenameAction>, Vec<Utf8File>) {
        let (actions, unchanged_paths) = rename_actions
            .into_iter()
            .partition(RenameAction::source_differs_from_target);

        (actions, unchanged_paths.into_iter().map(|ra| ra.target).collect())
    }

    #[must_use]
    pub fn source(&self) -> &Utf8File {
        &self.source
    }

    #[must_use]
    pub fn target(&self) -> &Utf8File {
        &self.target
    }

    #[must_use]
    pub fn source_differs_from_target(&self) -> bool {
        self.source() != self.target()
    }

    pub fn create_actions(rename_actions: Vec<RenameAction>) -> Vec<Action> {
        let target_paths =
            rename_actions.iter().map(RenameAction::target).collect::<Vec<_>>();

        let mut actions =
            Self::list_all_intermediate_paths_of_files(&target_paths)
                .into_iter()
                .map(|dir| Action::MakeDir(dir.as_path().to_owned()))
                .collect::<Vec<_>>();

        actions.extend(rename_actions.into_iter().map(Action::MoveFile));

        actions
    }

    fn list_all_intermediate_paths_of_files(
        paths: &[&Utf8File],
    ) -> Vec<Utf8Directory> {
        let mut directories = paths
            .iter()
            .flat_map(|p| {
                let parent = p.parent();

                parent
                    .ancestors()
                    .into_iter()
                    .filter(|p| {
                        let path: &Utf8Path = p.as_ref();

                        !path.as_str().is_empty()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        directories.sort();

        directories
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Action {
    MoveFile(RenameAction),
    CopyFile(RenameAction),
    RemoveFile(Utf8PathBuf),
    MakeDir(Utf8PathBuf),
    RemoveDir(Utf8PathBuf),
}

impl Action {
    #[must_use]
    pub fn is_rename_action(&self) -> bool {
        matches!(self, Self::MoveFile { .. } | Self::CopyFile(..))
    }

    #[must_use]
    pub fn source(&self) -> Option<&Utf8Path> {
        match self {
            Action::CopyFile(rename_action)
            | Action::MoveFile(rename_action) => {
                Some(rename_action.source().as_path())
            },
            Action::RemoveFile(_)
            | Action::MakeDir(_)
            | Action::RemoveDir(_) => None,
        }
    }

    #[must_use]
    pub fn target(&self) -> &Utf8Path {
        match self {
            Action::CopyFile(rename_action)
            | Action::MoveFile(rename_action) => {
                rename_action.target().as_path()
            },
            Action::RemoveFile(path)
            | Action::MakeDir(path)
            | Action::RemoveDir(path) => path,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_get_unique_source_directories_relative() {
        let paths = [
            "a/b/c/d.mp3",
            "a/b/c/e.mp3",
            "a/b/c/f.mp3",
            "a/b/c/g.mp3",
            "a/b/h/i.mp3",
            "a/b/h/j.mp3",
        ];

        let reference = ["a", "a/b", "a/b/c", "a/b/h"]
            .into_iter()
            .map(Utf8Directory::new)
            .collect::<TFMTResult<Vec<_>>>()
            .unwrap();

        let paths: Vec<Utf8File> =
            paths.iter().map(Utf8File::new).collect::<TFMTResult<_>>().unwrap();

        let paths_ref = paths.iter().collect::<Vec<_>>();

        let directories =
            RenameAction::list_all_intermediate_paths_of_files(&paths_ref);

        assert_eq!(directories, reference);
    }

    #[test]
    fn test_get_unique_source_directories_absolute() {
        let paths = [
            "/a/b/c/d.mp3",
            "/a/b/c/e.mp3",
            "/a/b/c/f.mp3",
            "/a/b/c/g.mp3",
            "/a/b/h/i.mp3",
            "/a/b/h/j.mp3",
        ];

        let reference = ["/", "/a", "/a/b", "/a/b/c", "/a/b/h"]
            .into_iter()
            .map(Utf8Directory::new)
            .collect::<TFMTResult<Vec<_>>>()
            .unwrap();

        let paths = paths
            .iter()
            .map(Utf8File::new)
            .collect::<TFMTResult<Vec<_>>>()
            .unwrap();

        let paths_ref = paths.iter().collect::<Vec<_>>();

        let directories =
            RenameAction::list_all_intermediate_paths_of_files(&paths_ref);

        assert_eq!(directories, reference);
    }
}
