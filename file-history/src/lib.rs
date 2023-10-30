// #![warn(missing_docs)]
#![warn(clippy::pedantic)]
//#![warn(clippy::cargo)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
//! This crate tracks moving of files and creation and deletion of folders in a reversible manner.

#[cfg(all(feature = "bincode", feature = "serde_json"))]
compile_error!("bincode and serde_json are mutually exclusive!");

/// Contains [`Action`]
pub mod change;
/// Contains [`History`]
pub mod history;

mod changelist;
mod disk;
mod util;

use std::path::PathBuf;

pub use change::{Change, ChangeType};
use changelist::ChangeList;
use disk::DiskHandler;
pub use history::History;
use thiserror::Error;

/// Wrapper for Result
pub(crate) type Result<T> = std::result::Result<T, HistoryError>;

#[derive(Error, Debug)]
/// Error relating to file-history
pub enum HistoryError {
    /// File in use
    #[error("The process cannot access the file because it is being used by another process. (os error 32):\n{0}")]
    FileInUse(PathBuf),

    /// Action was already applied.
    #[error("This action has already been applied: {0}")]
    AppliedTwice(Change),

    /// Action was already undone.
    #[error("This action has already been undone: {0}")]
    NotYetApplied(Change),

    /// Represents std::io::Error
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),

    #[cfg(feature = "bincode")]
    /// Represents bincode::Error
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),

    #[cfg(feature = "serde_json")]
    /// Represents serde_json::Error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
