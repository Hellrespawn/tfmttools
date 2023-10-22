use crate::{ActionGroup, HistoryError, Result};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize)]
struct HistoryOnDisk {
    applied_groups: Vec<ActionGroup>,
    undone_groups: Vec<ActionGroup>,
}

#[allow(clippy::module_name_repetitions)]
#[derive(PartialEq, Debug)]
pub(crate) struct DiskHandler {
    path: PathBuf,
}

impl DiskHandler {
    pub(crate) fn init_dir(directory: &Path, name: &str) -> DiskHandler {
        DiskHandler {
            path: directory
                .join(name)
                .with_extension(DiskHandler::extension()),
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
            }
        }
    }

    pub(crate) fn read(&self) -> Result<(Vec<ActionGroup>, Vec<ActionGroup>)> {
        match fs::read(&self.path) {
            Ok(file_contents) => {
                #[cfg(feature = "bincode")]
                let history: HistoryOnDisk =
                    bincode::deserialize(&file_contents)?;

                #[cfg(feature = "serde_json")]
                let history: HistoryOnDisk =
                    serde_json::from_slice(&file_contents)?;

                Ok((history.applied_groups, history.undone_groups))
            }
            Err(err) => {
                if let ErrorKind::NotFound = err.kind() {
                    Ok((Vec::new(), Vec::new()))
                } else {
                    Err(err.into())
                }
            }
        }
    }

    pub(crate) fn write(
        &self,
        applied_groups: &[ActionGroup],
        undone_groups: &[ActionGroup],
    ) -> Result<()> {
        let history = HistoryOnDisk {
            applied_groups: applied_groups.to_vec(),
            undone_groups: undone_groups.to_vec(),
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
    use super::*;
    use crate::action::Action;
    use assert_fs::prelude::*;
    use assert_fs::NamedTempFile;
    use color_eyre::Result;
    use predicates::prelude::*;

    static PREFIX: &str = "rust-file-history-disk-";

    fn init_file(path: &Path) -> DiskHandler {
        DiskHandler {
            path: path.to_owned(),
        }
    }

    fn get_temporary_file(name: &str) -> Result<NamedTempFile> {
        let name = format!("{PREFIX}{name}");
        let file = NamedTempFile::new(name)?;
        Ok(file)
    }

    fn get_test_group() -> ActionGroup {
        let mut action_group = ActionGroup::new();

        action_group.push(Action::mkdir("/file/test/create"));
        action_group.push(Action::rmdir("/file/test/remove"));
        action_group.push(Action::mv("/file/test/source", "/file/test/target"));

        action_group
    }

    fn get_test_queue() -> Vec<ActionGroup> {
        vec![
            get_test_group(),
            get_test_group(),
            get_test_group(),
            get_test_group(),
        ]
    }

    fn write_read_compare_test_data(disk_handler: &DiskHandler) -> Result<()> {
        let applied_actions_in = get_test_queue();
        let undone_actions_in = get_test_queue();

        disk_handler.write(&applied_actions_in, &undone_actions_in)?;

        let (applied_actions_out, undone_actions_out) = disk_handler.read()?;

        assert_eq!(applied_actions_in, applied_actions_out);
        assert_eq!(undone_actions_in, undone_actions_out);

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
