use camino::{Utf8Path, Utf8PathBuf};
use tracing::{debug, trace};

use super::{History, HistoryError, LoadHistoryResult, Result};

impl History {
    pub fn load(&mut self) -> Result<LoadHistoryResult> {
        let path = self.path.clone();

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
                path.clone()
            )))
        } else {
            debug!("Loading empty history");
            Ok(LoadHistoryResult::New)
        }
    }

    pub fn save(&mut self) -> Result<()> {
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

    fn serialize_self(&self) -> Result<Vec<u8>> {
        super::StoredHistory::current(self.records.clone()).validate()?;
        let result = serde_json::to_vec_pretty(&super::StoredHistory::current(
            self.records.clone(),
        ));

        result.map_err(|source| {
            HistoryError::SaveError(format!(
                "Unable to serialize history: {source}"
            ))
        })
    }

    fn deserialize_self(bytes: &[u8], path: &Utf8Path) -> Result<Self> {
        let stored: super::StoredHistory = serde_json::from_slice(bytes)
            .map_err(|source| {
                HistoryError::LoadError(format!(
                    "Unable to deserialize history: {source}"
                ))
            })?;
        stored.validate()?;
        let mut history = Self::new(path.to_owned());
        history.records = stored.records;
        trace!("Deserialized history:\n{:#?}", history);
        Ok(history)
    }
}
