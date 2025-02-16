use tfmttools_core::action::Action;
use tfmttools_core::error::TFMTResult;

use crate::fs::FsHandler;

pub struct ActionHandler<'a> {
    fs_handler: &'a FsHandler,
}

impl<'a> ActionHandler<'a> {
    pub fn new(fs_handler: &'a FsHandler) -> Self {
        Self { fs_handler }
    }

    pub fn apply(&self, action: Action) -> TFMTResult<Vec<Action>> {
        match &action {
            Action::MoveFile(move_file_action) => {
                self.fs_handler.move_file(
                    move_file_action.source(),
                    move_file_action.target(),
                )?;
            },
            Action::MakeDir(path) => {
                self.fs_handler.create_dir(path)?;
            },
            Action::RemoveDir(path) => {
                self.fs_handler.remove_dir(path)?;
            },
        }

        Ok(vec![action])
    }

    pub fn undo(&self, action: &Action) -> TFMTResult<()> {
        match action {
            Action::MoveFile(move_file_action) => {
                self.fs_handler.move_file(
                    move_file_action.target(),
                    move_file_action.source(),
                )?;
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
            Action::MoveFile(move_file_action) => {
                self.fs_handler.move_file(
                    move_file_action.source(),
                    move_file_action.target(),
                )?;
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
