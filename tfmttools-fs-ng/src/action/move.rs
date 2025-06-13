use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use tfmttools_core::error::TFMTResult;

use crate::FsOption;
use crate::action::Action;

#[derive(Debug, Serialize, Deserialize)]
pub struct MoveFile {
    source: Utf8PathBuf,
    target: Utf8PathBuf,
}

#[typetag::serde]
impl Action for MoveFile {
    fn apply_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::rename(&self.source, &self.target)?;
        }

        Ok(())
    }

    fn undo_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::rename(&self.target, &self.source)?;
        }

        Ok(())
    }
}
