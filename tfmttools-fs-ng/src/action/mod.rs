use std::path::Path;

use tfmttools_core::error::TFMTResult;

use crate::FsOption;

mod copy;
mod delete;
mod r#move;

pub trait Action {
    fn apply(&self, fs_options: &[FsOption]) -> TFMTResult;
    fn undo(&self, fs_options: &[FsOption]) -> TFMTResult;

    fn redo(&self, fs_options: &[FsOption]) -> TFMTResult {
        self.apply(fs_options)
    }
}

pub struct MakeDir<P>
where
    P: AsRef<Path>,
{
    path: P,
}
