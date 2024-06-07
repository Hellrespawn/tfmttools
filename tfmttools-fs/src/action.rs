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

    pub fn apply(&self, action: &Action) -> TFMTResult<()> {
        match action {
            Action::Rename(rename_action) => {
                self.fs_handler.move_file(
                    rename_action.source(),
                    rename_action.target(),
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

    pub fn undo(&self, action: &Action) -> TFMTResult<()> {
        match action {
            Action::Rename(rename_action) => {
                self.fs_handler.move_file(
                    rename_action.target(),
                    rename_action.source(),
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
        self.apply(action)
    }
}
