use tfmttools_core::action::Action;
use tfmttools_core::error::TFMTResult;
use tfmttools_core::util::{MoveMode, Utf8PathExt};
use tracing::trace;

use crate::fs::{FsHandler, MoveFileResult};

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

    pub fn apply(&self, action: Action) -> TFMTResult<Vec<Action>> {
        let actions = match action {
            Action::MoveFile(rename_action) => {
                if self.always_copy() {
                    self.fs_handler.copy_file(
                        rename_action.source().as_path(),
                        rename_action.target().as_path(),
                    )?;

                    self.fs_handler
                        .remove_file(rename_action.source().as_path())?;

                    let source = rename_action.source().to_owned();

                    vec![
                        Action::CopyFile(rename_action),
                        Action::RemoveFile(source.into_path_buf()),
                    ]
                } else {
                    let result = self.fs_handler.move_file(
                        rename_action.source().as_path(),
                        rename_action.target().as_path(),
                    )?;

                    if let MoveFileResult::CopiedAndRemoved = result {
                        let source = rename_action.source().to_owned();

                        vec![
                            Action::CopyFile(rename_action),
                            Action::RemoveFile(source.into_path_buf()),
                        ]
                    } else {
                        vec![Action::MoveFile(rename_action)]
                    }
                }
            },
            Action::CopyFile(rename_action) => {
                self.fs_handler.copy_file(
                    rename_action.source().as_path(),
                    rename_action.target().as_path(),
                )?;

                vec![Action::CopyFile(rename_action)]
            },
            Action::RemoveFile(path) => {
                vec![Action::RemoveFile(path)]
            },
            Action::MakeDir(path) => {
                self.fs_handler.create_dir(&path)?;

                vec![Action::MakeDir(path)]
            },
            Action::RemoveDir(path) => {
                self.fs_handler.remove_dir(&path)?;

                vec![Action::RemoveDir(path)]
            },
        };

        Ok(actions)
    }

    pub fn undo(&self, action: &Action) -> TFMTResult<()> {
        match action {
            Action::MoveFile(rename_action) => {
                self.fs_handler.move_file(
                    rename_action.target().as_path(),
                    rename_action.source().as_path(),
                )?;
            },
            Action::CopyFile(rename_action) => {
                self.fs_handler.copy_file(
                    rename_action.target().as_path(),
                    rename_action.source().as_path(),
                )?;

                self.fs_handler
                    .remove_file(rename_action.target().as_path())?;
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
        match action {
            Action::MoveFile(rename_action) => {
                self.fs_handler.move_file(
                    rename_action.source().as_path(),
                    rename_action.target().as_path(),
                )?;
            },
            Action::CopyFile(rename_action) => {
                self.fs_handler.copy_file(
                    rename_action.source().as_path(),
                    rename_action.target().as_path(),
                )?;
            },
            Action::RemoveFile(path) => {
                self.fs_handler.remove_file(path)?;
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
