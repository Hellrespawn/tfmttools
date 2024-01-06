use std::collections::HashSet;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use serde::{Deserialize, Serialize};

mod validation;

pub use validation::validate_move_actions;

use crate::fs::FsHandler;

#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub struct Move {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
}

impl Move {
    pub fn new(source: Utf8PathBuf, target: Utf8PathBuf) -> Self {
        Self { source, target }
    }

    pub fn filter_unchanged_destinations(move_actions: Vec<Move>) -> Vec<Move> {
        move_actions
            .into_iter()
            .filter(Move::source_differs_from_target)
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

    pub fn create_actions(move_actions: Vec<Move>) -> Vec<Action> {
        let target_paths =
            move_actions.iter().map(Move::target).collect::<Vec<_>>();

        let mut actions =
            Self::list_all_intermediate_paths_of_files(&target_paths)
                .into_iter()
                .filter(|p| !p.is_dir())
                .map(Action::MakeDir)
                .collect::<Vec<_>>();

        actions.extend(
            move_actions
                .into_iter()
                .map(|m| Action::Move { source: m.source, target: m.target }),
        );

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
    Move { source: Utf8PathBuf, target: Utf8PathBuf },
    MakeDir(Utf8PathBuf),
    RemoveDir(Utf8PathBuf),
}

impl Action {
    pub fn is_move(&self) -> bool {
        matches!(self, Self::Move { .. })
    }

    pub fn is_mk_dir(&self) -> bool {
        matches!(self, Self::MakeDir(_))
    }

    pub fn is_rm_dir(&self) -> bool {
        matches!(self, Self::RemoveDir(_))
    }

    pub fn apply(&self, fs_handler: &FsHandler) -> Result<()> {
        match self {
            Action::Move { source, target } => {
                fs_handler.move_file(source, target)?;
            },
            Action::MakeDir(path) => {
                fs_handler.create_dir(path)?;
            },
            Action::RemoveDir(path) => {
                fs_handler.remove_dir(path)?;
            },
        }

        Ok(())
    }

    pub fn undo(&self, fs_handler: &FsHandler) -> Result<()> {
        match self {
            Action::Move { source, target } => {
                fs_handler.move_file(target, source)?;
            },
            Action::MakeDir(path) => {
                fs_handler.remove_dir(path)?;
            },
            Action::RemoveDir(path) => {
                fs_handler.create_dir(path)?;
            },
        }

        Ok(())
    }

    pub fn redo(&self, fs_handler: &FsHandler) -> Result<()> {
        self.apply(fs_handler)
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
            Move::list_all_intermediate_paths_of_files(&paths_ref);

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
            Move::list_all_intermediate_paths_of_files(&paths_ref);

        assert_eq!(directories, reference);
    }
}
