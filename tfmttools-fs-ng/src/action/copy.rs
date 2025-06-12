use std::path::Path;

use tfmttools_core::error::TFMTResult;

use crate::{FsOption, action::Action};

pub struct CopyFile<P, Q>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    source: P,
    target: Q,
}

impl<P, Q> Action for CopyFile<P, Q>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    fn apply(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::copy(&self.source, &self.target)?;
        }

        Ok(())
    }

    fn undo(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::copy(&self.target, &self.source)?;
        }

        Ok(())
    }
}
