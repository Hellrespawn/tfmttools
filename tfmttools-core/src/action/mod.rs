use std::collections::HashSet;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

mod validation;

pub use validation::{FORBIDDEN_CHARACTERS, validate_rename_actions};

#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub struct RenameAction {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
}

impl RenameAction {
    pub fn new(source: Utf8PathBuf, target: Utf8PathBuf) -> Self {
        Self { source, target }
    }

    pub fn filter_unchanged_destinations(
        rename_actions: Vec<RenameAction>,
    ) -> Vec<RenameAction> {
        rename_actions
            .into_iter()
            .filter(RenameAction::source_differs_from_target)
            .collect()
    }

    pub fn source(&self) -> &Utf8Path {
        &self.source
    }

    pub fn target(&self) -> &Utf8Path {
        &self.target
    }

    pub fn source_differs_from_target(&self) -> bool {
        self.source() != self.target()
    }

    pub fn create_actions(rename_actions: Vec<RenameAction>) -> Vec<Action> {
        let target_paths =
            rename_actions.iter().map(RenameAction::target).collect::<Vec<_>>();

        let mut actions =
            Self::list_all_intermediate_paths_of_files(&target_paths)
                .into_iter()
                .filter(|p| !p.is_dir())
                .map(Action::MakeDir)
                .collect::<Vec<_>>();

        actions.extend(rename_actions.into_iter().map(Action::MoveFile));

        actions
    }

    fn list_all_intermediate_paths_of_files(
        paths: &[&Utf8Path],
    ) -> Vec<Utf8PathBuf> {
        let mut directories = paths
            .iter()
            .flat_map(|p| {
                p.parent()
                    .expect("Move::source() should always refer to a file.")
                    .ancestors()
                    .filter(|p| !p.as_str().is_empty())
                    .collect::<Vec<_>>()
            })
            .map(std::borrow::ToOwned::to_owned)
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        directories.sort();

        directories
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Action {
    MoveFile(RenameAction),
    CopyFile(RenameAction),
    RemoveFile(Utf8PathBuf),
    MakeDir(Utf8PathBuf),
    RemoveDir(Utf8PathBuf),
}

impl Action {
    pub fn is_rename_action(&self) -> bool {
        matches!(self, Self::MoveFile { .. } | Self::CopyFile(..))
    }

    pub fn source(&self) -> Option<&Utf8Path> {
        match self {
            Action::MoveFile(rename_action) => Some(rename_action.source()),
            Action::CopyFile(rename_action) => Some(rename_action.source()),
            Action::RemoveFile(_) => None,
            Action::MakeDir(_) => None,
            Action::RemoveDir(_) => None,
        }
    }

    pub fn target(&self) -> &Utf8Path {
        match self {
            Action::MoveFile(rename_action) => rename_action.target(),
            Action::CopyFile(rename_action) => rename_action.target(),
            Action::RemoveFile(path) => path,
            Action::MakeDir(path) => path,
            Action::RemoveDir(path) => path,
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
            .map(Utf8PathBuf::from)
            .collect::<Vec<_>>();

        let paths: Vec<Utf8PathBuf> =
            paths.iter().map(Utf8PathBuf::from).collect();

        let paths_ref =
            paths.iter().map(Utf8PathBuf::as_path).collect::<Vec<_>>();

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
            .map(Utf8PathBuf::from)
            .collect::<Vec<_>>();

        let paths: Vec<Utf8PathBuf> =
            paths.iter().map(Utf8PathBuf::from).collect();

        let paths_ref =
            paths.iter().map(Utf8PathBuf::as_path).collect::<Vec<_>>();

        let directories =
            RenameAction::list_all_intermediate_paths_of_files(&paths_ref);

        assert_eq!(directories, reference);
    }
}
