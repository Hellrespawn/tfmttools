use std::fs;

use camino::{Utf8Path, Utf8PathBuf};
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionRecord, ActionRecordMetadata};
use tfmttools_history::History;

use crate::Connection;
use crate::entity::{ActionEntity, RecordEntity};
use crate::error::{HistoryError, Result};

pub struct SqlHistory {
    path: Utf8PathBuf,
    conn: Connection,
}

impl History {
    pub fn load(path: &Utf8Path) -> color_eyre::Result<Self>
}

impl History<ActionRecord, ActionRecordMetadata> for SqlHistory {
    fn save(&mut self) -> tfmttools_history::Result<()> {
        todo!()
    }

    fn push(
        &mut self,
        actions: Vec<ActionRecord>,
        metadata: ActionRecordMetadata,
    ) -> tfmttools_history::Result<()> {
        todo!()
    }

    fn get_previous_record(
        &self,
    ) -> tfmttools_history::Result<
        Option<&tfmttools_history::Record<ActionRecord, ActionRecordMetadata>>,
    > {
        todo!()
    }

    fn get_records_to_undo(
        &self,
        amount: Option<usize>,
    ) -> tfmttools_history::Result<
        Vec<&tfmttools_history::Record<ActionRecord, ActionRecordMetadata>>,
    > {
        todo!()
    }

    fn get_records_to_redo(
        &self,
        amount: Option<usize>,
    ) -> tfmttools_history::Result<
        Vec<&tfmttools_history::Record<ActionRecord, ActionRecordMetadata>>,
    > {
        todo!()
    }

    fn get_records_to_undo_mut(
        &mut self,
        amount: Option<usize>,
    ) -> tfmttools_history::Result<
        Vec<&mut tfmttools_history::Record<ActionRecord, ActionRecordMetadata>>,
    > {
        todo!()
    }

    fn get_records_to_redo_mut(
        &mut self,
        amount: Option<usize>,
    ) -> tfmttools_history::Result<
        Vec<&mut tfmttools_history::Record<ActionRecord, ActionRecordMetadata>>,
    > {
        todo!()
    }

    fn remove(&mut self) -> tfmttools_history::Result<()> {
        todo!()
    }
}
