use std::fs::File;

use camino::Utf8PathBuf;

use super::{HistoryError, HistoryMode, Record, RecordState, Result};

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

    pub fn get_previous_record(&self) -> Result<Option<Record>> {
        Ok(self.records.last().cloned())
    }

    pub fn get_records_to_undo(
        &self,
        amount: Option<usize>,
    ) -> Result<Vec<Record>> {
        Ok(self.collect_records(HistoryMode::Undo, amount))
    }

    pub fn get_records_to_redo(
        &self,
        amount: Option<usize>,
    ) -> Result<Vec<Record>> {
        Ok(self.collect_records(HistoryMode::Redo, amount))
    }

    pub fn get_n_records_to_undo(&self, amount: usize) -> Result<Vec<Record>> {
        self.get_records_to_undo(Some(amount))
    }

    pub fn get_n_records_to_redo(&self, amount: usize) -> Result<Vec<Record>> {
        self.get_records_to_redo(Some(amount))
    }

    pub fn get_all_records_to_undo(&self) -> Result<Vec<Record>> {
        self.get_records_to_undo(None)
    }

    pub fn get_all_records_to_redo(&self) -> Result<Vec<Record>> {
        self.get_records_to_redo(None)
    }

    fn collect_records(
        &self,
        mode: HistoryMode,
        amount: Option<usize>,
    ) -> Vec<Record> {
        let records: Box<dyn Iterator<Item = &Record> + '_> = match mode {
            HistoryMode::Undo => {
                Box::new(
                    self.records.iter().rev().filter(|r| r.applied_count() > 0),
                )
            },
            HistoryMode::Redo => {
                Box::new(self.records.iter().filter(|r| {
                    r.redo_allowed()
                        && r.applied_count() < r.len()
                        && r.state() != RecordState::Superseded
                }))
            },
        };

        match amount {
            Some(amount) => records.take(amount).cloned().collect(),
            None => records.cloned().collect(),
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
