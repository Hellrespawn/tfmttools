use std::path::{Path, PathBuf};
use std::{fmt, fs};

use log::trace;
use serde::{Deserialize, Serialize};

use crate::{HistoryError, Result};

/// Change is responsible for doing and undoing filesystem operations
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Hash)]
pub struct Change {
    change_type: ChangeType,
    applied: bool,
}

impl fmt::Display for Change {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let indent = "  ";
        match &self.change_type {
            ChangeType::Mv { source, target } => write!(
                f,
                "Change::Move {{\n{indent}source: \"{}\",\n{indent}target: \"{}\"\n}}",
                source.display(),
                target.display(), indent=indent
            )?,
            ChangeType::MkDir(path) => {
                write!(
                    f, "Change::MakeDir(\n{}\"{}\"\n)", indent, path.display()
                )?;
            }
            ChangeType::RmDir(path) => {
                write!(
                    f, "Change::RemoveDir(\n{}\"{}\"\n)",indent,  path.display()
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

impl Change {
    pub fn mv<P, Q>(source: P, target: Q) -> Self
    where
        P: AsRef<Path>,
        Q: AsRef<Path>,
    {
        let change_type = ChangeType::Mv {
            source: source.as_ref().to_owned(),
            target: target.as_ref().to_owned(),
        };

        Self { change_type, applied: false }
    }

    pub fn mkdir<P>(target: P) -> Self
    where
        P: AsRef<Path>,
    {
        let change_type = ChangeType::MkDir(target.as_ref().to_owned());

        Self { change_type, applied: false }
    }

    pub fn rmdir<P>(target: P) -> Self
    where
        P: AsRef<Path>,
    {
        let change_type = ChangeType::RmDir(target.as_ref().to_owned());

        Self { change_type, applied: false }
    }

    pub(crate) fn change_type(&self) -> &ChangeType {
        &self.change_type
    }

    pub(crate) fn apply(&mut self) -> Result<()> {
        if self.applied {
            Err(HistoryError::AppliedTwice(self.clone()))
        } else {
            self.change_type.apply()?;
            self.applied = true;
            Ok(())
        }
    }

    pub(crate) fn undo(&mut self) -> Result<()> {
        if self.applied {
            self.change_type.undo()?;
            self.applied = false;
            Ok(())
        } else {
            Err(HistoryError::NotYetApplied(self.clone()))
        }
    }

    pub fn source(&self) -> Option<&Path> {
        if let ChangeType::Mv { source, .. } = &self.change_type {
            Some(source)
        } else {
            None
        }
    }

    pub fn target(&self) -> &Path {
        match self.change_type() {
            ChangeType::Mv { target, .. }
            | ChangeType::MkDir(target)
            | ChangeType::RmDir(target) => target,
        }
    }
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Hash)]
pub enum ChangeType {
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
impl ChangeType {
    /// Applies the change
    pub(crate) fn apply(&self) -> Result<()> {
        match self {
            ChangeType::Mv { source, target } => {
                ChangeType::copy_or_move_file(source, target)?;

                trace!(
                    "Renamed:\n\"{}\"\n\"{}\"",
                    &source.display(),
                    &target.display()
                );
            },

            ChangeType::MkDir(path) => {
                fs::create_dir(path)?;
                trace!("Created directory {}", path.display());
            },

            ChangeType::RmDir(path) => {
                fs::remove_dir(path)?;
                trace!("Removed directory {}", path.display());
            },
        }
        Ok(())
    }

    /// Undoes the change.
    pub(crate) fn undo(&self) -> Result<()> {
        match self {
            ChangeType::Mv { source, target } => {
                ChangeType::copy_or_move_file(target, source)?;

                trace!(
                    "Undid:\n\"{}\"\n\"{}\"",
                    &target.display(),
                    &source.display(),
                );
            },

            ChangeType::MkDir(path) => {
                fs::remove_dir(path)?;

                trace!("Undid directory {}", path.display());
            },

            ChangeType::RmDir(path) => {
                fs::create_dir(path)?;

                trace!("Recreated directory {}", path.display());
            },
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
    use assert_fs::prelude::*;
    use assert_fs::TempDir;
    use color_eyre::Result;
    use predicates::prelude::*;

    use super::*;

    #[test]
    fn test_make_dir() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.child("test");

        let mut change = Change::mkdir(&path);

        // Before: doesn't exist
        path.assert(predicate::path::missing());

        change.apply()?;

        // Applied: exists
        path.assert(predicate::path::exists());

        change.undo()?;

        // Undone: doesn't exist
        path.assert(predicate::path::missing());

        Ok(())
    }

    #[test]
    fn test_remove_dir() -> Result<()> {
        let dir = TempDir::new()?;
        let path = dir.child("test");
        ChangeType::MkDir(path.to_path_buf()).apply()?;

        // Before: exists
        path.assert(predicate::path::exists());

        let mut rmdir_change = Change::rmdir(&path);

        rmdir_change.apply()?;

        // Applied: doesn't exist
        path.assert(predicate::path::missing());

        rmdir_change.undo()?;

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

        let mut mv = Change::mv(&source, &target);

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

        let mut mv = Change::mv(&source, target);

        mv.apply()?;

        assert!(matches!(mv.apply(), Err(HistoryError::AppliedTwice(_))));

        mv.undo()?;

        assert!(matches!(mv.undo(), Err(HistoryError::NotYetApplied(_))));

        Ok(())
    }
}
