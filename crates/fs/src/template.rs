use camino::{Utf8Path, Utf8PathBuf};
use fs_err as fs;
use tfmttools_core::util::{Utf8Directory, Utf8PathExt};

use crate::PathIterator;
use crate::error::FsResult;

pub const TEMPLATE_EXTENSIONS: [&str; 3] = ["tfmt", "jinja", "j2"];

/// Discover template files without reading or compiling their contents.
pub fn discover_templates(
    directory: &Utf8Directory,
) -> FsResult<Vec<Utf8PathBuf>> {
    PathIterator::single_directory(directory.as_path())
        .filter_map(|entry| {
            match entry {
                Ok(path)
                    if path.extension().is_some_and(|extension| {
                        TEMPLATE_EXTENSIONS.contains(&extension)
                    }) =>
                {
                    Some(Ok(path))
                },
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            }
        })
        .collect()
}

pub fn read_template(path: &Utf8Path) -> FsResult<String> {
    Ok(fs::read_to_string(path)?)
}
