use crate::action::Action;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::{eyre::eyre, Result};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

pub(crate) enum SaveHistoryResult {
    Saved,
    Exists(Utf8PathBuf),
}

pub(crate) struct History {
    path: Utf8PathBuf,
    stack: RefStack<Record>,
}

impl History {
    pub(crate) fn load(path: &Utf8Path) -> Result<History> {
        let stack = if path.is_file() {
            let body = fs::read_to_string(path)?;

            serde_json::from_str(&body)?
        } else if path.exists() {
            return Err(eyre!(
                "History file path exists, but is not a file: {}",
                path
            ));
        } else {
            RefStack::new()
        };

        Ok(Self { path: path.to_owned(), stack })
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
        fs::write(path, serde_json::to_string(&self.stack)?)?;

        Ok(result)
    }

    pub(crate) fn push(&mut self, record: Record) {
        self.stack.push(record);
    }

    pub(crate) fn pop_ref(&mut self) -> Option<&Record> {
        self.stack.pop_ref()
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
        self.inner.truncate(self.inner.len() - self.cursor);
        self.cursor = 0;
        self.inner.push(item);
    }

    pub(crate) fn pop_ref(&mut self) -> Option<&T> {
        let item = self.inner.get(self.inner.len() - self.cursor - 1)?;

        self.cursor += 1;

        Some(item)
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
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ref_stack() {
        let mut stack = RefStack::new();

        stack.push("a");
        stack.push("b");
        stack.push("c");

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 0);

        assert_eq!(stack.pop_ref(), Some(&"c"));
        assert_eq!(stack.pop_ref(), Some(&"b"));

        assert_eq!(stack.inner, vec!["a", "b", "c"]);
        assert_eq!(stack.cursor, 2);

        stack.push("d");

        assert_eq!(stack.inner, vec!["a", "d"]);
        assert_eq!(stack.cursor, 0);
    }
}
