mod args;
mod commands;
mod config;
mod main;
mod ui;
mod util;

pub mod preview;

use console::Term;
use once_cell::sync::Lazy;

pub static TERM: Lazy<Term> = Lazy::new(Term::stdout);

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

pub use main::main;
