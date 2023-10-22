use crate::{HistoryError, Result};
use log::trace;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fmt, fs};

/// Action is responsible for doing and undoing filesystem operations
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Hash)]
pub struct Action {
    action_type: ActionType,
    applied: bool,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let indent = "  ";
        match &self.action_type {
            ActionType::Mv { source, target } => write!(
                f,
                "Action::Move {{\n{indent}source: \"{}\",\n{indent}target: \"{}\"\n}}",
                source.display(),
                target.display(), indent=indent
            )?,
            ActionType::MkDir(path) => {
                write!(
                    f, "Action::MakeDir(\n{}\"{}\"\n)", indent, path.display()
                )?;
            }
            ActionType::RmDir(path) => {
                write!(
                    f, "Action::RemoveDir(\n{}\"{}\"\n)",indent,  path.display()
                )?;
            }
        }

        write!(
            f,
            " - {}",
            if self.applied { "Applied" } else { "Not applied" }
        )?;

        Ok(())
    }
}

impl Action {
    /// Create new `Move` Action
    pub fn mv<P, Q>(source: P, target: Q) -> Self
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let action_type = ActionType::Mv {
            source: source.as_ref().to_owned(),
            target: target.as_ref().to_owned(),
        };

        Self {
            action_type,
            applied: false,
        }
    }

    /// Create new `MakeDir` Action
    pub fn mkdir<P>(target: P) -> Self
    where
        P: AsRef<Path>,
    {
        let action_type = ActionType::MkDir(target.as_ref().to_owned());

        Self {
            action_type,
            applied: false,
        }
    }

    /// Create new `RemoveDir` Action
    pub fn rmdir<P>(target: P) -> Self
    where
        P: AsRef<Path>,
    {
        let action_type = ActionType::RmDir(target.as_ref().to_owned());

        Self {
            action_type,
            applied: false,
        }
    }

    pub(crate) fn action_type(&self) -> &ActionType {
        &self.action_type
    }

    pub(crate) fn apply(&mut self) -> Result<()> {
        if self.applied {
            Err(HistoryError::AppliedTwice(self.clone()))
        } else {
            self.action_type.apply()?;
            self.applied = true;
            Ok(())
        }
    }

    pub(crate) fn undo(&mut self) -> Result<()> {
        if self.applied {
            self.action_type.undo()?;
            self.applied = false;
            Ok(())
        } else {
            Err(HistoryError::NotYetApplied(self.clone()))
        }
    }

    /// Gets source and target from this action.
    ///
    /// # Panics
    ///
    /// This function panics if this action's type is not `Action::Move`
    pub fn get_src_tgt_unchecked(&self) -> (&Path, &Path) {
        if let ActionType::Mv { source, target } = self.action_type() {
            (source, target)
        } else {
            panic!("Current Action is not Action::Move!")
        }
    }
}

/// Type is a reserved word.
#[allow(clippy::module_name_repetitions)]
/// Action is responsible for doing and undoing filesystem operations
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Hash)]
pub enum ActionType {
    /// Represents the moving of a file.
    Mv {
        /// Source path
        source: PathBuf,
        /// Target path
        target: PathBuf,
    },
    /// Represents the creating of a directory
    MkDir(PathBuf),
    /// Represents the deletion of a directory
    RmDir(PathBuf),
}
impl ActionType {
    /// Applies the action
    pub(crate) fn apply(&self) -> Result<()> {
        match self {
            ActionType::Mv { source, target } => {
                ActionType::copy_or_move_file(source, target)?;

                trace!(
                    "Renamed:\n\"{}\"\n\"{}\"",
                    &source.display(),
                    &target.display()
                );
            }

            ActionType::MkDir(path) => {
                fs::create_dir(path)?;
                trace!("Created directory {}", path.display());
            }

            ActionType::RmDir(path) => {
                fs::remove_dir(path)?;
                trace!("Removed directory {}", path.display());
            }
        }
        Ok(())
    }

    /// Undoes the action.
    pub(crate) fn undo(&self) -> Result<()> {
        match self {
            ActionType::Mv { source, target } => {
                ActionType::copy_or_move_file(target, source)?;

                trace!(
                    "Undid:\n\"{}\"\n\"{}\"",
                    &target.display(),
                    &source.display(),
                );
            }

            ActionType::MkDir(path) => {
                fs::remove_dir(path)?;

                trace!("Undid directory {}", path.display());
            }

            ActionType::RmDir(path) => {
                fs::create_dir(path)?;

                trace!("Recreated directory {}", path.display());
            }
        }
        Ok(())
    }

    fn copy_or_move_file(source: &Path, target: &Path) -> Result<()> {
        if let Err(err) = fs::rename(source, target) {
            // Can't rename across filesystem boundaries. Checks for
            // the appropriate error and copies/deletes instead.
            // Error codes are correct on Windows 10 20H2 and Arch
            // Linux.
            // UPSTREAM Use ErrorKind::CrossesDevices when it enters stable

            if let Some(error_code) = err.raw_os_error() {
                #[cfg(windows)]
                let expected_error_code = 17;

                #[cfg(unix)]
                let expected_error_code = 18;

                if expected_error_code == error_code {
                    fs::copy(source, target)?;
                    fs::remove_file(source)?;
                    return Ok(());
                };
            }

            Err(err.into())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    // TODO Write test for undoing file that's been moved
    use super::*;
    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use color_eyre::Result;
    use predicates::prelude::*;

    #[test]
    fn test_make_dir() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.child("test");

        let mut action = Action::mkdir(&path);

        // Before: doesn't exist
        path.assert(predicate::path::missing());

        action.apply()?;

        // Applied: exists
        path.assert(predicate::path::exists());

        action.undo()?;

        // Undone: doesn't exist
        path.assert(predicate::path::missing());

        Ok(())
    }

    #[test]
    fn test_remove_dir() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.child("test");
        ActionType::MkDir(path.to_path_buf()).apply()?;

        // Before: exists
        path.assert(predicate::path::exists());

        let mut rmdir_action = Action::rmdir(&path);

        rmdir_action.apply()?;

        // Applied: doesn't exist
        path.assert(predicate::path::missing());

        rmdir_action.undo()?;

        // Undone: exists
        path.assert(predicate::path::exists());

        Ok(())
    }

    #[test]
    fn test_move() -> Result<()> {
        let dir = TempDir::new()?;
        let source = dir.child("source");
        let target = dir.child("target");

        source.touch()?;

        // Before: source exists, target doesn't
        source.assert(predicate::path::exists());
        target.assert(predicate::path::missing());

        let mut mv = Action::mv(&source, &target);

        mv.apply()?;

        // Applied: source doesn't, target exists
        source.assert(predicate::path::missing());
        target.assert(predicate::path::exists());

        mv.undo()?;

        // Undone: source exists, target doesn't
        source.assert(predicate::path::exists());
        target.assert(predicate::path::missing());

        Ok(())
    }

    #[test]
    fn test_apply_twice() -> Result<()> {
        let dir = TempDir::new()?;
        let source = dir.child("source");
        let target = dir.child("target");

        source.touch()?;

        let mut mv = Action::mv(&source, target);

        mv.apply()?;

        assert!(matches!(mv.apply(), Err(HistoryError::AppliedTwice(_))));

        mv.undo()?;

        assert!(matches!(mv.undo(), Err(HistoryError::NotYetApplied(_))));

        Ok(())
    }
}
