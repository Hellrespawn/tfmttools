use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use tfmttools_core::error::{TFMTError, TFMTResult};
use tfmttools_core::util::ActionMode;
use tracing::trace;

use crate::PathIterator;
use crate::path_iterator::PathIteratorOptions;

pub enum MoveFileResult {
    Moved,
    CopiedAndRemoved,
    DryRun,
}

pub enum CopyFileResult {
    Copied,
    DryRun,
}

pub enum RemoveFileResult {
    Removed,
    DryRun,
}

pub enum CreateDirResult {
    Created,
    Exists,
    DryRun,
}

#[derive(Debug)]
pub enum RemoveDirResult {
    Removed,
    NotEmpty,
    DryRun,
}

pub fn gather_subdirectories(
    path: &Utf8Path,
    depth: usize,
) -> Vec<Utf8PathBuf> {
    let options = PathIteratorOptions::with_depth(path, depth);

    PathIterator::new(&options).flatten().filter(|p| p.is_dir()).collect()
}

#[derive(Debug)]
pub struct FsHandler {
    action_mode: ActionMode,
}

impl FsHandler {
    #[must_use]
    pub fn new(action_mode: ActionMode) -> Self {
        Self { action_mode }
    }

    pub fn write<P, C>(&self, path: P, contents: C) -> std::io::Result<()>
    where
        P: AsRef<Path>,
        C: AsRef<[u8]>,
    {
        if matches!(self.action_mode, ActionMode::DryRun) {
            Ok(())
        } else {
            fs_err::write(path, contents)
        }
    }

    pub fn move_file(
        &self,
        source: &Utf8Path,
        target: &Utf8Path,
    ) -> TFMTResult<MoveFileResult> {
        if matches!(self.action_mode, ActionMode::DryRun) {
            Ok(MoveFileResult::DryRun)
        } else {
            // std::fs::rename does not work across filesystem boundaries.
            // Check for the appropriate error and copies + deletes instead.
            // Error codes are correct on Windows 10 20H2 and Arch
            // Linux.
            // UPSTREAM Use ErrorKind::CrossesDevices when it enters stable

            #[cfg(windows)]
            const EXPECTED_ERROR_CODE: i32 = 17;

            #[cfg(unix)]
            const EXPECTED_ERROR_CODE: i32 = 18;

            if let Err(err) = std::fs::rename(source, target) {
                // HACK Unable to capture raw_os_error with fs_err, use
                // std::fs::rename instead.
                let is_expected_error =
                    err.raw_os_error().is_some_and(|code| {
                        code > 0 && code == EXPECTED_ERROR_CODE
                    });

                if is_expected_error {
                    fs_err::copy(source, target)?;
                    fs_err::remove_file(source)?;
                    Ok(MoveFileResult::CopiedAndRemoved)
                } else {
                    Err(TFMTError::UnexpectedMoveError(
                        source.to_owned(),
                        target.to_owned(),
                        err.to_string(),
                    ))
                }
            } else {
                Ok(MoveFileResult::Moved)
            }
        }
    }

    pub fn copy_file(
        &self,
        source: &Utf8Path,
        target: &Utf8Path,
    ) -> TFMTResult<CopyFileResult> {
        if matches!(self.action_mode, ActionMode::DryRun) {
            Ok(CopyFileResult::DryRun)
        } else {
            fs_err::copy(source, target)?;

            Ok(CopyFileResult::Copied)
        }
    }

    pub fn remove_file(&self, path: &Utf8Path) -> TFMTResult<RemoveFileResult> {
        if matches!(self.action_mode, ActionMode::DryRun) {
            Ok(RemoveFileResult::DryRun)
        } else {
            fs_err::remove_file(path)?;

            Ok(RemoveFileResult::Removed)
        }
    }

    pub fn create_dir(&self, path: &Utf8Path) -> TFMTResult<CreateDirResult> {
        if matches!(self.action_mode, ActionMode::DryRun) {
            Ok(CreateDirResult::DryRun)
        } else if path.is_dir() {
            Ok(CreateDirResult::Exists)
        } else if path.exists() {
            Err(TFMTError::NotADirectory(path.to_owned()))
        } else {
            fs_err::create_dir(path)?;

            Ok(CreateDirResult::Created)
        }
    }

    pub fn remove_dir(&self, path: &Utf8Path) -> TFMTResult<RemoveDirResult> {
        if matches!(self.action_mode, ActionMode::DryRun) {
            Ok(RemoveDirResult::DryRun)
        } else {
            let result = fs_err::remove_dir(path);

            if let Err(io_error) = result {
                if let Some(error_code) = io_error.raw_os_error() {
                    #[cfg(windows)]
                    // https://docs.microsoft.com/en-us/windows/win32/debug/system-error-codes--0-499-
                    // 145: Directory not empty
                    let expected_code = 145;

                    // https://nuetzlich.net/errno.html
                    // 39: Directory not empty
                    #[cfg(unix)]
                    let expected_code = 39;

                    if error_code == expected_code {
                        return Ok(RemoveDirResult::NotEmpty);
                    }

                    return Err(io_error.into());
                }
            }

            Ok(RemoveDirResult::Removed)
        }
    }

    pub fn remove_dir_all(
        &self,
        path: &Utf8Path,
    ) -> TFMTResult<RemoveDirResult> {
        if matches!(self.action_mode, ActionMode::DryRun) {
            Ok(RemoveDirResult::DryRun)
        } else {
            fs_err::remove_dir_all(path)?;
            Ok(RemoveDirResult::Removed)
        }
    }

    pub fn remove_empty_subdirectories(
        &self,
        path: &Utf8Path,
        recursion_depth: usize,
    ) -> TFMTResult<Vec<(Utf8PathBuf, RemoveDirResult)>> {
        let dirs = gather_subdirectories(path, recursion_depth)
            .into_iter()
            .rev()
            .map(|p| {
                let removed = self.remove_dir(&p)?;

                trace!("Removing dir: {p} => {removed:?}");

                Ok((p, removed))
            })
            .collect::<TFMTResult<Vec<_>>>()?;

        Ok(dirs)
    }
}

#[must_use]
pub fn get_longest_common_prefix(paths: &[&Utf8Path]) -> Option<Utf8PathBuf> {
    if paths.is_empty() {
        None
    } else if paths.len() == 1 {
        Some(
            paths[0]
                .parent()
                .expect("File should always have parent.")
                .to_owned(),
        )
    } else {
        let mut iter = paths.iter();

        // We have already returned if no files were found, so this unwrap
        // should be safe.
        let mut common_prefix = iter.next().unwrap().to_path_buf();

        for path in iter {
            let mut new_common_prefix = Utf8PathBuf::new();

            for (left, right) in
                path.components().zip(common_prefix.components())
            {
                if left == right {
                    new_common_prefix.push(left);
                } else {
                    break;
                }
            }
            common_prefix = new_common_prefix;
        }

        Some(common_prefix)
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use color_eyre::Result;

    #[test]
    fn test_remove_dir_error_codes() -> Result<()> {
        let tempdir = TempDir::new()?;

        let test_folder = tempdir.path().join("test_folder");
        let test_file = test_folder.join("test.file");

        #[cfg(windows)]
        let expected_code = 145;

        #[cfg(unix)]
        let expected_code = 39;

        fs_err::create_dir(&test_folder)?;
        fs_err::write(test_file, "")?;

        if let Err(err) = fs_err::remove_dir(test_folder) {
            if let Some(error_code) =
                std::io::Error::last_os_error().raw_os_error()
            {
                assert_eq!(
                    error_code, expected_code,
                    "Expected code {expected_code}, got {error_code}",
                );
                Ok(())
            } else {
                panic!("Received unexpected error:\n{err}");
            }
        } else {
            Ok(())
        }
    }
}
