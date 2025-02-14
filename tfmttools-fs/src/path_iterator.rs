use camino::{Utf8Path, Utf8PathBuf};
use ignore::{Walk, WalkBuilder};
use tfmttools_core::error::TFMTResult;

#[derive(Debug)]
pub struct PathIteratorOptions<'pio> {
    input_directory: &'pio Utf8Path,
    recursion_depth: Option<usize>,
}

impl<'pio> PathIteratorOptions<'pio> {
    pub fn new(input_directory: &'pio Utf8Path) -> Self {
        Self { input_directory, recursion_depth: None }
    }

    pub fn with_depth(
        input_directory: &'pio Utf8Path,
        recursion_depth: usize,
    ) -> Self {
        Self { input_directory, recursion_depth: Some(recursion_depth) }
    }

    pub fn recursion_depth(&self) -> usize {
        self.recursion_depth.map_or(1, |d| d + 1)
    }
}

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
    pub fn new(options: &PathIteratorOptions) -> Self {
        let walk = WalkBuilder::new(options.input_directory)
            .max_depth(Some(options.recursion_depth()))
            .sort_by_file_path(|left, right| left.cmp(right))
            .build();

        Self(walk)
    }

    pub fn single_directory(directory: &Utf8Path) -> Self {
        Self::new(&PathIteratorOptions::new(directory))
    }
}
