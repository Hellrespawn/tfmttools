#[cfg(all(feature = "serde_json", feature = "bincode"))]
compile_error!("Features `serde_json` and `bincode` are mutually exclusive.");

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::action::Action;

pub(crate) enum LoadHistoryResult {
    Loaded(History),
    New(History),
}

impl LoadHistoryResult {
    pub(crate) fn into_inner(self) -> History {
        match self {
            LoadHistoryResult::Loaded(history)
            | LoadHistoryResult::New(history) => history,
        }
    }
}

pub(crate) enum SaveHistoryResult {
    Saved,
    Exists(Utf8PathBuf),
}

pub(crate) struct History {
    path: Utf8PathBuf,
    stack: RefStack<Record>,
}

impl History {
    pub(crate) fn load(path: &Utf8Path) -> Result<LoadHistoryResult> {
        let result = if path.is_file() {
            let body = fs::read(path)?;

            LoadHistoryResult::Loaded(Self {
                path: path.to_owned(),
                stack: Self::deserialize(&body)?,
            })
        } else if path.exists() {
            return Err(eyre!(
                "History file path exists, but is not a file: {}",
                path
            ));
        } else {
            LoadHistoryResult::New(Self {
                path: path.to_owned(),
                stack: RefStack::new(),
            })
        };

        Ok(result)
    }

    #[cfg(feature = "serde_json")]
    fn deserialize(bytes: &[u8]) -> Result<RefStack<Record>> {
        Ok(serde_json::from_slice(bytes)?)
    }

    #[cfg(feature = "bincode")]
    fn deserialize(bytes: &[u8]) -> Result<RefStack<Record>> {
        Ok(bincode::deserialize(bytes)?)
    }

    pub(crate) fn save(&self) -> Result<SaveHistoryResult> {
        let result = if !self.path.is_file() && self.path.exists() {
            let tmp_dir: Utf8PathBuf = std::env::temp_dir().try_into()?;

            let tmp_file = tmp_dir.join(
                self.path.file_name().expect("history_path should be a file."),
            );

            SaveHistoryResult::Exists(tmp_file)
        } else {
            SaveHistoryResult::Saved
        };

        let path = match &result {
            SaveHistoryResult::Saved => &self.path,
            SaveHistoryResult::Exists(path) => path,
        };

        fs::create_dir_all(
            path.parent().expect("Path to file should always have a parent."),
        )?;

        let bytes = self.serialize()?;

        fs::write(path, bytes)?;

        Ok(result)
    }

    #[cfg(feature = "serde_json")]
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.stack)?)
    }

    #[cfg(feature = "bincode")]
    fn serialize(&self) -> Result<Vec<u8>> {
        Ok(bincode::serialize(&self.stack)?)
    }

    pub(crate) fn push(&mut self, record: Record) {
        self.stack.push(record);
    }

    pub(crate) fn get_records_to_undo(
        &mut self,
        n: usize,
    ) -> Option<&[Record]> {
        self.stack.refs_before_cursor(n)
    }

    pub(crate) fn get_records_to_redo(
        &mut self,
        n: usize,
    ) -> Option<&[Record]> {
        self.stack.refs_after_cursor(n)
    }

    pub(crate) fn path(&self) -> &str {
        self.path.as_ref()
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct RefStack<T> {
    inner: Vec<T>,
    cursor: usize,
}

impl<T> RefStack<T> {
    pub(crate) fn new() -> Self {
        Self { inner: Vec::new(), cursor: 0 }
    }

    pub(crate) fn push(&mut self, item: T) {
        self.inner.truncate(self.cursor);
        self.inner.push(item);
        self.cursor = self.inner.len();
    }

    #[cfg(test)]
    pub(crate) fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.inner.truncate(self.cursor);
        self.inner.extend(iter);
        self.cursor = self.inner.len();
    }

    pub(crate) fn refs_after_cursor(&mut self, n: usize) -> Option<&[T]> {
        let start = self.cursor;
        let end = std::cmp::min(self.cursor + n, self.inner.len());

        let range = start..end;

        if range.is_empty() {
            None
        } else {
            let amount = end - start;

            let items = self.inner.get(start..end);

            self.cursor += amount;

            items
        }
    }

    pub(crate) fn refs_before_cursor(&mut self, n: usize) -> Option<&[T]> {
        let start = self.cursor.saturating_sub(n);
        let end = self.cursor;

        let range = start..end;

        if range.is_empty() {
            None
        } else {
            let amount = end - start;

            let items = self.inner.get(start..end);

            self.cursor = self.cursor.saturating_sub(amount);

            items
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Record {
    actions: Vec<Action>,
    timestamp: Option<OffsetDateTime>,
}

impl Record {
    pub fn new(actions: Vec<Action>) -> Result<Self> {
        Ok(Self { actions, timestamp: Some(OffsetDateTime::now_local()?) })
    }

    pub(crate) fn iter(&self) -> impl DoubleEndedIterator<Item = &Action> {
        self.actions.iter()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty_ref_stack() {
        let mut stack: RefStack<usize> = RefStack::new();

        assert_eq!(stack.refs_before_cursor(1), None);
        assert_eq!(stack.cursor, 0);
        assert_eq!(stack.refs_before_cursor(3), None);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.refs_after_cursor(1), None);
        assert_eq!(stack.cursor, 0);
        assert_eq!(stack.refs_after_cursor(3), None);
        assert_eq!(stack.cursor, 0);
    }

    #[test]
    fn test_ref_stack_before_cursor() {
        let mut stack = RefStack::new();

        stack.push("a");
        stack.push("b");
        stack.push("c");

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.refs_before_cursor(1), Some(&["c"][..]));
        assert_eq!(stack.refs_before_cursor(1), Some(&["b"][..]));

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 1);

        stack.push("d");

        assert_eq!(stack.inner, vec!["a", "d"]);
        assert_eq!(stack.cursor, 2);
    }

    #[test]
    fn test_ref_stack_after_cursor() {
        let mut stack = RefStack::new();

        stack.push("a");
        stack.push("b");
        stack.push("c");

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.refs_before_cursor(1), Some(&["c"][..]));
        assert_eq!(stack.refs_before_cursor(1), Some(&["b"][..]));

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 1);

        assert_eq!(stack.refs_after_cursor(2), Some(&["b", "c"][..]));
        assert_eq!(stack.cursor, 3);
    }

    #[test]
    fn test_ref_stack_too_big_n() {
        let mut stack = RefStack::new();

        stack.extend(["a", "b", "c"]);

        assert_eq!(stack.refs_after_cursor(5), None);
        assert_eq!(stack.cursor, 3);

        assert_eq!(stack.refs_before_cursor(5), Some(&["a", "b", "c"][..]));
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.refs_before_cursor(5), None);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.refs_after_cursor(5), Some(&["a", "b", "c"][..]));
        assert_eq!(stack.cursor, 3);
    }
}
