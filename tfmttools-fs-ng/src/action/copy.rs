use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use tfmttools_core::error::TFMTResult;

use crate::FsOption;
use crate::action::Action;

#[derive(Debug, Serialize, Deserialize)]
pub struct CopyFile {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
}

impl CopyFile {
    pub fn new(source: Utf8PathBuf, target: Utf8PathBuf) -> Self {
        Self { source, target }
    }
}

#[typetag::serde]
impl Action for CopyFile {
    fn apply_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::copy(&self.source, &self.target)?;
        }

        Ok(())
    }

    fn undo_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::remove_file(&self.target)?;
        }

        Ok(())
    }
}
