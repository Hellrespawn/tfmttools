use camino::{Utf8Path, Utf8PathBuf};

// FIXME Don't just check moved file exists, check source file is gone.

/// Returns missing expected files.
pub fn check_reference_files_exist_and_get_missing<'a, I>(
    root: &Utf8Path,
    reference: I,
) -> Vec<Utf8PathBuf>
where
    I: Iterator<Item = &'a Option<String>>,
{
    reference
        .flatten()
        .map(|filename| root.join(filename))
        .filter(|path| !path.is_file())
        .collect()
}

/// Returns files that still exist
pub fn check_reference_files_dont_exist_and_get_remaining<'a, I>(
    root: &Utf8Path,
    reference: I,
) -> Vec<Utf8PathBuf>
where
    I: Iterator<Item = &'a Option<String>>,
{
    reference
        .flatten()
        .map(|filename| root.join(filename))
        .filter(|path| path.is_file())
        .collect()
}
