use std::path::Path;

use tfmttools_core::error::TFMTResult;

use crate::{FsOption, action::Action};

pub struct MoveFile<P, Q>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    source: P,
    target: Q,
}

impl<P, Q> Action for MoveFile<P, Q>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    fn apply(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::rename(&self.source, &self.target)?;
        }

        Ok(())
    }

    fn undo(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::rename(&self.target, &self.source)?;
        }

        Ok(())
    }
}
