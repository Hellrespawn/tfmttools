use camino::{Utf8Path, Utf8PathBuf};
use ignore::{Walk, WalkBuilder};
use tfmttools_core::error::TFMTResult;

pub struct PathIterator(Walk);

impl Iterator for PathIterator {
    type Item = TFMTResult<Utf8PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self
            .0
            .next()?
            .map_err(|e| e.into())
            .and_then(|d| Ok(d.into_path().try_into()?));

        Some(result)
    }
}

impl PathIterator {
    pub fn new(path: &Utf8Path, recursion_depth: Option<usize>) -> Self {
        let walk = WalkBuilder::new(path)
            .max_depth(Some(recursion_depth.map_or(1, |d| d + 1)))
            .sort_by_file_path(|left, right| left.cmp(right))
            .build();

        Self(walk)
    }
}
