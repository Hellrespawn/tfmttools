use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::util::MoveMode;

use super::PlannedAction;
use super::rename_planner::RenamePlanner;
use crate::error::FsResult;

/// Plan ordered actions without changing the filesystem.
pub fn plan_actions(
    rename_actions: Vec<RenameAction>,
    move_mode: MoveMode,
) -> FsResult<Vec<Action>> {
    let mut result = Vec::new();
    for planned in RenamePlanner::new(rename_actions).plan() {
        let action = match planned {
            PlannedAction::Action(action) => action,
            PlannedAction::Rename(rename) => {
                Action::move_from_rename_action(&rename)
            },
        };
        if let Action::MoveFile { source, target } = &action
            && (matches!(move_mode, MoveMode::AlwaysCopy)
                || super::prepared_execution::crosses_devices(source, target)?)
        {
            result.push(Action::CopyFile {
                source: source.clone(),
                target: target.clone(),
            });
            result.push(Action::RemoveFile(source.clone()));
        } else {
            result.push(action);
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use camino::Utf8PathBuf;
    use color_eyre::Result;
    use tfmttools_core::action::{Action, RenameAction};
    use tfmttools_core::history::OperationKind;
    use tfmttools_core::util::{Utf8File, Utf8PathExt};

    use super::*;
    use crate::prepare_action;

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

    fn dir_entry_name(path: &Utf8PathBuf) -> Result<String> {
        let parent = path.parent().expect("path should have a parent");

        Ok(fs_err::read_dir(parent)?
            .next()
            .expect("directory should contain exactly one entry")?
            .file_name()
            .to_string_lossy()
            .into_owned())
    }

    #[allow(clippy::unnecessary_wraps)]
    fn rename_action(
        source: &Utf8PathBuf,
        target: &Utf8PathBuf,
    ) -> Result<RenameAction> {
        Ok(RenameAction::new(Utf8File::new(source), Utf8File::new(target)))
    }

    fn apply_actions(actions: Vec<RenameAction>) -> Result<Vec<Action>> {
        let before = actions
            .iter()
            .map(|a| {
                let path = a.source().as_path();
                Ok((path.to_owned(), fs_err::read(path)?))
            })
            .collect::<Result<Vec<_>>>()?;
        let planned = plan_actions(actions, MoveMode::Auto)?;
        for (path, bytes) in before {
            assert_eq!(fs_err::read(path)?, bytes);
        }
        for action in &planned {
            execute(action, OperationKind::Apply)?;
        }
        Ok(planned)
    }

    fn execute(action: &Action, direction: OperationKind) -> Result<()> {
        let mut prepared = prepare_action(action, direction)?;
        prepared.execute()?;
        prepared.confirm()?;
        Ok(())
    }

    #[test]
    fn stages_two_file_swap() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let a = temp_path(&temp_dir, "A.mp3")?;
        let b = temp_path(&temp_dir, "B.mp3")?;
        write_file(&a, "a")?;
        write_file(&b, "b")?;

        apply_actions(vec![rename_action(&a, &b)?, rename_action(&b, &a)?])?;

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

        apply_actions(vec![
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

        apply_actions(vec![
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
    fn skips_make_dir_for_directory_that_already_exists() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let existing_dir = temp_dir.path().join("existing");
        fs_err::create_dir(&existing_dir)?;

        let source = temp_path(&temp_dir, "A.mp3")?;
        let target = Utf8PathBuf::try_from(existing_dir.join("B.mp3"))?;
        write_file(&source, "a")?;

        // Should not error even though the intermediate directory
        // ("existing") is already present on disk.
        apply_actions(vec![rename_action(&source, &target)?])?;

        assert_eq!(read_file(&target)?, "a");

        Ok(())
    }

    #[test]
    fn stages_case_only_rename() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let lower = temp_path(&temp_dir, "track.mp3")?;
        let title = temp_path(&temp_dir, "Track.mp3")?;
        write_file(&lower, "track")?;

        apply_actions(vec![rename_action(&lower, &title)?])?;

        assert_eq!(dir_entry_name(&title)?, "Track.mp3");
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

        let applied_actions = apply_actions(vec![
            rename_action(&a, &b)?,
            rename_action(&b, &a)?,
        ])?;

        for action in applied_actions.iter().rev() {
            execute(action, OperationKind::Undo)?;
        }

        assert_eq!(read_file(&a)?, "a");
        assert_eq!(read_file(&b)?, "b");

        for action in &applied_actions {
            execute(action, OperationKind::Redo)?;
        }

        assert_eq!(read_file(&a)?, "b");
        assert_eq!(read_file(&b)?, "a");

        Ok(())
    }
}
