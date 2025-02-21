use camino::{Utf8Path, Utf8PathBuf};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tfmttools_history_core::{
    History, HistoryError, LoadHistoryResult, Record, RecordState, Result,
};
use tracing::trace;

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
            let tmp_dir: Utf8PathBuf =
                std::env::temp_dir().try_into().map_err(|_| {
                    HistoryError::SaveError(
                        "Temporary directory is not valid UTF-8.".to_owned(),
                    )
                })?;

            let tmp_file = tmp_dir.join(
                self.path
                    .file_name()
                    .expect("history_path should be a file with a file name."),
            );

            Err(HistoryError::SaveErrorWithBackup(
                format!("{} exists but is not a file.", self.path),
                tmp_file,
            ))
        } else {
            Ok(())
        };

        let path = match &result {
            Ok(()) => &self.path,
            Err(HistoryError::SaveErrorWithBackup(_, path)) => path,
            Err(_) => return result,
        };

        let parent =
            path.parent().expect("Path to file should always have a parent.");

        fs_err::create_dir_all(parent).map_err(|err| {
            HistoryError::SaveError(format!(
                "Unable to create directory {parent}: {err}"
            ))
        })?;

        let bytes = Self::serialize(&self.stack)?;

        fs_err::write(path, bytes).map_err(|err| {
            HistoryError::SaveError(format!("Unable to write to {path}: {err}"))
        })?;

        result
    }

    fn push(&mut self, actions: Vec<A>, metadata: M) -> Result<()> {
        let record = Record::new(actions, metadata);

        self.stack.push(record);

        Ok(())
    }

    fn get_previous_record(&mut self) -> Result<Option<&Record<A, M>>> {
        Ok(self.stack.last())
    }

    fn get_records_to_undo(
        &mut self,
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
        &mut self,
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
        fs_err::remove_file(&self.path)
            .map_err(|err| HistoryError::RemoveError(err.to_string()))?;
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
            let body = fs_err::read(&path)
                .map_err(|err| HistoryError::LoadError(err.to_string()))?;

            let stack = Self::deserialize(&body)?;

            Ok((SerdeHistory { path, stack }, LoadHistoryResult::Loaded))
        } else if path.exists() {
            Err(HistoryError::LoadError(format!(
                "{} exists but is not a file.",
                path.to_owned()
            )))
        } else {
            Ok((
                SerdeHistory { path, stack: Vec::new() },
                LoadHistoryResult::New,
            ))
        }
    }

    fn serialize(stack: &Vec<Record<A, M>>) -> Result<Vec<u8>> {
        let result = serde_json::to_vec_pretty(stack);

        result.map_err(|source| {
            HistoryError::SaveError(format!(
                "Unable to serialize history: {}",
                source
            ))
        })
    }

    fn deserialize(bytes: &[u8]) -> Result<Vec<Record<A, M>>> {
        let records = serde_json::from_slice(bytes).map_err(|source| {
            HistoryError::LoadError(format!(
                "Unable to deserialize history: {}",
                source
            ))
        })?;

        trace!("Deserialized history:\n{:#?}", records);

        Ok(records)
    }
}
