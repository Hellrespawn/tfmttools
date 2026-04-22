use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::error::TFMTResult;
use tfmttools_core::util::{MoveMode, Utf8PathExt};
use tracing::trace;

use crate::fs_handler::{FsHandler, MoveFileResult};

pub struct ActionHandler<'a> {
    fs_handler: &'a FsHandler,
    move_mode: MoveMode,
}

impl<'a> ActionHandler<'a> {
    #[must_use]
    pub fn new(fs_handler: &'a FsHandler) -> Self {
        Self { fs_handler, move_mode: MoveMode::Auto }
    }

    #[must_use]
    pub fn move_mode(mut self, move_mode: MoveMode) -> Self {
        self.move_mode = move_mode;
        self
    }

    fn always_copy(&self) -> bool {
        matches!(self.move_mode, MoveMode::AlwaysCopy)
    }

    pub fn rename(
        &self,
        rename_action: &RenameAction,
    ) -> TFMTResult<Vec<Action>> {
        let applied_actions = if self.always_copy() {
            self.fs_handler.copy_file(
                rename_action.source().as_path(),
                rename_action.target().as_path(),
            )?;

            self.fs_handler.remove_file(rename_action.source().as_path())?;

            let source = rename_action.source().to_owned();

            vec![
                Action::copy_from_rename_action(rename_action),
                Action::RemoveFile(source.into_path_buf()),
            ]
        } else {
            let result = self.fs_handler.move_file(
                rename_action.source().as_path(),
                rename_action.target().as_path(),
            )?;

            actions_from_move_result(rename_action, result)
        };

        Ok(applied_actions)
    }

    pub fn apply(&self, action: &Action) -> TFMTResult {
        self.apply_forward(action)
    }

    pub fn undo(&self, action: &Action) -> TFMTResult {
        match action {
            Action::MoveFile { source, target } => {
                self.fs_handler
                    .move_file(target.as_path(), source.as_path())?;
            },

            Action::CopyFile { source, target } => {
                self.fs_handler
                    .copy_file(target.as_path(), source.as_path())?;

                self.fs_handler.remove_file(target.as_path())?;
            },
            Action::RemoveFile(_path) => {
                trace!(
                    "Ignoring undo of Action::RemoveFile, handled by associated Action::CopyFile"
                );
            },
            Action::MakeDir(path) => {
                self.fs_handler.remove_dir(path)?;
            },
            Action::RemoveDir(path) => {
                self.fs_handler.create_dir(path)?;
            },
        }

        Ok(())
    }

    pub fn redo(&self, action: &Action) -> TFMTResult<()> {
        self.apply_forward(action)
    }

    fn apply_forward(&self, action: &Action) -> TFMTResult {
        match action {
            Action::MoveFile { source, target } => {
                self.fs_handler
                    .move_file(source.as_path(), target.as_path())?;
            },

            Action::CopyFile { source, target } => {
                self.fs_handler
                    .copy_file(source.as_path(), target.as_path())?;
            },
            Action::RemoveFile(path) => {
                self.fs_handler.remove_file(path.as_path())?;
            },
            Action::MakeDir(path) => {
                self.fs_handler.create_dir(path)?;
            },
            Action::RemoveDir(path) => {
                self.fs_handler.remove_dir(path)?;
            },
        }

        Ok(())
    }
}

fn actions_from_move_result(
    rename_action: &RenameAction,
    result: MoveFileResult,
) -> Vec<Action> {
    if let MoveFileResult::CopiedAndRemoved = result {
        let source = rename_action.source().to_owned();

        vec![
            Action::copy_from_rename_action(rename_action),
            Action::RemoveFile(source.into_path_buf()),
        ]
    } else {
        vec![Action::move_from_rename_action(rename_action)]
    }
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use color_eyre::Result;
    use tfmttools_core::action::{Action, RenameAction};
    use tfmttools_core::util::Utf8File;

    use super::*;

    fn rename_action(
        source: &str,
        target: &str,
    ) -> Result<(RenameAction, Utf8PathBuf, Utf8PathBuf)> {
        let source = Utf8PathBuf::from(source);
        let target = Utf8PathBuf::from(target);
        let action =
            RenameAction::new(Utf8File::new(&source)?, Utf8File::new(&target)?);

        Ok((action, source, target))
    }

    #[test]
    fn copied_and_removed_move_records_copy_and_remove_actions() -> Result<()> {
        let (rename_action, source, target) =
            rename_action("source.mp3", "target.mp3")?;

        let actions = actions_from_move_result(
            &rename_action,
            MoveFileResult::CopiedAndRemoved,
        );

        assert_eq!(actions.len(), 2);
        assert!(matches!(
            &actions[0],
            Action::CopyFile {
                source: copy_source,
                target: copy_target,
            } if copy_source == &source && copy_target == &target
        ));
        assert!(matches!(
            &actions[1],
            Action::RemoveFile(remove_source) if remove_source == &source
        ));

        Ok(())
    }

    #[test]
    fn moved_result_records_move_action() -> Result<()> {
        let (rename_action, source, target) =
            rename_action("source.mp3", "target.mp3")?;

        let actions =
            actions_from_move_result(&rename_action, MoveFileResult::Moved);

        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            Action::MoveFile {
                source: move_source,
                target: move_target,
            } if move_source == &source && move_target == &target
        ));

        Ok(())
    }
}
