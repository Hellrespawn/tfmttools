use camino::{Utf8Path, Utf8PathBuf};
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

pub(crate) fn gather_subdirectories(
    path: &Utf8Path,
    depth: usize,
) -> Vec<Utf8PathBuf> {
    PathIterator::new(path, Some(depth))
        .flatten()
        .filter(|p| p.is_dir())
        .collect()
}

pub(crate) fn copy_or_move_file(
    source: &Utf8Path,
    target: &Utf8Path,
) -> Result<()> {
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

pub(crate) fn create_dir(path: &Utf8Path) -> Result<()> {
    fs::create_dir(path)?;

    Ok(())
}

pub(crate) fn remove_dir(path: &Utf8Path) -> Result<bool> {
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
                return Ok(false);
            }

            return Err(io_error.into());
        }
    }

    Ok(true)
}

pub(crate) fn remove_empty_subdirectories(
    dry_run: bool,
    path: &Utf8Path,
    recursion_depth: usize,
) -> Result<Vec<(Utf8PathBuf, bool)>> {
    let dirs = gather_subdirectories(path, recursion_depth)
        .into_iter()
        .map(|p| {
            let removed = if dry_run { remove_dir(&p)? } else { false };

            Ok((p, removed))
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(dirs)
}
