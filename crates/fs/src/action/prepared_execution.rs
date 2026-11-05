use camino::Utf8Path;
use tfmttools_core::action::Action;
use tfmttools_core::history::{
    AttemptDetails, BinaryPatchPair, OperationKind, StoredAction,
};

use super::file_switch::{
    TagSwitch, conflict, identity_if_regular, regular_file_metadata,
    sync_parent,
};
use crate::error::{FsError, FsResult};

pub struct PreparedAction {
    action: StoredAction,
    details: AttemptDetails,
    patches: Option<BinaryPatchPair>,
    effect: Effect,
    attempted: bool,
}
pub(super) enum Effect {
    Tag(TagSwitch),
    Filesystem(Action, OperationKind),
}
impl PreparedAction {
    pub(super) fn new(
        action: StoredAction,
        details: AttemptDetails,
        patches: Option<BinaryPatchPair>,
        effect: Effect,
    ) -> Self {
        Self { action, details, patches, effect, attempted: false }
    }

    #[must_use]
    pub fn action(&self) -> &StoredAction {
        &self.action
    }

    #[must_use]
    pub fn details(&self) -> &AttemptDetails {
        &self.details
    }

    #[must_use]
    pub fn patches(&self) -> Option<&BinaryPatchPair> {
        self.patches.as_ref()
    }

    pub fn retain_artifacts(&mut self) {
        if let Effect::Tag(tag) = &mut self.effect {
            tag.retain();
        }
    }

    pub fn execute(&mut self) -> FsResult<()> {
        if self.attempted {
            return Err(conflict(Utf8Path::new(&self.details.paths[0])));
        }
        self.attempted = true;
        // Preserve backups on any execution error, including direct callers.
        self.retain_artifacts();
        match &mut self.effect {
            Effect::Tag(tag) => tag.execute(),
            Effect::Filesystem(action, kind) => {
                execute_filesystem(action, *kind)
            },
        }
    }

    pub fn confirm(&mut self) -> FsResult<()> {
        match &mut self.effect {
            Effect::Tag(tag) => tag.confirm(),
            Effect::Filesystem(..) => Ok(()),
        }
    }
}
pub fn prepare_action(
    action: &Action,
    direction: OperationKind,
) -> FsResult<PreparedAction> {
    if matches!(action, Action::EditTagValues { .. }) {
        return Err(conflict(action.target()));
    }
    validate_inputs(action, direction)?;
    let mut paths = vec![];
    if let Some(source) = action.source() {
        paths.push(source.to_string());
    }
    paths.push(action.target().to_string());
    let instructions = match action {
        Action::MoveFile { .. } => {
            "Check source and destination to determine whether the file moved."
        },
        Action::CopyFile { .. } => {
            "Check source and destination: a failed copy can leave a partial destination. For undo, the source of the reverse copy is deleted only after the destination is verified. Repair to the applied or not-applied state before resolving history."
        },
        Action::RemoveFile(..) => {
            "Check whether the file was deleted. For copy/delete moves inspect the previously confirmed copy destination too."
        },
        _ => {
            "Check whether the directory action happened. Nonempty directory removal is a no-op."
        },
    };
    Ok(PreparedAction::new(
        StoredAction::from(action),
        AttemptDetails { paths, instructions: instructions.into() },
        None,
        Effect::Filesystem(action.clone(), direction),
    ))
}
fn validate_inputs(action: &Action, kind: OperationKind) -> FsResult<()> {
    let undo = kind == OperationKind::Undo;
    match action {
        Action::MoveFile { source, target }
        | Action::CopyFile { source, target } => {
            let (source, target) =
                if undo { (target, source) } else { (source, target) };
            regular_file_metadata(source)?.ok_or_else(|| conflict(source))?;
            if regular_file_metadata(target)?.is_some() {
                let allowed = match action {
                    Action::MoveFile { .. } => same_case_alias(source, target)?,
                    _ => {
                        undo && identity_if_regular(source)?
                            == identity_if_regular(target)?
                    },
                };
                if !allowed {
                    return Err(conflict(target));
                }
            }
        },
        Action::RemoveFile(path) if !undo => {
            regular_file_metadata(path)?.ok_or_else(|| conflict(path))?;
        },
        _ => {},
    }
    Ok(())
}
fn execute_filesystem(action: &Action, kind: OperationKind) -> FsResult<()> {
    validate_inputs(action, kind)?;
    let undo = kind == OperationKind::Undo;
    match action {
        Action::MoveFile { source, target } => {
            let (source, target) =
                if undo { (target, source) } else { (source, target) };
            fs_err::rename(source, target)?;
            sync_parent(source)?;
            sync_parent(target)?;
        },
        Action::CopyFile { source, target } => {
            let (source, target) =
                if undo { (target, source) } else { (source, target) };
            let identity =
                identity_if_regular(source)?.ok_or_else(|| conflict(source))?;
            let existing = identity_if_regular(target)?;
            if existing.as_ref().is_some_and(|i| !undo || i != &identity) {
                return Err(conflict(target));
            }
            if existing.is_none() {
                // A partial copy stays at the already reported destination.
                // create_new preserves collision safety without hidden artifacts.
                let mut output = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(target)?;
                std::io::copy(&mut std::fs::File::open(source)?, &mut output)?;
                output
                    .set_permissions(fs_err::metadata(source)?.permissions())?;
                output.sync_all()?;
            }
            if identity_if_regular(source)?.as_ref() != Some(&identity)
                || identity_if_regular(target)?.as_ref() != Some(&identity)
            {
                return Err(conflict(target));
            }
            // New copies were synced through the writable handle above.
            // An existing undo destination has not been modified.
            sync_parent(target)?;
            if undo {
                fs_err::remove_file(source)?;
                sync_parent(source)?;
            }
        },
        Action::RemoveFile(path) => {
            if !undo {
                fs_err::remove_file(path)?;
                sync_parent(path)?;
            }
        },
        Action::MakeDir(path) | Action::RemoveDir(path) => {
            let creates = matches!(action, Action::MakeDir(_)) != undo;
            if creates {
                fs_err::create_dir_all(path)?;
            } else if path.exists() && fs_err::read_dir(path)?.next().is_none()
            {
                fs_err::remove_dir(path)?;
            }
            sync_parent(path)?;
        },
        Action::EditTagValues { .. } => unreachable!(),
    }
    Ok(())
}
pub(crate) fn same_case_alias(
    source: &Utf8Path,
    target: &Utf8Path,
) -> FsResult<bool> {
    if !source
        .file_name()
        .zip(target.file_name())
        .is_some_and(|(a, b)| a.eq_ignore_ascii_case(b))
    {
        return Ok(false);
    }
    let parent = |path: &Utf8Path| {
        let parent = path.parent().ok_or_else(|| conflict(path))?;
        let parent = if parent.as_str().is_empty() {
            Utf8Path::new(".")
        } else {
            parent
        };
        Ok::<_, FsError>(fs_err::canonicalize(parent)?)
    };
    let source_parent = parent(source)?;
    if source_parent != parent(target)? {
        return Ok(false);
    }
    #[cfg(any(unix, windows))]
    {
        let source_name = source.file_name().unwrap();
        let target_name = target.file_name().unwrap();
        if source_name != target_name {
            let names = fs_err::read_dir(&source_parent)?
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<std::io::Result<Vec<_>>>()?;
            // Two directory entries can share an inode through hard links.
            // Only a single entry reached with different casing is an alias.
            if names.iter().any(|name| name == source_name)
                && names.iter().any(|name| name == target_name)
            {
                return Ok(false);
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let a = fs_err::metadata(source)?;
        let b = fs_err::metadata(target)?;
        Ok(a.dev() == b.dev() && a.ino() == b.ino())
    }
    #[cfg(windows)]
    {
        let a = winapi_util::file::information(std::fs::File::open(source)?)?;
        let b = winapi_util::file::information(std::fs::File::open(target)?)?;
        Ok(a.volume_serial_number() == b.volume_serial_number()
            && a.file_index() == b.file_index())
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(false)
    }
}
#[cfg_attr(not(unix), allow(clippy::unnecessary_wraps))]
pub(super) fn crosses_devices(
    source: &Utf8Path,
    target: &Utf8Path,
) -> FsResult<bool> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let ancestor = |path: &Utf8Path| -> FsResult<u64> {
            for p in path.ancestors() {
                // A relative path's final ancestor is empty, meaning cwd.
                let p =
                    if p.as_str().is_empty() { Utf8Path::new(".") } else { p };
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

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn case_alias_accepts_bare_relative_paths() {
        let file = tempfile::Builder::new()
            .prefix(".tfmt-case-alias-")
            .tempfile_in(".")
            .unwrap();
        let source =
            Utf8Path::new(file.path().file_name().unwrap().to_str().unwrap());
        let target = source.as_str().to_uppercase();
        assert!(same_case_alias(source, Utf8Path::new(&target)).unwrap());
    }
}
