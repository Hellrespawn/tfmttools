use std::fs::File;
use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use tempfile::Builder;
use tfmttools_core::action::TagValueChange;
use tfmttools_core::history::{
    AttemptDetails, BinaryPatchPair, ByteIdentity, HistoryMode, StoredAction,
};

use super::binary_patch::{apply_patch, byte_identity, create_patch_pair};
use super::prepared_execution::{Effect, PreparedAction};
use super::tag_edit::write_tag_candidate;
use crate::error::{FsError, FsResult};

pub(super) struct TagSwitch {
    path: Utf8PathBuf,
    resolved: Utf8PathBuf,
    candidate: tempfile::NamedTempFile,
    retained: tempfile::NamedTempFile,
    before: ByteIdentity,
    after: ByteIdentity,
}
impl TagSwitch {
    pub(super) fn execute(&mut self) -> FsResult<()> {
        if resolve_audio_path(&self.path)? != self.resolved {
            return Err(conflict(&self.path));
        }
        single_link(&self.resolved)?;
        if identity_if_regular(&self.resolved)?.as_ref() != Some(&self.before)
            || identity_if_regular(
                Utf8Path::from_path(self.candidate.path()).unwrap(),
            )?
            .as_ref()
                != Some(&self.after)
            || identity_if_regular(
                Utf8Path::from_path(self.retained.path()).unwrap(),
            )?
            .is_none_or(|i| i.length != 0)
        {
            return Err(conflict(&self.resolved));
        }
        fs_err::rename(&self.resolved, self.retained.path())?;
        sync_parent(&self.resolved)?;
        // Failure leaves the original in the reported backup. No automatic restore.
        fs_err::rename(self.candidate.path(), &self.resolved)?;
        sync_parent(&self.resolved)?;
        if identity_if_regular(&self.resolved)?.as_ref() != Some(&self.after) {
            return Err(conflict(&self.resolved));
        }
        Ok(())
    }

    pub(super) fn confirm(&mut self) -> FsResult<()> {
        let backup = Utf8Path::from_path(self.retained.path()).unwrap();
        if identity_if_regular(backup)?.as_ref() != Some(&self.before) {
            return Err(conflict(backup));
        }
        fs_err::remove_file(backup)?;
        sync_parent(&self.resolved)
    }

    pub(super) fn retain(&mut self) {
        self.candidate.disable_cleanup(true);
        self.retained.disable_cleanup(true);
    }
}
pub fn prepare_tag_edit(
    path: &Utf8Path,
    changes: &[TagValueChange],
) -> FsResult<PreparedAction> {
    let resolved = resolve_audio_path(path)?;
    single_link(&resolved)?;
    let before = fs_err::read(&resolved)?;
    let mut candidate = Builder::new()
        .prefix(".tfmt-candidate-")
        .suffix(&format!(".{}", resolved.extension().unwrap_or("audio")))
        .tempfile_in(resolved.parent().unwrap())?;
    candidate.write_all(&before)?;
    write_tag_candidate(
        Utf8Path::from_path(candidate.path()).ok_or_else(|| {
            FsError::Execution("Non-UTF-8 candidate path".into())
        })?,
        changes,
    )?;
    let after = fs_err::read(candidate.path())?;
    let pair = create_patch_pair(&before, &after)?;
    fs_err::set_permissions(
        candidate.path(),
        fs_err::metadata(&resolved)?.permissions(),
    )?;
    candidate.as_file().sync_all()?;
    let action = StoredAction::EditTagValues {
        path: path.to_string(),
        changes: changes
            .iter()
            .map(tfmttools_core::history::StoredTagValueChange::from)
            .collect(),
    };
    finish_preparation(
        path,
        &resolved,
        candidate,
        action,
        pair,
        HistoryMode::Redo,
    )
}

pub fn prepare_tag_replay(
    action: &StoredAction,
    pair: &BinaryPatchPair,
    direction: HistoryMode,
) -> FsResult<PreparedAction> {
    let StoredAction::EditTagValues { path, .. } = action else {
        return Err(FsError::Execution("Expected a recorded tag edit".into()));
    };
    let path = Utf8Path::new(path);
    let resolved = resolve_audio_path(path)?;
    single_link(&resolved)?;
    let bytes = fs_err::read(&resolved)?;
    let output = apply_patch(&bytes, pair, direction)?;
    let mut candidate = Builder::new()
        .prefix(".tfmt-candidate-")
        .suffix(&format!(".{}", resolved.extension().unwrap_or("audio")))
        .tempfile_in(resolved.parent().unwrap())?;
    candidate.write_all(&output)?;
    fs_err::set_permissions(
        candidate.path(),
        fs_err::metadata(&resolved)?.permissions(),
    )?;
    candidate.as_file().sync_all()?;

    finish_preparation(
        path,
        &resolved,
        candidate,
        action.clone(),
        pair.clone(),
        direction,
    )
}

fn finish_preparation(
    path: &Utf8Path,
    resolved: &Utf8Path,
    candidate: tempfile::NamedTempFile,
    action: StoredAction,
    pair: BinaryPatchPair,
    direction: HistoryMode,
) -> FsResult<PreparedAction> {
    let retained = Builder::new()
        .prefix(".tfmt-original-")
        .tempfile_in(resolved.parent().unwrap())?;
    sync_parent(resolved)?;
    let details = AttemptDetails {
        paths: vec![path.to_string(),resolved.to_string(),candidate.path().to_string_lossy().into_owned(),retained.path().to_string_lossy().into_owned()],
        instructions: "Inspect the original path, candidate, and retained backup. Restore the original for not-applied, or finish installing the candidate for applied. Resolve history explicitly, then remove unneeded artifacts manually.".into(),
    };
    let (before, after) = match direction {
        HistoryMode::Undo => (pair.after.clone(), pair.before.clone()),
        HistoryMode::Redo => (pair.before.clone(), pair.after.clone()),
    };
    Ok(PreparedAction::new(
        action,
        details,
        Some(pair),
        Effect::Tag(TagSwitch {
            path: path.to_owned(),
            resolved: resolved.to_owned(),
            candidate,
            retained,
            before,
            after,
        }),
    ))
}
pub(super) fn resolve_audio_path(path: &Utf8Path) -> FsResult<Utf8PathBuf> {
    let mut current = if path.is_absolute() {
        path.to_owned()
    } else {
        Utf8PathBuf::try_from(std::env::current_dir()?)?.join(path)
    };
    for _ in 0..40 {
        let parent = current.parent().ok_or_else(|| conflict(&current))?;
        let canonical_parent =
            Utf8PathBuf::try_from(fs_err::canonicalize(parent)?)?;
        current = canonical_parent
            .join(current.file_name().ok_or_else(|| conflict(&current))?);
        match fs_err::symlink_metadata(&current) {
            Ok(meta) if meta.file_type().is_symlink() => {
                let target =
                    Utf8PathBuf::try_from(fs_err::read_link(&current)?)?;
                current = if target.is_absolute() {
                    target
                } else {
                    canonical_parent.join(target)
                };
            },
            Ok(_) => return Ok(current),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(current);
            },
            Err(e) => return Err(e.into()),
        }
    }
    Err(FsError::Execution("Too many audio symlink levels".into()))
}

pub(super) fn identity_if_regular(
    path: &Utf8Path,
) -> FsResult<Option<ByteIdentity>> {
    match fs_err::symlink_metadata(path) {
        Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => {
            Ok(Some(byte_identity(&fs_err::read(path)?)))
        },
        Ok(_) => Err(conflict(path)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
pub(super) fn single_link(path: &Utf8Path) -> FsResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if fs_err::metadata(path)?.nlink() != 1 {
            return Err(FsError::Execution(format!(
                "Cannot replace hard-linked audio file: {path}"
            )));
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(FsError::Execution(
            "Audio replacement requires a supported hard-link count check"
                .into(),
        ))
    }
}
pub(super) fn sync_parent(path: &Utf8Path) -> FsResult<()> {
    #[cfg(unix)]
    {
        File::open(path.parent().ok_or_else(|| conflict(path))?)?.sync_all()?;
    }
    Ok(())
}

pub(super) fn conflict(path: &Utf8Path) -> FsError {
    FsError::Execution(format!(
        "Unexpected file state at {path}; inspect the reported paths manually"
    ))
}
