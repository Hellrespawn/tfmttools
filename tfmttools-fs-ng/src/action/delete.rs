use std::path::Path;

use tfmttools_core::error::TFMTResult;

use crate::{FsOption, action::Action};

pub struct RemoveFile<P, Q>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    path: P,
    backup_dir: Q,
}

impl<P, Q> Action for RemoveFile<P, Q>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    fn apply(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::remove_file(&self.path)?;
        }

        Ok(())
    }

    fn undo(&self, fs_options: &[FsOption]) -> TFMTResult {
        todo!()
    }
}

pub struct RemoveDir<P>
where
    P: AsRef<Path>,
{
    path: P,
}
