use std::ops::{Deref, DerefMut};

use ::serde::de::DeserializeOwned;
use ::serde::Serialize;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use fs_err as fs;

use super::record::Record;
use super::serde::HistorySerde;
use super::stack::RefStack;

pub enum LoadHistoryResult<T>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
{
    Loaded(History<T>),
    New(History<T>),
}

impl<T> Deref for LoadHistoryResult<T>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
{
    type Target = History<T>;

    fn deref(&self) -> &Self::Target {
        match self {
            LoadHistoryResult::Loaded(history)
            | LoadHistoryResult::New(history) => history,
        }
    }
}

impl<T> DerefMut for LoadHistoryResult<T>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
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

pub struct History<T>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
{
    path: Utf8PathBuf,
    stack: RefStack<Record<T>>,
}

impl<T> History<T>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
{
    pub fn load(path: &Utf8Path) -> Result<LoadHistoryResult<T>> {
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

    pub fn push(&mut self, items: Vec<T>) -> Result<()> {
        let record = Record::new(items)?;
        self.stack.push(record);
        Ok(())
    }

    pub fn get_records_to_undo(&mut self, n: usize) -> Option<&[Record<T>]> {
        self.stack.popn(n)
    }

    pub fn get_records_to_redo(&mut self, n: usize) -> Option<&[Record<T>]> {
        self.stack.unpopn(n)
    }

    pub fn path(&self) -> &str {
        self.path.as_ref()
    }
}
