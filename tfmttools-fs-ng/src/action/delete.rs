use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use tfmttools_core::error::{TFMTError, TFMTResult};

use crate::FsOption;
use crate::action::Action;
use crate::checksum::get_file_checksum;

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveFile {
    path: Utf8PathBuf,
    backup_dir: Utf8PathBuf,
    checksum: String,
}

impl RemoveFile {
    pub fn new(path: Utf8PathBuf, backup_dir: Utf8PathBuf) -> TFMTResult<Self> {
        let checksum = get_file_checksum(&path)?;

        Ok(Self { path, backup_dir, checksum })
    }

    fn get_backup_path(&self) -> TFMTResult<Utf8PathBuf> {
        let filename = self
            .path
            .file_name()
            .ok_or(TFMTError::NotAFile(self.path.clone()))?;

        let backup_path =
            self.backup_dir.join(format!("{}_{}", filename, self.checksum));

        Ok(backup_path)
    }
}

#[typetag::serde]
impl Action for RemoveFile {
    fn apply_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::copy(&self.path, &self.get_backup_path()?)?;
            fs_err::remove_file(&self.path)?;
        }

        Ok(())
    }

    fn undo_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::copy(&self.get_backup_path()?, &self.path)?;
            fs_err::remove_file(&self.get_backup_path()?)?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RemoveDir {
    path: Utf8PathBuf,
}

#[typetag::serde]
impl Action for RemoveDir {
    fn apply_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::remove_dir(&self.path)?;
        }

        Ok(())
    }

    fn undo_with(&self, fs_options: &[FsOption]) -> TFMTResult {
        if !fs_options.contains(&FsOption::DryRun) {
            fs_err::create_dir(&self.path)?;
        }

        Ok(())
    }
}
