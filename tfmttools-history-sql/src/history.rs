use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use tfmttools_core::action::Action;

use crate::Connection;
use crate::entity::{ActionEntity, RecordEntity};
use crate::error::{HistoryError, Result};

pub enum LoadHistoryResult {
    Loaded(History),
    New(History),
}

impl LoadHistoryResult {
    pub fn unwrap(self) -> History {
        match self {
            LoadHistoryResult::Loaded(history)
            | LoadHistoryResult::New(history) => history,
        }
    }

    pub fn unwrap_ref(&self) -> &History {
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

pub struct History {
    path: Utf8PathBuf,
    conn: Connection,
}

impl History {
    pub fn open(path: &Utf8Path) -> Result<LoadHistoryResult> {
        let history = if path.exists() && !path.is_file() {
            return Err(HistoryError::PathIsNotAFile(path.to_owned()));
        } else {
            let mut conn = Connection::open(path)?;

            crate::migration::migrate_database(&mut conn)?;

            if path.is_file() {
                LoadHistoryResult::Loaded(Self { path: path.to_owned(), conn })
            } else {
                LoadHistoryResult::New(Self { path: path.to_owned(), conn })
            }
        };

        Ok(history)
    }

    pub fn insert_actions(
        &mut self,
        actions: &[Action],
        template: &str,
        arguments: &str,
    ) -> Result<(RecordEntity, Vec<ActionEntity>)> {
        let record_entity =
            RecordEntity::insert(&mut self.conn, template, arguments)?;

        let action_entities = actions
            .iter()
            .map(|action| {
                ActionEntity::insert_from_action(
                    &mut self.conn,
                    action,
                    record_entity.id,
                )
            })
            .collect::<rusqlite::Result<Vec<_>>>()?;

        RecordEntity::supersede_undone_records(&mut self.conn, &record_entity)?;

        Ok((record_entity, action_entities))
    }

    pub fn undo_records<F>(&mut self, amount: usize, function: F) -> Result<()>
    where
        F: Fn(Action) -> color_eyre::Result<()>,
    {
        let records =
            RecordEntity::get_records_to_undo(&mut self.conn, amount)?;

        for record in records {
            let actions = record.get_actions(&mut self.conn)?;
        }

        todo!()

        // Start transaction
        // undo actions
        // mark undone
        // commit
    }
}
