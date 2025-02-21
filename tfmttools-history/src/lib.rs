mod error;
mod history;
mod record;
mod serde_history;

pub use error::{HistoryError, Result};
pub use history::{History, LoadHistoryResult};
pub use record::{Record, RecordState};
pub use serde_history::SerdeHistory;

#[derive(Copy, Clone, Debug)]
pub enum HistoryMode {
    Undo,
    Redo,
}

impl HistoryMode {
    pub fn verb(&self) -> &str {
        match self {
            HistoryMode::Undo => "undo",
            HistoryMode::Redo => "redo",
        }
    }

    pub fn verb_capitalized(&self) -> &str {
        match self {
            HistoryMode::Undo => "Undo",
            HistoryMode::Redo => "Redo",
        }
    }
}
