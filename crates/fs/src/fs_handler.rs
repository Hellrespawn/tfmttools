use camino::{Utf8Path, Utf8PathBuf};
use tfmttools_core::util::FSMode;

#[derive(Debug)]
pub struct FsHandler {
    fs_mode: FSMode,
}

impl FsHandler {
    #[must_use]
    pub fn new(fs_mode: FSMode) -> Self {
        Self { fs_mode }
    }

    #[must_use]
    pub fn is_dry_run(&self) -> bool {
        matches!(self.fs_mode, FSMode::DryRun)
    }
}

#[must_use]
pub fn get_longest_common_prefix<'a>(
    paths: impl IntoIterator<Item = &'a Utf8Path>,
) -> Option<Utf8PathBuf> {
    let mut iter = paths.into_iter();
    let first = iter.next()?;

    if let Some(second) = iter.next() {
        let mut common_prefix = first.to_path_buf();

        for path in std::iter::once(second).chain(iter) {
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
    } else {
        Some(
            first.parent().expect("File should always have parent.").to_owned(),
        )
    }
}
