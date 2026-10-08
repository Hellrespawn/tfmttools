use std::fs::File;

use camino::Utf8PathBuf;

use super::{
    ActionRecordMetadata, HistoryError, HistoryMode, Record, RecordState,
    Result,
};
use crate::action::Action;

#[derive(Debug, Clone, Copy)]
pub enum LoadHistoryResult {
    Loaded,
    New,
}

#[derive(Debug)]
pub struct History {
    pub(super) path: Utf8PathBuf,
    // Retain the handle for the complete load/change/save session.
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

    pub fn push(
        &mut self,
        actions: Vec<Action>,
        metadata: ActionRecordMetadata,
    ) -> Result<()> {
        let stored: Vec<_> = actions
            .into_iter()
            .map(|action| super::StoredAction::from(&action))
            .collect();
        for action in &stored {
            Action::try_from(action)?;
        }
        let mut new_record = Record::new(stored, metadata);

        *new_record.id_mut() = Some(self.records.len());

        self.records.push(new_record);

        let undone_records = self.get_all_records_to_redo()?;

        undone_records.into_iter().try_for_each(|record| -> Result<()> {
            self.set_record_state(record, RecordState::Superseded)?;

            Ok(())
        })?;

        Ok(())
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
                Box::new(self.records.iter().rev().filter(|r| {
                    matches!(
                        r.state(),
                        RecordState::Applied | RecordState::Redone
                    )
                }))
            },
            HistoryMode::Redo => {
                Box::new(
                    self.records
                        .iter()
                        .filter(|r| matches!(r.state(), RecordState::Undone)),
                )
            },
        };

        match amount {
            Some(amount) => records.take(amount).cloned().collect(),
            None => records.cloned().collect(),
        }
    }

    pub fn set_record_state(
        &mut self,
        mut record: Record,
        state: RecordState,
    ) -> Result<Record> {
        if let Some(id) = record.id() {
            let mut found_records = self
                .records
                .iter_mut()
                .filter(|r| r.id().is_some_and(|r_id| r_id == id));

            let Some(found_record) = found_records.next() else {
                return Err(HistoryError::MutError(format!(
                    "Unable to find saved record with id {id}"
                )));
            };

            if found_records.next().is_some() {
                Err(HistoryError::MutError(format!(
                    "Found multiple saved records with id {id}"
                )))
            } else {
                record.set_state(state);
                found_record.set_state(state);

                Ok(record)
            }
        } else {
            Err(HistoryError::MutError(
                "Unable to set the state of unsaved record.".to_owned(),
            ))
        }
    }

    pub fn remove(&mut self) -> Result<()> {
        self.lock_history()?;
        if let Some(connection) = &self.connection {
            let pending: i64 = connection.query_row(
                "SELECT count(*) FROM operations",
                [],
                |row| row.get(0),
            )?;
            if pending != 0 {
                return Err(HistoryError::RemoveError(
                    "History has pending recovery work".into(),
                ));
            }
        }
        self.connection.take();
        self.records.clear();
        fs_err::remove_file(&self.path)
            .map_err(|err| HistoryError::RemoveError(err.to_string()))?;
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
