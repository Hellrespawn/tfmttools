use color_eyre::Result;
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

pub struct PathIterator {
    entries: VecDeque<(PathBuf, usize)>,
    recursion_depth: usize,
}

impl PathIterator {
    pub fn new(path: PathBuf, recursion_depth: usize) -> Self {
        Self { entries: vec![(path, 0)].into(), recursion_depth }
    }

    fn handle_dir(&mut self, path: &Path, depth: usize) -> Result<()> {
        // println!("depth: {depth}\trd: {}", self.recursion_depth);
        if depth >= self.recursion_depth {
            let read_dir = path.read_dir()?;

            let entries = read_dir
                .map(|r| r.map(|d| (d.path(), depth + 1)))
                .collect::<std::io::Result<Vec<_>>>()?;

            self.entries.extend(entries);
        }

        Ok(())
    }
}

impl Iterator for PathIterator {
    type Item = Result<PathBuf>;

    fn next(&mut self) -> Option<Self::Item> {
        let entry = self.entries.pop_front();

        if let Some((path, depth)) = entry {
            // dbg!(&self.entries);
            if path.is_file() {
                // println!("Handling file {}", path.display());
                Some(Ok(path))
            } else if path.is_dir() {
                println!("Handling dir {}", path.display());
                if let Err(error) = self.handle_dir(&path, depth) {
                    Some(Err(error))
                } else {
                    self.next()
                }
            } else {
                self.next()
            }
        } else {
            None
        }
    }
}
