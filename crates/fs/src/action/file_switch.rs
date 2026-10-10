use std::fs::{File, OpenOptions};
use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use tempfile::Builder;
use tfmttools_core::action::TagValueChange;
use tfmttools_core::history::{
    BinaryPatchPair, ByteIdentity, HistoryMode, PreparedAction,
    RecoveryDescriptor, StoredAction,
};

use super::binary_patch::{apply_patch, byte_identity, create_patch_pair};
use super::tag_edit::write_tag_candidate;
use crate::error::{FsError, FsResult};

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
            FsError::Recovery("Non-UTF-8 candidate path".into())
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
        pair.clone(),
        pair.before,
        pair.after,
    )
}

pub fn prepare_tag_replay(
    action: &StoredAction,
    pair: &BinaryPatchPair,
    direction: HistoryMode,
) -> FsResult<PreparedAction> {
    let StoredAction::EditTagValues { path, .. } = action else {
        return Err(FsError::Recovery("Expected a recorded tag edit".into()));
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
    let (before, after) = match direction {
        HistoryMode::Undo => (pair.after.clone(), pair.before.clone()),
        HistoryMode::Redo => (pair.before.clone(), pair.after.clone()),
    };
    finish_preparation(
        path,
        &resolved,
        candidate,
        action.clone(),
        pair.clone(),
        before,
        after,
    )
}

fn finish_preparation(
    path: &Utf8Path,
    resolved: &Utf8Path,
    candidate: tempfile::NamedTempFile,
    action: StoredAction,
    pair: BinaryPatchPair,
    before: ByteIdentity,
    after: ByteIdentity,
) -> FsResult<PreparedAction> {
    let retained = Builder::new()
        .prefix(".tfmt-original-")
        .tempfile_in(resolved.parent().unwrap())?;
    let candidate_path = Utf8PathBuf::try_from(candidate.path().to_owned())?;
    let retained_path = Utf8PathBuf::try_from(retained.path().to_owned())?;
    // Sync directory entries before the coordinator can commit their paths.
    sync_parent(resolved)?;
    let _ = candidate.keep().map_err(|e| e.error)?;
    if let Err(error) = retained.keep() {
        let _ = fs_err::remove_file(&candidate_path);
        return Err(error.error.into());
    }
    Ok(PreparedAction {
        action,
        recovery: RecoveryDescriptor::FileSwitch {
            path: path.to_string(),
            resolved: resolved.to_string(),
            candidate: candidate_path.to_string(),
            retained: retained_path.to_string(),
            before,
            after,
        },
        patches: Some(pair),
    })
}

pub fn install_prepared(entry: &PreparedAction) -> FsResult<()> {
    recover_prepared(entry)
}
pub fn recover_prepared(entry: &PreparedAction) -> FsResult<()> {
    let RecoveryDescriptor::FileSwitch {
        path,
        resolved,
        candidate,
        retained,
        before,
        after,
    } = &entry.recovery
    else {
        return super::recorded_execution::recover_filesystem(entry);
    };
    let resolved = Utf8Path::new(resolved);
    let candidate = Utf8Path::new(candidate);
    let retained = Utf8Path::new(retained);
    validate_paths(Utf8Path::new(path), resolved, candidate, retained)?;
    let current = identity_if_regular(resolved)?;
    let original = identity_if_regular(retained)?;
    let prepared = identity_if_regular(candidate)?;
    if current.as_ref() == Some(after)
        && !(before == after
            && original.as_ref().is_some_and(|identity| identity.length == 0)
            && prepared.as_ref() == Some(after))
    {
        if original.as_ref().is_some_and(|identity| identity != before) {
            return Err(conflict(retained));
        }
        if prepared.as_ref().is_some_and(|identity| identity != after) {
            return Err(conflict(candidate));
        }
        return sync_parent(resolved);
    }
    if prepared.as_ref() != Some(after) {
        return Err(conflict(candidate));
    }
    if current.as_ref() == Some(before) {
        single_link(resolved)?;
        if let Some(identity) = original {
            if identity.length != 0 {
                return Err(conflict(retained));
            }
        } else {
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(retained)?
                .sync_all()?;
        }
        // Recheck immediately before retention, including the reserved slot.
        if identity_if_regular(resolved)?.as_ref() != Some(before)
            || identity_if_regular(retained)?.is_none_or(|i| i.length != 0)
        {
            return Err(conflict(resolved));
        }
        fs_err::rename(resolved, retained)?;
        sync_parent(resolved)?;
    } else if current.is_some() || original.as_ref() != Some(before) {
        return Err(conflict(resolved));
    }
    if let Err(error) = fs_err::rename(candidate, resolved) {
        if fs_err::symlink_metadata(resolved).is_err()
            && identity_if_regular(retained)?.as_ref() == Some(before)
        {
            fs_err::rename(retained, resolved)?;
            sync_parent(resolved)?;
        }
        return Err(error.into());
    }
    sync_parent(resolved)?;
    if identity_if_regular(resolved)?.as_ref() != Some(after) {
        return Err(conflict(resolved));
    }
    Ok(())
}

pub fn cleanup_prepared(entry: &PreparedAction) -> FsResult<()> {
    let RecoveryDescriptor::FileSwitch {
        path,
        resolved,
        candidate,
        retained,
        before,
        after,
    } = &entry.recovery
    else {
        return super::recorded_execution::cleanup_copy(entry);
    };
    let resolved = Utf8Path::new(resolved);
    let candidate = Utf8Path::new(candidate);
    let retained = Utf8Path::new(retained);
    validate_paths(Utf8Path::new(path), resolved, candidate, retained)?;
    if identity_if_regular(resolved)?.as_ref() != Some(after) {
        return Err(conflict(resolved));
    }
    for (path, expected) in [(retained, before), (candidate, after)] {
        if let Some(identity) = identity_if_regular(path)? {
            if &identity != expected {
                return Err(conflict(path));
            }
            fs_err::remove_file(path)?;
        }
    }
    sync_parent(resolved)
}

/// Remove only preparation artifacts when committing intent failed before effects.
pub fn discard_prepared(entry: &PreparedAction) -> FsResult<()> {
    let RecoveryDescriptor::FileSwitch {
        path,
        resolved,
        candidate,
        retained,
        before,
        after,
    } = &entry.recovery
    else {
        return super::recorded_execution::cleanup_copy(entry);
    };
    let resolved = Utf8Path::new(resolved);
    let candidate = Utf8Path::new(candidate);
    let retained = Utf8Path::new(retained);
    validate_paths(Utf8Path::new(path), resolved, candidate, retained)?;
    if identity_if_regular(resolved)?.as_ref() != Some(before)
        || identity_if_regular(retained)?.is_none_or(|i| i.length != 0)
    {
        return Err(conflict(resolved));
    }
    if identity_if_regular(candidate)?.as_ref() != Some(after) {
        return Err(conflict(candidate));
    }
    fs_err::remove_file(candidate)?;
    fs_err::remove_file(retained)?;
    sync_parent(resolved)
}

fn validate_paths(
    path: &Utf8Path,
    resolved: &Utf8Path,
    candidate: &Utf8Path,
    retained: &Utf8Path,
) -> FsResult<()> {
    if resolve_audio_path(path)? != resolved
        || !resolved.is_absolute()
        || candidate.parent() != resolved.parent()
        || retained.parent() != resolved.parent()
        || !candidate
            .file_name()
            .is_some_and(|n| n.starts_with(".tfmt-candidate-"))
        || !retained
            .file_name()
            .is_some_and(|n| n.starts_with(".tfmt-original-"))
        || candidate == retained
    {
        return Err(FsError::Recovery(
            "Recovery paths or audio symlink changed; refusing switch".into(),
        ));
    }
    Ok(())
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
    Err(FsError::Recovery("Too many audio symlink levels".into()))
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
fn single_link(path: &Utf8Path) -> FsResult<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if fs_err::metadata(path)?.nlink() != 1 {
            return Err(FsError::Recovery(format!(
                "Cannot replace hard-linked audio file: {path}"
            )));
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(FsError::Recovery(
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

// Later actions may remove an artifact's now-empty directory. Sync its
// surviving ancestor to make that absence durable before recording cleanup.
pub(super) fn sync_cleanup_parent(path: &Utf8Path) -> FsResult<()> {
    let mut entry = path.to_owned();
    loop {
        match sync_parent(&entry) {
            Ok(()) => return Ok(()),
            Err(FsError::Io(error))
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                let directory = entry.parent().ok_or_else(|| conflict(path))?;
                match fs_err::symlink_metadata(directory) {
                    Err(missing)
                        if missing.kind() == std::io::ErrorKind::NotFound =>
                    {
                        entry = directory.to_owned();
                    },
                    // Never bypass a dangling link or another inspection error.
                    _ => return Err(error.into()),
                }
            },
            Err(error) => return Err(error),
        }
    }
}
pub(super) fn conflict(path: &Utf8Path) -> FsError {
    FsError::Recovery(format!(
        "Unexpected file state at {path}; retained files require inspection"
    ))
}

/// Delete verified recovery artifacts only after the coordinator has validated
/// the final state of the entire operation and committed finalization.
pub fn cleanup_completed_artifacts(entry: &PreparedAction) -> FsResult<()> {
    if let RecoveryDescriptor::FileSwitch {
        resolved,
        candidate,
        retained,
        before,
        after,
        ..
    } = &entry.recovery
    {
        let resolved = Utf8Path::new(resolved);
        let candidate = Utf8Path::new(candidate);
        let retained = Utf8Path::new(retained);
        if candidate.parent() != resolved.parent()
            || retained.parent() != resolved.parent()
            || !candidate
                .file_name()
                .is_some_and(|n| n.starts_with(".tfmt-candidate-"))
            || !retained
                .file_name()
                .is_some_and(|n| n.starts_with(".tfmt-original-"))
        {
            return Err(conflict(retained));
        }
        for (path, expected) in [(retained, before), (candidate, after)] {
            if let Some(actual) = identity_if_regular(path)? {
                if &actual != expected {
                    return Err(conflict(path));
                }
                fs_err::remove_file(path)?;
            }
        }
        // Retry a directory sync even if an earlier cleanup already removed
        // the artifacts but stopped before recording durable completion.
        sync_cleanup_parent(resolved)
    } else {
        super::recorded_execution::cleanup_copy(entry)
    }
}
