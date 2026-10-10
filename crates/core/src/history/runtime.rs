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
    pub(super) cache_usable: bool,
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
            cache_usable: true,
            connection: None,
            read_only: false,
        }
    }

    /// Check the cache boundary before fallible reporting or mutation.
    /// After a failed commit/load, call `load` successfully or discard the session.
    pub fn ensure_usable(&self) -> Result<()> {
        if self.cache_usable {
            Ok(())
        } else {
            Err(HistoryError::LoadError(
                "History session requires a successful reload".into(),
            ))
        }
    }

    pub(super) fn publish_records(&mut self, records: Vec<Record>) {
        for record in records {
            let id = record.id().expect("Stored record has an ID");
            match self.records.binary_search_by_key(&id, |r| {
                r.id().expect("Stored record has an ID")
            }) {
                Ok(position) => self.records[position] = record,
                Err(position) => self.records.insert(position, record),
            }
        }
    }

    /// # Panics
    /// Panics if a failed commit or load has not been followed by a successful reload.
    #[must_use]
    pub fn previous_record(&self) -> Option<&Record> {
        self.ensure_usable().expect("Reload history after a failed commit or load before accessing records");
        self.records.last()
    }

    /// # Panics
    /// Panics if a failed commit or load has not been followed by a successful reload.
    #[must_use]
    pub fn records_for(
        &self,
        mode: HistoryMode,
        limit: Option<usize>,
    ) -> Vec<&Record> {
        self.ensure_usable().expect("Reload history after a failed commit or load before accessing records");
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
        self.ensure_usable()?;
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

    /// # Panics
    /// Panics if a failed commit or load has not been followed by a successful reload.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ensure_usable().expect("Reload history after a failed commit or load before accessing records");
        self.records.is_empty()
    }

    /// # Panics
    /// Panics if a failed commit or load has not been followed by a successful reload.
    #[must_use]
    pub fn records(&self) -> &[Record] {
        self.ensure_usable().expect("Reload history after a failed commit or load before accessing records");
        &self.records
    }
}
