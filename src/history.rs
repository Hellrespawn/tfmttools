use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::action::Action;

struct HistorySerde;

impl HistorySerde {
    #[cfg(feature = "debug")]
    fn serialize(stack: &RefStack<Record>) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(stack)?)
    }

    #[cfg(not(feature = "debug"))]
    fn serialize(stack: &RefStack<Record>) -> Result<Vec<u8>> {
        Ok(bincode::serialize(stack)?)
    }

    fn deserialize(bytes: &[u8]) -> Result<RefStack<Record>> {
        let bincode_result = bincode::deserialize(bytes);

        let json_result = serde_json::from_slice(bytes);

        if let Ok(stack) = bincode_result {
            Ok(stack)
        } else if let Ok(stack) = json_result {
            Ok(stack)
        } else {
            Err(eyre!(
                "Unable to deserialize history:\nbincode: {}\njson: {}",
                bincode_result.unwrap_err(),
                json_result.unwrap_err()
            ))
        }
    }
}

pub enum LoadHistoryResult {
    Loaded(History),
    New(History),
}

impl LoadHistoryResult {
    pub fn into_inner(self) -> History {
        match self {
            LoadHistoryResult::Loaded(history)
            | LoadHistoryResult::New(history) => history,
        }
    }
}

pub enum SaveHistoryResult {
    Saved,
    Exists(Utf8PathBuf),
}

pub struct History {
    path: Utf8PathBuf,
    stack: RefStack<Record>,
}

impl History {
    pub fn load(path: &Utf8Path) -> Result<LoadHistoryResult> {
        let result = if path.is_file() {
            let body = fs::read(path)?;

            LoadHistoryResult::Loaded(Self {
                path: path.to_owned(),
                stack: HistorySerde::deserialize(&body)?,
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

    pub fn save(&self) -> Result<SaveHistoryResult> {
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

        let bytes = HistorySerde::serialize(&self.stack)?;

        fs::write(path, bytes)?;

        Ok(result)
    }

    pub fn push(&mut self, record: Record) {
        self.stack.push(record);
    }

    pub fn get_records_to_undo(&mut self, n: usize) -> Option<&[Record]> {
        self.stack.refs_before_cursor(n)
    }

    pub fn get_records_to_redo(&mut self, n: usize) -> Option<&[Record]> {
        self.stack.refs_after_cursor(n)
    }

    pub fn path(&self) -> &str {
        self.path.as_ref()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefStack<T> {
    inner: Vec<T>,
    cursor: usize,
}

impl<T> RefStack<T> {
    pub fn new() -> Self {
        Self { inner: Vec::new(), cursor: 0 }
    }

    pub fn push(&mut self, item: T) {
        self.inner.truncate(self.cursor);
        self.inner.push(item);
        self.cursor = self.inner.len();
    }

    #[cfg(test)]
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = T>,
    {
        self.inner.truncate(self.cursor);
        self.inner.extend(iter);
        self.cursor = self.inner.len();
    }

    pub fn refs_after_cursor(&mut self, n: usize) -> Option<&[T]> {
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

    pub fn refs_before_cursor(&mut self, n: usize) -> Option<&[T]> {
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    actions: Vec<Action>,
    timestamp: Option<OffsetDateTime>,
}

impl Record {
    pub fn new(actions: Vec<Action>) -> Result<Self> {
        Ok(Self { actions, timestamp: Some(OffsetDateTime::now_local()?) })
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Action> {
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
