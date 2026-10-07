use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use tempfile::NamedTempFile;
use tracing::debug;

use super::{History, HistoryError, LoadHistoryResult, Result, StoredHistory};

impl History {
    pub fn load(&mut self) -> Result<LoadHistoryResult> {
        let path = resolve_history_path(&self.path)
            .map_err(|error| HistoryError::LoadError(error.to_string()))?;
        if path.exists() && !path.is_file() {
            return Err(HistoryError::LoadError(format!(
                "{path} exists but is not a file."
            )));
        }
        match fs_err::read(&path) {
            Ok(bytes) => {
                let (stored, migrated) =
                    decode_history(&bytes).map_err(|error| {
                        HistoryError::LoadError(format!(
                            "{}: {error}",
                            self.path
                        ))
                    })?;
                self.records = stored.records;
                self.upgrade_source = migrated.then_some(bytes);
                debug!("Loaded history from {}", self.path);
                Ok(LoadHistoryResult::Loaded)
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                debug!("Loading empty history");
                Ok(LoadHistoryResult::New)
            },
            Err(error) => {
                Err(HistoryError::LoadError(format!("{}: {error}", self.path)))
            },
        }
    }

    /// Check predictable upgrade-backup failures before applying actions.
    /// This is read-only; save still creates the backup exclusively.
    pub fn prepare_save(&self) -> Result<()> {
        let path = resolve_history_path(&self.path)?;
        if let Some(source) = &self.upgrade_source {
            check_upgrade_backup(&path, source)?;
        }
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        // Prepare and validate everything before creating any output file.
        let stored = StoredHistory::current(self.records.clone());
        stored
            .validate()
            .map_err(|error| HistoryError::SaveError(error.to_string()))?;
        let bytes = serde_json::to_vec_pretty(&stored).map_err(|error| {
            HistoryError::SaveError(format!(
                "Unable to serialize history: {error}"
            ))
        })?;
        let path = resolve_history_path(&self.path)?;
        if path.exists() && !path.is_file() {
            let tmp_dir: Utf8PathBuf =
                std::env::temp_dir().try_into().map_err(|_| {
                    HistoryError::SaveError(
                        "Temporary directory is not valid UTF-8.".to_owned(),
                    )
                })?;
            let name = self.path.file_name().ok_or_else(|| {
                HistoryError::SaveError(format!(
                    "History path has no file name: {}",
                    self.path
                ))
            })?;
            let recovery = tmp_dir.join(name);
            write_atomically(&recovery, &bytes)?;
            return Err(HistoryError::SaveErrorWithBackup(
                format!("{} exists but is not a file.", self.path),
                recovery,
            ));
        }
        create_parent(&path)?;
        if let Some(source) = &self.upgrade_source {
            preserve_upgrade_backup(&path, source)?;
        }
        write_atomically(&path, &bytes)?;
        self.upgrade_source = None;
        Ok(())
    }
}

fn decode_history(bytes: &[u8]) -> Result<(StoredHistory, bool)> {
    let value = serde_json::from_slice(bytes).map_err(|error| {
        HistoryError::LoadError(format!("Invalid history JSON: {error}"))
    })?;
    super::migration::upgrade_document(value)
}

fn parent(path: &Utf8Path) -> &Utf8Path {
    path.parent()
        .filter(|parent| !parent.as_str().is_empty())
        .unwrap_or_else(|| Utf8Path::new("."))
}

fn create_parent(path: &Utf8Path) -> Result<()> {
    fs_err::create_dir_all(parent(path)).map_err(|error| {
        HistoryError::SaveError(format!(
            "Unable to create directory {}: {error}",
            parent(path)
        ))
    })
}

// Follow the final-component link chain, including relative and dangling
// targets, so atomic replacement updates the history file rather than its link.
fn resolve_history_path(path: &Utf8Path) -> Result<Utf8PathBuf> {
    let mut destination = path.to_owned();
    for _ in 0..40 {
        match fs_err::symlink_metadata(&destination) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let target = fs_err::read_link(&destination).map_err(|error| {
                    HistoryError::SaveError(format!("Unable to resolve history link {destination}: {error}"))
                })?;
                let target = Utf8PathBuf::from_path_buf(target).map_err(|target| {
                    HistoryError::SaveError(format!("History link {destination} has a non-UTF-8 target: {}", target.display()))
                })?;
                destination = if target.is_absolute() {
                    target
                } else {
                    parent(&destination).join(target)
                };
            },
            Ok(_) => return Ok(destination),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(destination);
            },
            Err(error) => {
                return Err(HistoryError::SaveError(format!(
                    "Unable to inspect history {destination}: {error}"
                )));
            },
        }
    }
    Err(HistoryError::SaveError(format!(
        "Too many history symlink levels: {path}"
    )))
}

fn check_upgrade_backup(path: &Utf8Path, bytes: &[u8]) -> Result<()> {
    let backup = Utf8PathBuf::from(format!("{path}.v0.bak"));
    match fs_err::symlink_metadata(&backup) {
        Ok(metadata) if !metadata.file_type().is_file() => {
            Err(HistoryError::SaveError(format!(
                "Upgrade backup {backup} exists but is not a regular file; refusing to reuse it"
            )))
        },
        Ok(_) => {
            let existing = fs_err::read(&backup).map_err(|error| {
                HistoryError::SaveError(format!(
                    "Unable to read upgrade backup {backup}: {error}"
                ))
            })?;
            if existing == bytes {
                Ok(())
            } else {
                Err(HistoryError::SaveError(format!(
                    "Upgrade backup {backup} differs from the original history; refusing to overwrite"
                )))
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(HistoryError::SaveError(format!(
                "Unable to inspect upgrade backup {backup}: {error}"
            )))
        },
    }
}

fn preserve_upgrade_backup(path: &Utf8Path, bytes: &[u8]) -> Result<()> {
    let backup = Utf8PathBuf::from(format!("{path}.v0.bak"));
    match fs_err::OpenOptions::new().write(true).create_new(true).open(&backup)
    {
        Ok(mut file) => {
            if let Err(error) = file.write_all(bytes) {
                drop(file);
                let cleanup = fs_err::remove_file(&backup);
                return Err(HistoryError::SaveError(format!(
                    "Unable to write upgrade backup {backup}: {error}; partial backup cleanup: {cleanup:?}"
                )));
            }
            Ok(())
        },
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            check_upgrade_backup(path, bytes)
        },
        Err(error) => {
            Err(HistoryError::SaveError(format!(
                "Unable to create upgrade backup {backup}: {error}"
            )))
        },
    }
}

fn write_atomically(path: &Utf8Path, bytes: &[u8]) -> Result<()> {
    create_parent(path)?;
    let mut temporary =
        NamedTempFile::new_in(parent(path)).map_err(|error| {
            HistoryError::SaveError(format!(
                "Unable to create temporary history for {path}: {error}"
            ))
        })?;
    temporary.write_all(bytes).map_err(|error| {
        HistoryError::SaveError(format!(
            "Unable to write temporary history for {path}: {error}"
        ))
    })?;
    #[cfg(test)]
    tests::check_replacement()?;
    temporary.persist(path).map_err(|error| {
        HistoryError::SaveError(format!(
            "Unable to replace history {path}: {error}"
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use tempfile::TempDir;

    use super::*;

    thread_local! {
        static FAIL_REPLACEMENT: Cell<bool> = const { Cell::new(false) };
    }

    pub(super) fn check_replacement() -> Result<()> {
        if FAIL_REPLACEMENT.replace(false) {
            Err(HistoryError::SaveError("Injected replacement failure".into()))
        } else {
            Ok(())
        }
    }

    #[test]
    fn failed_replacement_preserves_source_backup_and_retry_state() {
        let directory = TempDir::new().unwrap();
        let path =
            Utf8PathBuf::from_path_buf(directory.path().join("history.json"))
                .unwrap();
        let source = include_bytes!(
            "../../tests/fixtures/history/v0-pre-canonical-tags.json"
        );
        fs_err::write(&path, source).unwrap();
        let mut history = History::new(path.clone());
        history.load().unwrap();
        FAIL_REPLACEMENT.set(true);
        assert!(history.save().is_err());
        assert_eq!(fs_err::read(&path).unwrap(), source);
        assert_eq!(fs_err::read(format!("{path}.v0.bak")).unwrap(), source);
        assert!(history.upgrade_source.is_some());
        assert_eq!(fs_err::read_dir(directory.path()).unwrap().count(), 2);
        history.save().unwrap();
        assert!(history.upgrade_source.is_none());
        assert!(decode_history(&fs_err::read(path).unwrap()).is_ok());
    }

    #[test]
    fn atomic_replacement_failure_cleans_temporary_file() {
        let directory = TempDir::new().unwrap();
        let path =
            Utf8PathBuf::from_path_buf(directory.path().join("destination"))
                .unwrap();
        fs_err::create_dir(&path).unwrap();
        fs_err::write(path.join("sentinel"), b"original").unwrap();
        assert!(write_atomically(&path, b"replacement").is_err());
        assert_eq!(fs_err::read(path.join("sentinel")).unwrap(), b"original");
        assert_eq!(fs_err::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn relative_filename_has_current_directory_parent() {
        assert_eq!(parent(Utf8Path::new("history.json")), Utf8Path::new("."));
    }
}
