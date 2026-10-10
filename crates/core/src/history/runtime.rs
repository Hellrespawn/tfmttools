use std::fs::File;

use camino::Utf8PathBuf;

use super::{HistoryError, HistoryMode, Record, Result};

#[derive(Debug, Clone, Copy)]
pub enum LoadHistoryResult {
    Loaded,
    New,
}

#[derive(Debug)]
pub struct History {
    pub(super) path: Utf8PathBuf,
    // Retain the handle for the complete history session.
    pub(super) lock_file: Option<File>,

    pub(super) records: Vec<Record>,
    pub(super) connection: Option<rusqlite::Connection>,
    pub(super) read_only: bool,
}

impl History {
    #[must_use]
    pub fn new(path: Utf8PathBuf) -> Self {
        Self {
            path,
            lock_file: None,
            records: Vec::new(),
            connection: None,
            read_only: false,
        }
    }

    #[must_use]
    pub fn previous_record(&self) -> Option<&Record> {
        self.records.last()
    }

    #[must_use]
    pub fn records_for(
        &self,
        mode: HistoryMode,
        limit: Option<usize>,
    ) -> Vec<&Record> {
        let limit = limit.unwrap_or(usize::MAX);
        match mode {
            HistoryMode::Undo => {
                self.records
                    .iter()
                    .rev()
                    .filter(|r| r.can_undo())
                    .take(limit)
                    .collect()
            },
            HistoryMode::Redo => {
                self.records
                    .iter()
                    .filter(|r| r.can_redo())
                    .take(limit)
                    .collect()
            },
        }
    }

    pub fn remove(&mut self) -> Result<()> {
        if self.read_only {
            return Err(HistoryError::RemoveError("Read-only history".into()));
        }
        self.lock_history()?;
        if let Some(connection) = &self.connection {
            let pending: i64 = connection.query_row(
                "SELECT count(*) FROM attempts",
                [],
                |row| row.get(0),
            )?;
            if pending != 0 {
                return Err(HistoryError::RemoveError(
                    "History has an unresolved attempt".into(),
                ));
            }
        }
        let path = if let Some(connection) = &self.connection {
            Utf8PathBuf::from(connection.path().ok_or_else(|| {
                HistoryError::RemoveError(
                    "History database path unavailable".into(),
                )
            })?)
        } else {
            super::persistence::resolve_history_path(&self.path)?
        };
        self.connection.take();
        fs_err::remove_file(&path)
            .map_err(|err| HistoryError::RemoveError(err.to_string()))?;
        self.records.clear();
        Ok(())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    #[must_use]
    pub fn records(&self) -> &[Record] {
        &self.records
    }
}
