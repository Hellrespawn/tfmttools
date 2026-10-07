mod error;
mod history;
mod model;

pub use error::{HistoryError, Result};
pub use history::{History, LoadHistoryResult};
pub use model::{ActionRecordMetadata, Record, RecordState, TemplateMetadata};

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
