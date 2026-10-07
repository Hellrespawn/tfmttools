mod conversion;
mod error;
mod legacy_tag_keys;
mod migration;
mod model;
mod persistence;
mod runtime;
mod schema;
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

pub use schema::history_schema_json;
pub use stored::{
    CURRENT_SCHEMA_VERSION, StoredAction, StoredHistory, StoredTagValueChange,
    StoredTagValueKind,
};
