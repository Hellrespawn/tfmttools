use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;
use ignore::{Walk, WalkBuilder};

pub struct PathIterator(Walk);

impl Iterator for PathIterator {
    type Item = Result<Utf8PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.0.next()?;

        Some(Self::transform_iterator_result(result))
    }
}

impl PathIterator {
    pub fn new(path: &Utf8Path, recursion_depth: Option<usize>) -> Self {
        Self(
            WalkBuilder::new(path)
                .max_depth(Some(recursion_depth.unwrap_or(1)))
                .build(),
        )
    }

    fn transform_iterator_result(
        result: Result<ignore::DirEntry, ignore::Error>,
    ) -> Result<Utf8PathBuf> {
        Ok(result?.into_path().try_into()?)
    }
}

pub fn gather_subdirectories(
    path: &Utf8Path,
    depth: usize,
) -> Vec<Utf8PathBuf> {
    PathIterator::new(path, Some(depth))
        .flatten()
        .filter(|p| p.is_dir())
        .collect()
}

pub enum MoveFileResult {
    Moved,
    CopiedAndRemoved,
    DryRun,
}

pub fn move_file(
    dry_run: bool,
    source: &Utf8Path,
    target: &Utf8Path,
) -> Result<MoveFileResult> {
    if dry_run {
        Ok(MoveFileResult::DryRun)
    } else {
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
                    return Ok(MoveFileResult::CopiedAndRemoved);
                }

                return Err(err.into());
            }
        }

        Ok(MoveFileResult::Moved)
    }
}

pub enum CreateDirResult {
    Created,
    Exists,
    DryRun,
}

pub fn create_dir(dry_run: bool, path: &Utf8Path) -> Result<CreateDirResult> {
    if dry_run {
        Ok(CreateDirResult::DryRun)
    } else if path.is_dir() {
        Ok(CreateDirResult::Exists)
    } else if path.exists() {
        Err(eyre!("Path exists but is not a directory: {}", path))
    } else {
        fs::create_dir(path)?;

        Ok(CreateDirResult::Created)
    }
}

pub enum RemoveDirResult {
    Removed,
    NotEmpty,
    DryRun,
}

pub fn remove_dir(dry_run: bool, path: &Utf8Path) -> Result<RemoveDirResult> {
    if dry_run {
        Ok(RemoveDirResult::DryRun)
    } else {
        let result = fs::remove_dir(path);

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

pub enum RemoveDirAllResult {
    Removed,
    DryRun,
}

pub fn remove_dir_all(
    dry_run: bool,
    path: &Utf8Path,
) -> Result<RemoveDirAllResult> {
    if dry_run {
        Ok(RemoveDirAllResult::DryRun)
    } else {
        fs::remove_dir_all(path)?;
        Ok(RemoveDirAllResult::Removed)
    }
}

pub fn remove_empty_subdirectories(
    dry_run: bool,
    path: &Utf8Path,
    recursion_depth: usize,
) -> Result<Vec<(Utf8PathBuf, RemoveDirResult)>> {
    let dirs = gather_subdirectories(path, recursion_depth)
        .into_iter()
        .map(|p| {
            let removed = remove_dir(dry_run, &p)?;

            Ok((p, removed))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(dirs)
}

pub fn get_longest_common_prefix(paths: &[&Utf8Path]) -> Option<Utf8PathBuf> {
    if paths.is_empty() {
        None
    } else {
        let mut iter = paths.iter();

        // We have already returned if no files were found, so this unwrap
        // should be safe.
        let mut common_path = iter.next().unwrap().to_path_buf();

        for path in iter {
            let mut new_common_path = Utf8PathBuf::new();

            for (left, right) in path.components().zip(common_path.components())
            {
                if left == right {
                    new_common_path.push(left);
                } else {
                    break;
                }
            }
            common_path = new_common_path;
        }

        Some(common_path)
    }
}
