use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use rusqlite::Params;
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_history_core::{History, HistoryError, Record, Result};

use crate::Connection;
use crate::entity::{ActionEntity, RecordEntity};

pub struct SqlHistory {
    path: Utf8PathBuf,
    conn: Connection,
}

impl SqlHistory {
    pub fn load(path: &Utf8Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|err| HistoryError::LoadError(err.to_string()))?;
        let path = path.to_owned();

        let mut history = SqlHistory { path, conn };

        history.begin_transaction()?;

        Ok(history)
    }

    fn execute<P>(&mut self, sql: &str, params: P) -> Result<usize>
    where
        P: Params,
    {
        self.conn
            .0
            .execute(sql, params)
            .map_err(|err| HistoryError::MiscError(err.to_string()))
    }

    fn begin_transaction(&mut self) -> Result<()> {
        self.execute("BEGIN TRANSACTION", ())?;

        Ok(())
    }

    fn commit_transaction(&mut self) -> Result<()> {
        self.execute("COMMIT", ())?;

        Ok(())
    }

    fn rollback_transaction(&mut self) -> Result<()> {
        self.execute("ROLLBACK", ())?;

        Ok(())
    }
}

impl Drop for SqlHistory {
    fn drop(&mut self) {
        let _ = self.rollback_transaction();
    }
}

impl History<Action, ActionRecordMetadata> for SqlHistory {
    fn save(&mut self) -> Result<()> {
        self.commit_transaction()?;
        self.begin_transaction()?;

        Ok(())
    }

    fn push(
        &mut self,
        actions: Vec<Action>,
        metadata: ActionRecordMetadata,
    ) -> Result<()> {
        let record = RecordEntity::insert(
            &mut self.conn,
            metadata.template(),
            &metadata.arguments().join(";"),
        )
        .map_err(|err| HistoryError::SaveError(err.to_string()))?;

        let _actions = actions
            .iter()
            .map(|action| {
                ActionEntity::insert_from_action(
                    &mut self.conn,
                    action,
                    record.id,
                )
            })
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| HistoryError::SaveError(err.to_string()))?;

        Ok(())
    }

    fn get_previous_record(
        &mut self,
    ) -> Result<Option<&Record<Action, ActionRecordMetadata>>> {
        let option = RecordEntity::get_previous(&mut self.conn)
            .map_err(|err| HistoryError::LoadError(err.to_string()))?;

        Ok(option.map(|entity| Record::from(entity)).as_ref())
    }

    fn get_records_to_undo(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&Record<Action, ActionRecordMetadata>>> {
        todo!()
    }

    fn get_records_to_redo(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&Record<Action, ActionRecordMetadata>>> {
        todo!()
    }

    fn get_records_to_undo_mut(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&mut Record<Action, ActionRecordMetadata>>> {
        todo!()
    }

    fn get_records_to_redo_mut(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&mut Record<Action, ActionRecordMetadata>>> {
        todo!()
    }

    fn remove(&mut self) -> Result<()> {
        todo!()
    }
}
