use std::collections::HashSet;

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use fs_err as fs;

#[derive(PartialEq)]
pub(crate) struct Move {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
}

impl Move {
    pub(crate) fn new(source: Utf8PathBuf, target: Utf8PathBuf) -> Self {
        Self { source, target }
    }

    pub(crate) fn filter_unchanged_destinations(
        move_actions: Vec<Move>,
    ) -> Vec<Move> {
        move_actions
            .into_iter()
            .filter(Move::source_differs_from_target)
            .collect()
    }

    pub(crate) fn source(&self) -> &Utf8Path {
        &self.source
    }

    pub(crate) fn target(&self) -> &Utf8Path {
        &self.target
    }

    pub(crate) fn source_equals_target(&self) -> bool {
        self.source() == self.target()
    }

    pub(crate) fn source_differs_from_target(&self) -> bool {
        !self.source_equals_target()
    }

    pub(crate) fn apply(self, dry_run: bool) -> Result<Vec<Action>> {
        let mut actions = Self::create_directory_if_not_exists(
            dry_run,
            self.target()
                .parent()
                .expect("Move target should always be a file with a parent."),
        )?;

        if !dry_run {
            actions.extend(self.copy_or_move_file()?);
        }

        Ok(actions)
    }

    pub(crate) fn get_unique_source_directories(
        move_actions: &[Move],
    ) -> Vec<Utf8PathBuf> {
        // let common_path = crate::fs::get_common_path(
        //     &move_actions.iter().map(Move::source).collect::<Vec<_>>(),
        // );

        let mut directories = move_actions
            .iter()
            .flat_map(|m| {
                m.source()
                    .parent()
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
        directories.reverse();

        directories
    }

    fn copy_or_move_file(self) -> Result<Option<Action>> {
        if self.source_equals_target() {
            Ok(None)
        } else {
            crate::fs::copy_or_move_file(self.source(), self.target())?;

            let action = Action::Move(self);

            Ok(Some(action))
        }
    }

    fn create_directory_if_not_exists(
        dry_run: bool,
        path: &Utf8Path,
    ) -> Result<Vec<Action>> {
        if path.is_dir() {
            Ok(Vec::new())
        } else {
            let mut actions = Vec::new();

            if let Some(parent) = path.parent() {
                actions.extend(Self::create_directory_if_not_exists(
                    dry_run, parent,
                )?);
            }

            if !dry_run {
                fs::create_dir(path)?;
            }

            actions.push(Action::MakeDir(path.to_owned()));

            Ok(actions)
        }
    }
}

pub(crate) enum Action {
    Move(Move),
    MakeDir(Utf8PathBuf),
    RemoveDir(Utf8PathBuf),
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

        let reference = ["a/b/h", "a/b/c", "a/b", "a"]
            .into_iter()
            .map(Utf8PathBuf::from)
            .collect::<Vec<_>>();

        let move_actions: Vec<Move> = paths
            .iter()
            .map(|s| Move::new(Utf8PathBuf::from(s), Utf8PathBuf::new()))
            .collect();

        let directories = Move::get_unique_source_directories(&move_actions);

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

        let reference = ["/a/b/h", "/a/b/c", "/a/b", "/a", "/"]
            .into_iter()
            .map(Utf8PathBuf::from)
            .collect::<Vec<_>>();

        let move_actions: Vec<Move> = paths
            .iter()
            .map(|s| Move::new(Utf8PathBuf::from(s), Utf8PathBuf::new()))
            .collect();

        let directories = Move::get_unique_source_directories(&move_actions);

        assert_eq!(directories, reference);
    }
}
