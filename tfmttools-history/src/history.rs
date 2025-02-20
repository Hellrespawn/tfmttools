use camino::{Utf8Path, Utf8PathBuf};
use fs_err as fs;
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::record::Record;
use super::serde::HistorySerde;
use super::stack::RefStack;
use crate::record::RecordState;
use crate::{HistoryError, Result};

pub enum LoadHistoryResultNew {
    Loaded,
    New,
}

pub enum LoadHistoryResult<T, M>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
    M: std::fmt::Debug + Serialize + DeserializeOwned,
{
    Loaded(History<T, M>),
    New(History<T, M>),
}

impl<T, M> LoadHistoryResult<T, M>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
    M: std::fmt::Debug + Serialize + DeserializeOwned,
{
    pub fn unwrap(self) -> History<T, M> {
        match self {
            LoadHistoryResult::Loaded(history)
            | LoadHistoryResult::New(history) => history,
        }
    }

    pub fn unwrap_ref(&self) -> &History<T, M> {
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

pub trait HistoryExt<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned,
    M: std::fmt::Debug + Serialize + DeserializeOwned,
{
    fn push(&mut self, actions: Vec<A>, metadata: M) -> Result<()>;

    fn get_records_to_undo(
        &self,
        amount: Option<usize>,
    ) -> Result<Vec<Record<A, M>>>;

    fn get_records_to_redo(
        &self,
        amount: Option<usize>,
    ) -> Result<Vec<Record<A, M>>>;

    fn set_record_state(
        &mut self,
        record: &mut Record<A, M>,
        state: RecordState,
    ) -> Result<()>;

    fn get_n_records_to_undo(
        &self,
        amount: usize,
    ) -> Result<Vec<Record<A, M>>> {
        self.get_records_to_undo(Some(amount))
    }

    fn get_n_records_to_redo(
        &self,
        amount: usize,
    ) -> Result<Vec<Record<A, M>>> {
        self.get_records_to_redo(Some(amount))
    }

    fn get_all_records_to_undo(&self) -> Result<Vec<Record<A, M>>> {
        self.get_records_to_undo(None)
    }

    fn get_all_records_to_redo(&self) -> Result<Vec<Record<A, M>>> {
        self.get_records_to_redo(None)
    }

    fn mark_record_undone(&mut self, record: &mut Record<A, M>) -> Result<()> {
        self.set_record_state(record, RecordState::Undone)
    }

    fn mark_record_redone(&mut self, record: &mut Record<A, M>) -> Result<()> {
        self.set_record_state(record, RecordState::Redone)
    }
}

pub struct History<T, M>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
    M: std::fmt::Debug + Serialize + DeserializeOwned,
{
    path: Utf8PathBuf,
    stack: RefStack<Record<T, M>>,
}

impl<T, M> History<T, M>
where
    T: std::fmt::Debug + Serialize + DeserializeOwned,
    M: std::fmt::Debug + Serialize + DeserializeOwned,
{
    pub fn load(path: &Utf8Path) -> Result<LoadHistoryResult<T, M>> {
        let result = if path.is_file() {
            let body = fs::read(path)?;

            LoadHistoryResult::Loaded(Self {
                path: path.to_owned(),
                stack: HistorySerde::deserialize(&body)?,
            })
        } else if path.exists() {
            return Err(HistoryError::PathIsNotAFile(path.to_owned()));
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

    pub fn push(&mut self, record: Record<T, M>) -> Result<()> {
        self.stack.push(record);
        Ok(())
    }

    pub fn get_records_to_undo(
        &self,
    ) -> impl ExactSizeIterator<Item = &Record<T, M>> {
        self.stack.get_unpopped_refs().iter().rev()
    }

    pub fn get_records_to_redo(
        &self,
    ) -> impl ExactSizeIterator<Item = &Record<T, M>> {
        self.stack.get_popped_refs().iter()
    }

    pub fn pop_records_to_undo(
        &mut self,
        n: usize,
    ) -> impl ExactSizeIterator<Item = &Record<T, M>> {
        self.stack.pop_refs(n).iter().rev()
    }

    pub fn unpop_records_to_redo(
        &mut self,
        n: usize,
    ) -> impl ExactSizeIterator<Item = &Record<T, M>> {
        self.stack.unpop_refs(n).iter()
    }

    pub fn path(&self) -> &str {
        self.path.as_ref()
    }

    pub fn find_record<P>(&self, predicate: P) -> Option<&Record<T, M>>
    where
        P: Fn(&Record<T, M>) -> bool,
    {
        self.stack.find(predicate)
    }
}
