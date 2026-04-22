use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::error::TFMTResult;
use tfmttools_core::util::{MoveMode, Utf8Directory, Utf8PathExt};
use tracing::trace;

use crate::fs::{FsHandler, MoveFileResult};

mod rename_cycles;
mod rename_planner;
mod rename_staging;

use rename_planner::RenamePlanner;

enum PlannedAction {
    Action(Action),
    Rename(RenameAction),
}

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
        let planned_actions = Self::plan_rename_actions(rename_actions);

        planned_actions.into_iter().flat_map(|planned_action| {
            match planned_action {
                PlannedAction::Action(action) => {
                    match self.handler.apply(&action) {
                        Ok(()) => vec![Ok(action)],
                        Err(err) => vec![Err(err)],
                    }
                },
                PlannedAction::Rename(rename_action) => {
                    match self.handler.rename(&rename_action) {
                        Ok(actions) => actions.into_iter().map(Ok).collect(),
                        Err(err) => vec![Err(err)],
                    }
                },
            }
        })
    }

    fn plan_rename_actions(
        rename_actions: Vec<RenameAction>,
    ) -> Vec<PlannedAction> {
        RenamePlanner::new(rename_actions).plan()
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

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use camino::Utf8PathBuf;
    use color_eyre::Result;
    use tfmttools_core::util::{FSMode, Utf8File};

    use super::*;

    fn temp_path(temp_dir: &TempDir, name: &str) -> Result<Utf8PathBuf> {
        Ok(Utf8PathBuf::try_from(temp_dir.path().join(name))?)
    }

    fn write_file(path: &Utf8PathBuf, contents: &str) -> Result<()> {
        fs_err::write(path, contents)?;
        Ok(())
    }

    fn read_file(path: &Utf8PathBuf) -> Result<String> {
        Ok(fs_err::read_to_string(path)?)
    }

    fn rename_action(
        source: &Utf8PathBuf,
        target: &Utf8PathBuf,
    ) -> Result<RenameAction> {
        Ok(RenameAction::new(Utf8File::new(source)?, Utf8File::new(target)?))
    }

    fn apply_actions(
        fs_handler: &FsHandler,
        actions: Vec<RenameAction>,
    ) -> Result<Vec<Action>> {
        ActionExecutor::new(fs_handler)
            .apply_rename_actions(actions)
            .collect::<TFMTResult<Vec<_>>>()
            .map_err(Into::into)
    }

    #[test]
    fn stages_two_file_swap() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let a = temp_path(&temp_dir, "A.mp3")?;
        let b = temp_path(&temp_dir, "B.mp3")?;
        write_file(&a, "a")?;
        write_file(&b, "b")?;

        let fs_handler = FsHandler::new(FSMode::Default);
        apply_actions(&fs_handler, vec![
            rename_action(&a, &b)?,
            rename_action(&b, &a)?,
        ])?;

        assert_eq!(read_file(&a)?, "b");
        assert_eq!(read_file(&b)?, "a");

        Ok(())
    }

    #[test]
    fn stages_three_file_cycle() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let a = temp_path(&temp_dir, "A.mp3")?;
        let b = temp_path(&temp_dir, "B.mp3")?;
        let c = temp_path(&temp_dir, "C.mp3")?;
        write_file(&a, "a")?;
        write_file(&b, "b")?;
        write_file(&c, "c")?;

        let fs_handler = FsHandler::new(FSMode::Default);
        apply_actions(&fs_handler, vec![
            rename_action(&a, &b)?,
            rename_action(&b, &c)?,
            rename_action(&c, &a)?,
        ])?;

        assert_eq!(read_file(&a)?, "c");
        assert_eq!(read_file(&b)?, "a");
        assert_eq!(read_file(&c)?, "b");

        Ok(())
    }

    #[test]
    fn stages_chain_into_freed_target() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let a = temp_path(&temp_dir, "A.mp3")?;
        let b = temp_path(&temp_dir, "B.mp3")?;
        let c = temp_path(&temp_dir, "C.mp3")?;
        let d = temp_path(&temp_dir, "D.mp3")?;
        write_file(&a, "a")?;
        write_file(&b, "b")?;
        write_file(&c, "c")?;

        let fs_handler = FsHandler::new(FSMode::Default);
        apply_actions(&fs_handler, vec![
            rename_action(&a, &b)?,
            rename_action(&b, &c)?,
            rename_action(&c, &d)?,
        ])?;

        assert!(!a.exists());
        assert_eq!(read_file(&b)?, "a");
        assert_eq!(read_file(&c)?, "b");
        assert_eq!(read_file(&d)?, "c");

        Ok(())
    }

    #[test]
    fn stages_case_only_rename() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let lower = temp_path(&temp_dir, "track.mp3")?;
        let title = temp_path(&temp_dir, "Track.mp3")?;
        write_file(&lower, "track")?;

        let fs_handler = FsHandler::new(FSMode::Default);
        apply_actions(&fs_handler, vec![rename_action(&lower, &title)?])?;

        assert!(!lower.exists());
        assert_eq!(read_file(&title)?, "track");

        Ok(())
    }

    #[test]
    fn undo_and_redo_staged_swap() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let a = temp_path(&temp_dir, "A.mp3")?;
        let b = temp_path(&temp_dir, "B.mp3")?;
        write_file(&a, "a")?;
        write_file(&b, "b")?;

        let fs_handler = FsHandler::new(FSMode::Default);
        let applied_actions = apply_actions(&fs_handler, vec![
            rename_action(&a, &b)?,
            rename_action(&b, &a)?,
        ])?;

        let action_handler = ActionHandler::new(&fs_handler);
        for action in applied_actions.iter().rev() {
            action_handler.undo(action)?;
        }

        assert_eq!(read_file(&a)?, "a");
        assert_eq!(read_file(&b)?, "b");

        for action in &applied_actions {
            action_handler.redo(action)?;
        }

        assert_eq!(read_file(&a)?, "b");
        assert_eq!(read_file(&b)?, "a");

        Ok(())
    }
}
