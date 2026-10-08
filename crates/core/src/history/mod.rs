mod conversion;
mod database;
mod error;
mod model;
mod patch;
mod persistence;
mod runtime;
mod stored;

pub use error::{HistoryError, Result};
pub use model::{ActionRecordMetadata, Record, RecordState, TemplateMetadata};
pub use runtime::{History, LoadHistoryResult};

#[derive(Copy, Clone, Debug)]
pub enum HistoryMode {
    Undo,
    Redo,
}

impl HistoryMode {
    #[must_use]
    pub fn verb(&self) -> &str {
        match self {
            HistoryMode::Undo => "undo",
            HistoryMode::Redo => "redo",
        }
    }

    #[must_use]
    pub fn verb_capitalized(&self) -> &str {
        match self {
            HistoryMode::Undo => "Undo",
            HistoryMode::Redo => "Redo",
        }
    }
}

pub use database::history_schema_sql;
pub use patch::{BinaryPatchPair, ByteIdentity};
pub use stored::{StoredAction, StoredTagValueChange, StoredTagValueKind};
