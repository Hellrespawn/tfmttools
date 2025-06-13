use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use tfmttools_core::error::TFMTResult;

use crate::FsOption;

mod copy;
mod delete;
mod r#move;

pub use copy::CopyFile;
pub use delete::{RemoveDir, RemoveFile};
pub use r#move::MoveFile;

#[typetag::serde(tag = "type")]
pub trait Action: std::fmt::Debug {
    fn apply_with(&self, fs_options: &[FsOption]) -> TFMTResult;
    fn undo_with(&self, fs_options: &[FsOption]) -> TFMTResult;

    fn redo_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        self.apply_with(fs_options)
    }

    fn apply(&self) -> TFMTResult {
        self.apply_with(&[])
    }
    fn undo(&self) -> TFMTResult {
        self.undo_with(&[])
    }

    fn redo(&self) -> TFMTResult {
        self.redo_with(&[])
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MakeDir {
    path: Utf8PathBuf,
}

#[typetag::serde]
impl Action for MakeDir {
    fn apply_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        todo!()
    }

    fn undo_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        todo!()
    }
}
