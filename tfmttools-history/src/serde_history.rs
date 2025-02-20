use camino::{Utf8Path, Utf8PathBuf};
use fs_err as fs;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::trace;

use crate::error::Result;
use crate::history::LoadHistoryResultNew;
use crate::record::RecordState;
use crate::serde::HistorySerde;
use crate::stack::RefStack;
use crate::{HistoryError, HistoryExt, LoadHistoryResult, Record};

pub enum SaveHistoryResult {
    Saved,
    Exists(Utf8PathBuf),
}

pub struct SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    path: Utf8PathBuf,
    stack: Vec<Record<A, M>>,
}

impl<A, M> HistoryExt<A, M> for SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    fn push(&mut self, actions: Vec<A>, metadata: M) -> Result<()> {
        let record = Record::new(actions, metadata);

        self.stack.push(record);

        Ok(())
    }

    fn get_records_to_undo(
        &self,
        amount: Option<usize>,
    ) -> Result<Vec<Record<A, M>>> {
        let iter = self
            .stack
            .iter()
            .rev()
            .filter(|r| {
                matches!(r.state(), RecordState::Applied | RecordState::Redone)
            })
            .cloned();

        let actions = if let Some(amount) = amount {
            iter.take(amount).collect()
        } else {
            iter.collect()
        };

        Ok(actions)
    }

    fn get_records_to_redo(
        &self,
        amount: Option<usize>,
    ) -> Result<Vec<Record<A, M>>> {
        let iter = self
            .stack
            .iter()
            .filter(|r| matches!(r.state(), RecordState::Undone))
            .cloned();

        let actions = if let Some(amount) = amount {
            iter.take(amount).collect()
        } else {
            iter.collect()
        };

        Ok(actions)
    }

    fn set_record_state(
        &mut self,
        record: &mut Record<A, M>,
        state: crate::record::RecordState,
    ) -> Result<()> {
        todo!()
    }
}

impl<A, M> SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    fn load(&mut self, path: &Utf8Path) -> Result<LoadHistoryResultNew> {
        if path.is_file() {
            let body = fs::read(path)?;

            self.stack = Self::deserialize(&body)?;
            self.path = path.to_owned();

            Ok(LoadHistoryResultNew::Loaded)
        } else if path.exists() {
            Err(HistoryError::PathIsNotAFile(path.to_owned()))
        } else {
            Ok(LoadHistoryResultNew::New)
        }
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

        let bytes = Self::serialize(&self.stack)?;

        fs::write(path, bytes)?;

        Ok(result)
    }

    fn serialize(stack: &Vec<Record<A, M>>) -> Result<Vec<u8>> {
        let result = serde_json::to_vec_pretty(stack);

        let bytes =
            result.map_err(|source| HistoryError::Serialize { source })?;

        Ok(bytes)
    }

    fn deserialize(bytes: &[u8]) -> Result<Vec<Record<A, M>>> {
        let stack = serde_json::from_slice(bytes)
            .map_err(|source| HistoryError::Deserialize { source })?;

        trace!("Deserialized history:\n{:#?}", stack);

        Ok(stack)
    }
}
