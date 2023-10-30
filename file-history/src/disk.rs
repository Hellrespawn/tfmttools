use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use fs_err as fs;
use serde::{Deserialize, Serialize};

use crate::{ChangeList, HistoryError, Result};

#[derive(Serialize, Deserialize)]
struct HistoryDto {
    applied_lists: Vec<ChangeList>,
    undone_lists: Vec<ChangeList>,
}

#[allow(clippy::module_name_repetitions)]
#[derive(PartialEq, Debug)]
pub(crate) struct DiskHandler {
    path: PathBuf,
}

impl DiskHandler {
    pub(crate) fn init_dir(directory: &Path, name: &str) -> DiskHandler {
        DiskHandler {
            path: directory.join(name).with_extension(DiskHandler::extension()),
        }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn clear(&self) -> Result<bool> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(true),
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    Ok(false)
                } else {
                    if let Some(error_code) = err.raw_os_error() {
                        if error_code == 32 {
                            return Err(HistoryError::FileInUse(
                                self.path().to_owned(),
                            ));
                        }
                    }

                    Err(err.into())
                }
            },
        }
    }

    pub(crate) fn read(&self) -> Result<(Vec<ChangeList>, Vec<ChangeList>)> {
        match fs::read(&self.path) {
            Ok(file_contents) => {
                #[cfg(feature = "bincode")]
                let history: HistoryDto = bincode::deserialize(&file_contents)?;

                #[cfg(feature = "serde_json")]
                let history: HistoryDto =
                    serde_json::from_slice(&file_contents)?;

                Ok((history.applied_lists, history.undone_lists))
            },
            Err(err) => {
                if let ErrorKind::NotFound = err.kind() {
                    Ok((Vec::new(), Vec::new()))
                } else {
                    Err(err.into())
                }
            },
        }
    }

    pub(crate) fn write(
        &self,
        applied_lists: &[ChangeList],
        undone_lists: &[ChangeList],
    ) -> Result<()> {
        let history = HistoryDto {
            applied_lists: applied_lists.to_vec(),
            undone_lists: undone_lists.to_vec(),
        };

        #[cfg(feature = "bincode")]
        let serialized = bincode::serialize(&history)?;

        #[cfg(feature = "serde_json")]
        let serialized = serde_json::to_string_pretty(&history)?;

        let result = fs::write(&self.path, serialized);

        if let Err(err) = &result {
            if let Some(error_code) = err.raw_os_error() {
                if error_code == 32 {
                    return Err(HistoryError::FileInUse(
                        self.path().to_owned(),
                    ));
                }
            }
        }

        Ok(result?)
    }

    #[cfg(feature = "serde_json")]
    fn extension() -> &'static str {
        "json"
    }

    #[cfg(feature = "bincode")]
    fn extension() -> &'static str {
        "hist"
    }
}

#[cfg(test)]
mod tests {
    use assert_fs::prelude::*;
    use assert_fs::NamedTempFile;
    use color_eyre::Result;
    use predicates::prelude::*;

    use super::*;
    use crate::change::Change;

    static PREFIX: &str = "rust-file-history-disk-";

    fn init_file(path: &Path) -> DiskHandler {
        DiskHandler { path: path.to_owned() }
    }

    fn get_temporary_file(name: &str) -> Result<NamedTempFile> {
        let name = format!("{PREFIX}{name}");
        let file = NamedTempFile::new(name)?;
        Ok(file)
    }

    fn get_test_list() -> ChangeList {
        let mut change_list = ChangeList::new();

        change_list.push(Change::mkdir("/file/test/create"));
        change_list.push(Change::rmdir("/file/test/remove"));
        change_list.push(Change::mv("/file/test/source", "/file/test/target"));

        change_list
    }

    fn get_test_queue() -> Vec<ChangeList> {
        vec![get_test_list(), get_test_list(), get_test_list(), get_test_list()]
    }

    fn write_read_compare_test_data(disk_handler: &DiskHandler) -> Result<()> {
        let applied_changes_in = get_test_queue();
        let undone_changes_in = get_test_queue();

        disk_handler.write(&applied_changes_in, &undone_changes_in)?;

        let (applied_changes_out, undone_changes_out) = disk_handler.read()?;

        assert_eq!(applied_changes_in, applied_changes_out);
        assert_eq!(undone_changes_in, undone_changes_out);

        Ok(())
    }

    #[test]
    fn test_write_and_read() -> Result<()> {
        let file = get_temporary_file("test_write_and_read")?;
        let disk_handler = init_file(file.path());

        write_read_compare_test_data(&disk_handler)?;

        Ok(())
    }

    #[test]
    fn test_clear() -> Result<()> {
        let file = get_temporary_file("test_clear")?;
        let disk_handler = init_file(file.path());

        file.assert(predicate::path::missing());

        assert!(!disk_handler.clear()?);

        // These two indicate the same thing.
        assert!(!disk_handler.clear()?);
        file.assert(predicate::path::missing());

        Ok(())
    }

    #[test]
    fn test_write_and_read_from_clear() -> Result<()> {
        let file = get_temporary_file("test_write_and_read_from_clear()")?;
        let disk_handler = init_file(file.path());

        assert!(!disk_handler.clear()?);

        write_read_compare_test_data(&disk_handler)?;

        assert!(disk_handler.clear()?);
        assert!(!disk_handler.clear()?);

        Ok(())
    }
}
