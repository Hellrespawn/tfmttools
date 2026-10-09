use std::io::Write;

use camino::{Utf8Path, Utf8PathBuf};
use tempfile::Builder;
use tfmttools_core::action::Action;
use tfmttools_core::history::{
    OperationKind, PreparedAction, RecoveryDescriptor, StoredAction,
};

use super::file_switch::{conflict, identity_if_regular, sync_parent};
use crate::error::{FsError, FsResult};

pub fn prepare_action(
    action: &Action,
    direction: OperationKind,
) -> FsResult<PreparedAction> {
    let undo = direction == OperationKind::Undo;
    let recovery = match action {
        Action::MoveFile { source, target } => {
            let (source, target) =
                if undo { (target, source) } else { (source, target) };
            let identity =
                identity_if_regular(source)?.ok_or_else(|| conflict(source))?;
            if identity_if_regular(target)?.is_some()
                && !same_case_alias(source, target)?
            {
                return Err(conflict(target));
            }
            RecoveryDescriptor::Move {
                source: source.to_string(),
                target: target.to_string(),
                identity,
            }
        },
        Action::CopyFile { source, target } => {
            let (source, target) =
                if undo { (target, source) } else { (source, target) };
            let identity =
                identity_if_regular(source)?.ok_or_else(|| conflict(source))?;
            if let Some(existing) = identity_if_regular(target)?
                && (!undo || existing != identity)
            {
                return Err(conflict(target));
            }
            let mut candidate =
                Builder::new().prefix(".tfmt-copy-").tempfile_in(
                    target.parent().ok_or_else(|| conflict(target))?,
                )?;
            candidate.write_all(&fs_err::read(source)?)?;
            fs_err::set_permissions(
                candidate.path(),
                fs_err::metadata(source)?.permissions(),
            )?;
            candidate.as_file().sync_all()?;
            let candidate_path =
                Utf8PathBuf::try_from(candidate.path().to_owned())?;
            if identity_if_regular(&candidate_path)?.as_ref() != Some(&identity)
            {
                return Err(conflict(source));
            }
            sync_parent(target)?;
            let _ = candidate.keep().map_err(|e| e.error)?;
            RecoveryDescriptor::Copy {
                source: source.to_string(),
                target: target.to_string(),
                identity,
                remove_source: undo,
                candidate: candidate_path.to_string(),
            }
        },
        Action::RemoveFile(path) => {
            if undo {
                RecoveryDescriptor::Noop
            } else {
                RecoveryDescriptor::Remove {
                    path: path.to_string(),
                    identity: identity_if_regular(path)?
                        .ok_or_else(|| conflict(path))?,
                }
            }
        },
        Action::MakeDir(path) | Action::RemoveDir(path) => {
            let before_exists = directory_exists(path)?;
            let desired = matches!(action, Action::MakeDir(_)) != undo;
            let after_exists = if !desired
                && before_exists
                && fs_err::read_dir(path)?.next().is_some()
            {
                true
            } else {
                desired
            };
            RecoveryDescriptor::Directory {
                path: path.to_string(),
                before_exists,
                after_exists,
            }
        },
        Action::EditTagValues { .. } => {
            return Err(FsError::Recovery(
                "Tag edits require candidate preparation and recorded patches"
                    .into(),
            ));
        },
    };
    Ok(PreparedAction {
        action: StoredAction::from(action),
        recovery,
        patches: None,
    })
}

pub(super) fn recover_filesystem(entry: &PreparedAction) -> FsResult<()> {
    match &entry.recovery {
        RecoveryDescriptor::Move { source, target, identity } => {
            let source = Utf8Path::new(source);
            let target = Utf8Path::new(target);
            let from = identity_if_regular(source)?;
            let to = identity_if_regular(target)?;
            if from.is_none() && to.as_ref() == Some(identity) {
                return Ok(());
            }
            if from.as_ref() != Some(identity) {
                return Err(conflict(source));
            }
            if to.is_some() && !same_case_alias(source, target)? {
                return Err(conflict(target));
            }
            fs_err::rename(source, target)?;
            sync_parent(source)?;
            sync_parent(target)?;
        },
        RecoveryDescriptor::Copy {
            source,
            target,
            identity,
            remove_source,
            candidate,
        } => {
            let source = Utf8Path::new(source);
            let target = Utf8Path::new(target);
            let candidate = Utf8Path::new(candidate);
            validate_copy_path(target, candidate)?;
            let from = identity_if_regular(source)?;
            let to = identity_if_regular(target)?;
            if to.as_ref() == Some(identity) {
                if from.as_ref().is_some_and(|i| i != identity) {
                    return Err(conflict(source));
                }
                if !remove_source && from.is_none() {
                    return Err(conflict(source));
                }
            } else {
                if to.is_some() {
                    return Err(conflict(target));
                }
                if from.as_ref() != Some(identity)
                    || identity_if_regular(candidate)?.as_ref()
                        != Some(identity)
                {
                    return Err(conflict(source));
                }
                fs_err::rename(candidate, target)?;
                sync_parent(target)?;
            }
            if *remove_source && identity_if_regular(source)?.is_some() {
                fs_err::remove_file(source)?;
                sync_parent(source)?;
            }
        },
        RecoveryDescriptor::Remove { path, identity } => {
            let path = Utf8Path::new(path);
            if let Some(actual) = identity_if_regular(path)? {
                if &actual != identity {
                    return Err(conflict(path));
                }
                fs_err::remove_file(path)?;
                sync_parent(path)?;
            }
        },
        RecoveryDescriptor::Directory { path, before_exists, after_exists } => {
            let path = Utf8Path::new(path);
            let actual = directory_exists(path)?;
            if actual == *after_exists {
                return Ok(());
            }
            if actual != *before_exists {
                return Err(conflict(path));
            }
            if *after_exists {
                fs_err::create_dir(path)?;
            } else {
                fs_err::remove_dir(path)?;
            }
            sync_parent(path)?;
        },
        RecoveryDescriptor::Noop => {},
        RecoveryDescriptor::FileSwitch { .. } => {
            unreachable!("handled by file switch")
        },
    }
    Ok(())
}

pub(super) fn cleanup_copy(entry: &PreparedAction) -> FsResult<()> {
    if let RecoveryDescriptor::Copy { target, candidate, identity, .. } =
        &entry.recovery
    {
        let target = Utf8Path::new(target);
        let candidate = Utf8Path::new(candidate);
        validate_copy_path(target, candidate)?;
        if let Some(actual) = identity_if_regular(candidate)? {
            if &actual != identity {
                return Err(conflict(candidate));
            }
            fs_err::remove_file(candidate)?;
            sync_parent(candidate)?;
        }
    }
    Ok(())
}
fn validate_copy_path(target: &Utf8Path, candidate: &Utf8Path) -> FsResult<()> {
    if target.parent() != candidate.parent()
        || !candidate.file_name().is_some_and(|n| n.starts_with(".tfmt-copy-"))
    {
        return Err(conflict(candidate));
    }
    Ok(())
}
fn directory_exists(path: &Utf8Path) -> FsResult<bool> {
    match fs_err::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => Ok(true),
        Ok(_) => Err(conflict(path)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}
fn same_case_alias(source: &Utf8Path, target: &Utf8Path) -> FsResult<bool> {
    if source.parent() != target.parent()
        || !source
            .file_name()
            .zip(target.file_name())
            .is_some_and(|(a, b)| a.eq_ignore_ascii_case(b))
    {
        return Ok(false);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let a = fs_err::metadata(source)?;
        let b = fs_err::metadata(target)?;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    #[cfg(not(unix))]
    {
        Ok(false)
    }
}
pub(super) fn crosses_devices(
    source: &Utf8Path,
    target: &Utf8Path,
) -> FsResult<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let ancestor = |path: &Utf8Path| -> FsResult<u64> {
            for p in path.ancestors() {
                if let Ok(meta) = fs_err::metadata(p) {
                    return Ok(meta.dev());
                }
            }
            Err(conflict(path))
        };
        Ok(ancestor(source)?
            != ancestor(target.parent().ok_or_else(|| conflict(target))?)?)
    }
    #[cfg(not(unix))]
    {
        Ok(source.components().next() != target.components().next())
    }
}
