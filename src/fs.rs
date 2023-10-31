use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
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
    pub fn new(path: &Utf8Path) -> Self {
        Self(WalkBuilder::new(path).max_depth(Some(1)).build())
    }

    pub fn recursive(path: &Utf8Path, recursion_depth: usize) -> Self {
        Self(WalkBuilder::new(path).max_depth(Some(recursion_depth)).build())
    }

    fn transform_iterator_result(
        result: Result<ignore::DirEntry, ignore::Error>,
    ) -> Result<Utf8PathBuf> {
        Ok(result?.into_path().try_into()?)
    }
}
