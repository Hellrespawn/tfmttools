mod history;
mod record;
mod serde;
mod stack;

pub use history::{History, LoadHistoryResult, SaveHistoryResult};
pub use record::Record;

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
