use camino::{Utf8Path, Utf8PathBuf};
use fs_err as fs;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tracing::trace;

use crate::error::Result;
use crate::history::LoadHistoryResult;
use crate::record::RecordState;
use crate::{History, HistoryError, Record};

#[derive(Debug)]
pub struct SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    path: Utf8PathBuf,
    stack: Vec<Record<A, M>>,
}

impl<A, M> History<A, M> for SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    fn save(&mut self) -> Result<()> {
        let result = if !self.path.is_file() && self.path.exists() {
            let tmp_dir: Utf8PathBuf = std::env::temp_dir().try_into()?;

            let tmp_file = tmp_dir.join(
                self.path.file_name().expect("history_path should be a file."),
            );

            Err(HistoryError::SaveErrorWithBackup {
                expected: self.path.to_owned(),
                actual: tmp_file,
            })
        } else {
            Ok(())
        };

        let path = match &result {
            Ok(()) => &self.path,
            Err(HistoryError::PathIsNotAFile(path)) => path,
            Err(_) => return result,
        };

        fs::create_dir_all(
            path.parent().expect("Path to file should always have a parent."),
        )?;

        let bytes = Self::serialize(&self.stack)?;

        fs::write(path, bytes)?;

        result
    }

    fn push(&mut self, actions: Vec<A>, metadata: M) -> Result<()> {
        let record = Record::new(actions, metadata);

        self.stack.push(record);

        Ok(())
    }

    fn get_previous_record(&self) -> Result<Option<&Record<A, M>>> {
        Ok(self.stack.last())
    }

    fn get_records_to_undo(
        &self,
        amount: Option<usize>,
    ) -> Result<Vec<&Record<A, M>>> {
        let iter = self.stack.iter().rev().filter(|r| {
            matches!(r.state(), RecordState::Applied | RecordState::Redone)
        });

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
    ) -> Result<Vec<&Record<A, M>>> {
        let iter = self
            .stack
            .iter()
            .filter(|r| matches!(r.state(), RecordState::Undone));

        let actions = if let Some(amount) = amount {
            iter.take(amount).collect()
        } else {
            iter.collect()
        };

        Ok(actions)
    }

    fn get_records_to_undo_mut(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&mut Record<A, M>>> {
        let iter = self.stack.iter_mut().rev().filter(|r| {
            matches!(r.state(), RecordState::Applied | RecordState::Redone)
        });

        let actions = if let Some(amount) = amount {
            iter.take(amount).collect()
        } else {
            iter.collect()
        };

        Ok(actions)
    }

    fn get_records_to_redo_mut(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&mut Record<A, M>>> {
        let iter = self
            .stack
            .iter_mut()
            .filter(|r| matches!(r.state(), RecordState::Undone));

        let actions = if let Some(amount) = amount {
            iter.take(amount).collect()
        } else {
            iter.collect()
        };

        Ok(actions)
    }

    fn remove(&mut self) -> Result<()> {
        self.stack.clear();
        fs_err::remove_file(&self.path)?;
        Ok(())
    }
}

impl<A, M> SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    pub fn load(path: &Utf8Path) -> Result<(Self, LoadHistoryResult)> {
        let path = path.to_owned();

        if path.is_file() {
            let body = fs::read(&path)?;

            let stack = Self::deserialize(&body)?;

            Ok((SerdeHistory { path, stack }, LoadHistoryResult::Loaded))
        } else if path.exists() {
            Err(HistoryError::PathIsNotAFile(path.to_owned()))
        } else {
            Ok((
                SerdeHistory { path, stack: Vec::new() },
                LoadHistoryResult::New,
            ))
        }
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
