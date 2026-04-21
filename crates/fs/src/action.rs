use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::error::TFMTResult;
use tfmttools_core::util::{MoveMode, Utf8Directory, Utf8PathExt};
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

            if let MoveFileResult::CopiedAndRemoved = result {
                let source = rename_action.source().to_owned();

                vec![
                    Action::copy_from_rename_action(rename_action),
                    Action::RemoveFile(source.into_path_buf()),
                ]
            } else {
                vec![Action::move_from_rename_action(rename_action)]
            }
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

pub struct ActionExecutor<'a> {
    handler: ActionHandler<'a>,
}

impl<'a> ActionExecutor<'a> {
    #[must_use]
    pub fn new(fs_handler: &'a FsHandler) -> Self {
        Self { handler: ActionHandler::new(fs_handler) }
    }

    #[must_use]
    pub fn move_mode(mut self, move_mode: MoveMode) -> Self {
        self.handler = self.handler.move_mode(move_mode);
        self
    }

    pub fn apply_rename_actions(
        &self,
        rename_actions: Vec<RenameAction>,
    ) -> impl Iterator<Item = TFMTResult<Action>> + '_ {
        let make_dir_actions =
            RenameAction::get_make_dir_actions(&rename_actions);

        let make_dir_iter = make_dir_actions.into_iter().map(|action| {
            self.handler.apply(&action)?;

            Ok(action)
        });

        let move_files_iter =
            rename_actions.into_iter().flat_map(|rename_action| {
                match self.handler.rename(&rename_action) {
                    Ok(actions) => actions.into_iter().map(Ok).collect(),
                    Err(err) => vec![Err(err)],
                }
            });

        make_dir_iter.chain(move_files_iter)
    }

    pub fn apply_actions(
        &self,
        actions: impl IntoIterator<Item = Action>,
    ) -> TFMTResult<Vec<Action>> {
        actions
            .into_iter()
            .map(|action| {
                self.handler.apply(&action)?;

                Ok(action)
            })
            .collect()
    }

    pub fn remove_directories(
        &self,
        directories: Vec<Utf8Directory>,
    ) -> TFMTResult<Vec<Action>> {
        self.apply_actions(
            directories
                .into_iter()
                .rev()
                .map(|dir| Action::RemoveDir(dir.into_path_buf())),
        )
    }
}
