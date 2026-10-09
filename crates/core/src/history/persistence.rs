use std::fs::{File, TryLockError};
use std::io::Read;

use camino::{Utf8Path, Utf8PathBuf};

use super::{History, HistoryError, LoadHistoryResult, Result, database};

impl History {
    fn ensure_locked(&mut self, path: &Utf8Path) -> Result<()> {
        if self.lock_file.is_some() {
            return Ok(());
        }

        let lock_path = Utf8PathBuf::from(format!("{path}.lock"));
        let acquire = || -> std::io::Result<File> {
            if let Some(parent) = lock_path.parent()
                && !parent.as_str().is_empty()
            {
                fs_err::create_dir_all(parent)?;
            }
            File::options()
                .write(true)
                .create(true)
                .truncate(false)
                .open(&lock_path)
        };
        let file = acquire().map_err(|source| {
            HistoryError::LockError { path: lock_path.clone(), source }
        })?;

        match file.try_lock() {
            Ok(()) => {
                self.lock_file = Some(file);
                Ok(())
            },
            Err(TryLockError::WouldBlock) => {
                Err(HistoryError::Locked(self.path.clone()))
            },
            Err(TryLockError::Error(source)) => {
                Err(HistoryError::LockError { path: lock_path, source })
            },
        }
    }

    pub(super) fn lock_history(&mut self) -> Result<()> {
        let path = resolve_history_path(&self.path)?;
        self.ensure_locked(&path)
    }

    pub fn load(&mut self) -> Result<LoadHistoryResult> {
        let path = resolve_history_path(&self.path)?;
        if !self.read_only {
            self.ensure_locked(&path)?;
        }
        if !path.exists() {
            return Ok(LoadHistoryResult::New);
        }
        inspect_header(&path)?;
        let connection = database::open(&path, self.read_only, false)?;
        database::read_records(&connection, false)?;
        super::journal::validate_patches(&connection)?;
        let records = database::read_records(&connection, true)?;
        self.connection = Some(connection);
        self.records = records;
        Ok(LoadHistoryResult::Loaded)
    }

    pub fn open_read_only(path: Utf8PathBuf) -> Result<Self> {
        let mut history = Self::new(path);
        history.read_only = true;
        history.load()?;
        Ok(history)
    }

    pub fn prepare_save(&self) -> Result<()> {
        if self.read_only {
            return Err(HistoryError::SaveError("Read-only history".into()));
        }
        let path = resolve_history_path(&self.path)?;
        if path.exists() {
            inspect_header(&path)?;
        }
        Ok(())
    }

    pub(super) fn ensure_connection(&mut self) -> Result<()> {
        if self.read_only {
            return Err(HistoryError::SaveError("Read-only history".into()));
        }
        self.lock_history()?;
        if self.connection.is_none() {
            let path = resolve_history_path(&self.path)?;
            let new = !path.exists();
            if !new {
                inspect_header(&path)?;
            }
            fs_err::create_dir_all(parent(&path))
                .map_err(|e| HistoryError::SaveError(e.to_string()))?;
            self.connection = Some(database::open(&path, false, new)?);
        }
        Ok(())
    }

    pub fn save(&mut self) -> Result<()> {
        self.prepare_save()?;
        self.ensure_connection()?;
        if !self.pending_operations()?.is_empty() {
            return Err(HistoryError::SaveError(
                "History has pending recovery work".into(),
            ));
        }
        database::save_records(self.connection.as_mut().unwrap(), &self.records)
    }
}

fn inspect_header(path: &Utf8Path) -> Result<()> {
    if !path.is_file() {
        return Err(HistoryError::LoadError(format!(
            "{path} exists but is not a file"
        )));
    }
    let mut file = fs_err::File::open(path)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    let mut header = [0; 16];
    let read = file
        .read(&mut header)
        .map_err(|e| HistoryError::LoadError(e.to_string()))?;
    if read != 16 || &header != b"SQLite format 3\0" {
        return Err(HistoryError::LoadError(format!(
            "{path}: unsupported history format; JSON histories are not imported. Move the old history aside explicitly to start a new history."
        )));
    }
    Ok(())
}

impl Drop for History {
    fn drop(&mut self) {
        if !self.read_only
            && let Some(connection) = &self.connection
        {
            if rusqlite::version_number() < 3_046_000 {
                let _ = connection.execute_batch("PRAGMA analysis_limit=400");
            }
            let _ = connection.execute_batch("PRAGMA optimize");
        }
    }
}

fn parent(path: &Utf8Path) -> &Utf8Path {
    path.parent()
        .filter(|parent| !parent.as_str().is_empty())
        .unwrap_or_else(|| Utf8Path::new("."))
}

// Follow the final-component link chain, including relative and dangling
// targets, so atomic replacement updates the history file rather than its link.
pub(super) fn resolve_history_path(path: &Utf8Path) -> Result<Utf8PathBuf> {
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
