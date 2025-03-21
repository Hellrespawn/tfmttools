use camino::{Utf8Path, Utf8PathBuf};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tfmttools_history_core::{
    History, HistoryError, LoadHistoryResult, Record, RecordState, Result,
};
use tracing::{debug, trace};

#[derive(Debug, Serialize, Deserialize)]
#[serde(
    bound = "A: Serialize + DeserializeOwned, M: Serialize + DeserializeOwned"
)]
pub struct SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    #[serde(skip)]
    path: Utf8PathBuf,

    records: Vec<Record<A, M>>,
}

impl<A, M> History<A, M> for SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    fn load(&mut self) -> Result<LoadHistoryResult> {
        let path = self.path.to_owned();

        if path.is_file() {
            let body = fs_err::read(&path)
                .map_err(|err| HistoryError::LoadError(err.to_string()))?;

            let history = Self::deserialize_self(&body, &path)?;

            self.records = history.records;

            debug!("Loaded history from {path}");

            Ok(LoadHistoryResult::Loaded)
        } else if path.exists() {
            Err(HistoryError::LoadError(format!(
                "{} exists but is not a file.",
                path.to_owned()
            )))
        } else {
            debug!("Loading empty history");
            Ok(LoadHistoryResult::New)
        }
    }

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

        let bytes = self.serialize_self()?;

        fs_err::write(path, bytes).map_err(|err| {
            HistoryError::SaveError(format!("Unable to write to {path}: {err}"))
        })?;

        result
    }

    fn push(&mut self, actions: Vec<A>, metadata: M) -> Result<()> {
        let mut new_record = Record::new(actions, metadata);

        *new_record.id_mut() = Some(self.records.len());

        self.records.push(new_record);

        let undone_records = self.get_all_records_to_redo()?;

        undone_records.into_iter().try_for_each(|record| {
            self.set_record_state(record, RecordState::Superseded)?;

            Ok(())
        })?;

        Ok(())
    }

    fn get_previous_record(&mut self) -> Result<Option<Record<A, M>>> {
        Ok(self.records.last().cloned())
    }

    fn get_records_to_undo(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<Record<A, M>>> {
        let iter = self
            .records
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
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<Record<A, M>>> {
        let iter = self
            .records
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
        mut record: Record<A, M>,
        state: RecordState,
    ) -> Result<Record<A, M>> {
        if let Some(id) = record.id() {
            let found_records = self
                .records
                .iter_mut()
                .filter(|r| r.id().is_some_and(|r_id| r_id == id))
                .collect::<Vec<_>>();

            if found_records.is_empty() {
                Err(HistoryError::MutError(format!(
                    "Unable to find saved record with id {}",
                    id
                )))
            } else if found_records.len() > 1 {
                Err(HistoryError::MutError(format!(
                    "Found multiple saved records with id {}",
                    id
                )))
            } else {
                record.set_state(state);
                for found_record in found_records {
                    found_record.set_state(state);
                }

                Ok(record)
            }
        } else {
            Err(HistoryError::MutError(
                "Unable to set the state of unsaved record.".to_owned(),
            ))
        }
    }

    fn remove(&mut self) -> Result<()> {
        self.records.clear();
        fs_err::remove_file(&self.path)
            .map_err(|err| HistoryError::RemoveError(err.to_string()))?;
        Ok(())
    }

    fn is_empty(&mut self) -> bool {
        self.records.is_empty()
    }
}

impl<A, M> SerdeHistory<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
    M: std::fmt::Debug + Serialize + DeserializeOwned + Clone,
{
    pub fn new(path: Utf8PathBuf) -> Self {
        Self { path, records: Vec::new() }
    }

    fn serialize_self(&self) -> Result<Vec<u8>> {
        let result = serde_json::to_vec_pretty(self);

        result.map_err(|source| {
            HistoryError::SaveError(format!(
                "Unable to serialize history: {}",
                source
            ))
        })
    }

    fn deserialize_self(bytes: &[u8], path: &Utf8Path) -> Result<Self> {
        let mut history: SerdeHistory<A, M> = serde_json::from_slice(bytes)
            .map_err(|source| {
                HistoryError::LoadError(format!(
                    "Unable to deserialize history: {}",
                    source
                ))
            })?;

        history.path = path.to_owned();

        trace!("Deserialized history:\n{:#?}", history);

        Ok(history)
    }
}
