use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

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

    #[must_use]
    pub fn intermediate_directories(
        rename_actions: &[RenameAction],
    ) -> Vec<Utf8Directory> {
        rename_actions
            .iter()
            .flat_map(|action| {
                action
                    .target()
                    .as_path()
                    .parent()
                    .expect("Utf8File should have parent")
                    .ancestors()
            })
            .filter(|path| !path.as_str().is_empty())
            .map(Utf8Directory::new)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::Utf8Directory;

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
            .collect::<Vec<_>>();

        let paths: Vec<Utf8File> = paths.iter().map(Utf8File::new).collect();

        let actions = paths
            .into_iter()
            .map(|path| RenameAction::new(path.clone(), path))
            .collect::<Vec<_>>();

        let directories = RenameAction::intermediate_directories(&actions);

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
            .collect::<Vec<_>>();

        let paths = paths.iter().map(Utf8File::new).collect::<Vec<_>>();

        let actions = paths
            .into_iter()
            .map(|path| RenameAction::new(path.clone(), path))
            .collect::<Vec<_>>();

        let directories = RenameAction::intermediate_directories(&actions);

        assert_eq!(directories, reference);
    }
}
