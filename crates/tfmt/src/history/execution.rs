use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use color_eyre::Result;
use color_eyre::eyre::bail;
use tfmttools_core::action::Action;
use tfmttools_core::history::{
    ActionRecordMetadata, ByteIdentity, History, HistoryMode, OperationId,
    OperationKind, PendingOperation, Record, RecoveryDescriptor, StoredAction,
};
use tfmttools_fs::{
    FsHandler, byte_identity, cleanup_completed_artifacts, discard_prepared,
    install_prepared, prepare_action, prepare_tag_edit, prepare_tag_replay,
    recover_prepared,
};

pub(crate) fn recover_pending(
    history: &mut History,
    fs: &FsHandler,
) -> Result<()> {
    if fs.is_dry_run() {
        return Ok(());
    }
    for operation in history.pending_operations()? {
        if operation.plan.is_none() {
            history.cancel_unstarted(operation.id)?;
            continue;
        }
        println!(
            "Recovering interrupted history operation {}...",
            operation.id.0
        );
        run_operation(history, operation.id)?;
    }
    Ok(())
}

pub(crate) fn execute_recorded(
    history: &mut History,
    fs: &FsHandler,
    actions: Vec<Action>,
    metadata: ActionRecordMetadata,
) -> Result<Record> {
    let actions = absolute_actions(actions)?;
    let plan: Vec<_> = actions.iter().map(StoredAction::from).collect();
    if fs.is_dry_run() {
        return Ok(Record::new(plan, metadata));
    }
    recover_pending(history, fs)?;
    let id =
        history.begin_operation(OperationKind::Apply, None, Some(metadata))?;
    if let Err(error) = history.set_operation_plan(id, &plan) {
        history.cancel_unstarted(id)?;
        return Err(error.into());
    }
    run_or_cancel_unstarted(history, id)
}

pub(crate) fn replay_record(
    history: &mut History,
    fs: &FsHandler,
    record: &Record,
    direction: HistoryMode,
) -> Result<Record> {
    if fs.is_dry_run() {
        return Ok(record.clone());
    }
    let kind = match direction {
        HistoryMode::Undo => OperationKind::Undo,
        HistoryMode::Redo => OperationKind::Redo,
    };
    let plan: Vec<_> = match direction {
        HistoryMode::Undo => record.iter().rev().cloned().collect(),
        HistoryMode::Redo => record.iter().cloned().collect(),
    };
    let id = history.begin_operation(kind, record.id(), None)?;
    if let Err(error) = history.set_operation_plan(id, &plan) {
        history.cancel_unstarted(id)?;
        return Err(error.into());
    }
    run_or_cancel_unstarted(history, id)
}

fn run_or_cancel_unstarted(
    history: &mut History,
    id: OperationId,
) -> Result<Record> {
    let result = run_operation(history, id);
    if result.is_err()
        && history
            .pending_operations()?
            .iter()
            .any(|o| o.id == id && o.entries.is_empty() && !o.finalized)
    {
        history.cancel_unstarted(id)?;
    }
    result
}

fn pending(history: &History, id: OperationId) -> Result<PendingOperation> {
    history
        .pending_operations()?
        .into_iter()
        .find(|o| o.id == id)
        .ok_or_else(|| color_eyre::eyre::eyre!("Pending operation disappeared"))
}
fn run_operation(history: &mut History, id: OperationId) -> Result<Record> {
    let mut operation = pending(history, id)?;
    if !operation.finalized {
        for entry in &operation.entries {
            if !entry.completed {
                recover_prepared(&entry.prepared)?;
                history.complete_action(id, entry.position)?;
            }
        }
        operation = pending(history, id)?;
        let plan = operation.plan.as_ref().unwrap();
        for (position, action) in
            plan.iter().enumerate().skip(operation.entries.len())
        {
            // Earlier effects establish the preconditions for dependent actions.
            verify_expected(&expected_states(&pending(history, id)?))?;
            let executable = Action::try_from(action)?;
            let entry = match &executable {
                Action::EditTagValues { path, changes }
                    if operation.kind == OperationKind::Apply =>
                {
                    prepare_tag_edit(path, changes)?
                },
                Action::EditTagValues { .. } => {
                    let action_position =
                        if operation.kind == OperationKind::Undo {
                            plan.len() - 1 - position
                        } else {
                            position
                        };
                    let pair = history
                        .patches(operation.record_id, action_position)?
                        .ok_or_else(|| {
                            color_eyre::eyre::eyre!(
                                "Recorded binary patches missing"
                            )
                        })?;
                    let direction = if operation.kind == OperationKind::Undo {
                        HistoryMode::Undo
                    } else {
                        HistoryMode::Redo
                    };
                    prepare_tag_replay(action, &pair, direction)?
                },
                _ => prepare_action(&executable, operation.kind)?,
            };
            if let Err(error) = history.append_prepared(id, &entry) {
                // A failed commit can be ambiguous: never discard a candidate if
                // the journal may already refer to it.
                if history.pending_operations().is_ok_and(|ops| {
                    ops.iter()
                        .find(|o| o.id == id)
                        .is_some_and(|o| o.entries.len() == position)
                }) {
                    discard_prepared(&entry)?;
                }
                return Err(error.into());
            }
            install_prepared(&entry)?;
            history.complete_action(id, position)?;
        }
    }
    operation = pending(history, id)?;
    verify_expected(&expected_states(&operation))?;
    let record = history.finish_operation(id)?;
    for entry in &operation.entries {
        if !entry.cleaned {
            cleanup_completed_artifacts(&entry.prepared)?;
            history.complete_cleanup(id, entry.position)?;
        }
    }
    Ok(record)
}

#[derive(Clone)]
enum Expected {
    File(Option<ByteIdentity>),
    Directory(bool),
}
fn expected_states(operation: &PendingOperation) -> BTreeMap<String, Expected> {
    let mut expected = BTreeMap::new();
    for entry in operation.entries.iter().filter(|e| e.completed) {
        match &entry.prepared.recovery {
            RecoveryDescriptor::FileSwitch { resolved, after, .. } => {
                expected.insert(
                    resolved.clone(),
                    Expected::File(Some(after.clone())),
                );
            },
            RecoveryDescriptor::Move { source, target, identity } => {
                expected.insert(source.clone(), Expected::File(None));
                expected.insert(
                    target.clone(),
                    Expected::File(Some(identity.clone())),
                );
            },
            RecoveryDescriptor::Copy {
                source,
                target,
                identity,
                remove_source,
                ..
            } => {
                expected.insert(
                    source.clone(),
                    Expected::File(if *remove_source {
                        None
                    } else {
                        Some(identity.clone())
                    }),
                );
                expected.insert(
                    target.clone(),
                    Expected::File(Some(identity.clone())),
                );
            },
            RecoveryDescriptor::Remove { path, .. } => {
                expected.insert(path.clone(), Expected::File(None));
            },
            RecoveryDescriptor::Directory { path, after_exists, .. } => {
                expected
                    .insert(path.clone(), Expected::Directory(*after_exists));
            },
            RecoveryDescriptor::Noop => {},
        }
    }
    expected
}
fn verify_expected(expected: &BTreeMap<String, Expected>) -> Result<()> {
    for (path, state) in expected {
        match state {
            Expected::File(identity) => {
                let actual = match std::fs::symlink_metadata(path) {
                    Ok(meta)
                        if meta.is_file() && !meta.file_type().is_symlink() =>
                    {
                        Some(byte_identity(&std::fs::read(path)?))
                    },
                    Ok(_) => {
                        bail!(
                            "Unexpected file at {path}; operation remains recoverable"
                        )
                    },
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                    Err(e) => return Err(e.into()),
                };
                if &actual != identity {
                    bail!(
                        "File changed during recorded operation at {path}; recovery stopped"
                    );
                }
            },
            Expected::Directory(exists) => {
                if std::path::Path::new(path).is_dir() != *exists {
                    bail!(
                        "Directory changed during recorded operation at {path}"
                    );
                }
            },
        }
    }
    Ok(())
}
fn absolute_actions(actions: Vec<Action>) -> Result<Vec<Action>> {
    let cwd = Utf8PathBuf::try_from(std::env::current_dir()?)?;
    let absolute = |path: Utf8PathBuf| -> Utf8PathBuf {
        if path.is_absolute() { path } else { cwd.join(path) }
    };
    Ok(actions
        .into_iter()
        .map(|action| {
            match action {
                Action::MoveFile { source, target } => {
                    Action::MoveFile {
                        source: absolute(source),
                        target: absolute(target),
                    }
                },
                Action::CopyFile { source, target } => {
                    Action::CopyFile {
                        source: absolute(source),
                        target: absolute(target),
                    }
                },
                Action::RemoveFile(path) => Action::RemoveFile(absolute(path)),
                Action::MakeDir(path) => Action::MakeDir(absolute(path)),
                Action::RemoveDir(path) => Action::RemoveDir(absolute(path)),
                Action::EditTagValues { path, changes } => {
                    Action::EditTagValues { path: absolute(path), changes }
                },
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use tfmttools_core::action::{TagValueChange, TagValueKind};
    use tfmttools_core::history::TemplateMetadata;
    use tfmttools_core::util::FSMode;

    use super::*;
    fn metadata() -> ActionRecordMetadata {
        ActionRecordMetadata::new(
            TemplateMetadata::Validation { value: "test".into() },
            vec![],
            "unit".into(),
        )
    }
    fn audio(dir: &TempDir) -> Utf8PathBuf {
        let path = Utf8PathBuf::try_from(dir.path().join("song.mp3")).unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
            &path,
        )
        .unwrap();
        path
    }
    fn edit(path: &Utf8PathBuf, from: &str, to: &str) -> Action {
        Action::EditTagValues {
            path: path.clone(),
            changes: vec![TagValueChange::new(
                "track_title".into(),
                TagValueKind::Text,
                from.into(),
                to.into(),
            )],
        }
    }
    #[test]
    fn sequential_edits_and_rename_replay_all_bytes_in_order() {
        let dir = TempDir::new().unwrap();
        let path = audio(&dir);
        let target =
            Utf8PathBuf::try_from(dir.path().join("moved.mp3")).unwrap();
        let before = std::fs::read(&path).unwrap();
        let fs = FsHandler::new(FSMode::Default);
        let mut h = History::new(
            Utf8PathBuf::try_from(dir.path().join("tfmt.hist")).unwrap(),
        );
        h.load().unwrap();
        let record = execute_recorded(
            &mut h,
            &fs,
            vec![
                edit(&path, "Nemo", "First"),
                edit(&path, "First", "Second"),
                Action::MoveFile {
                    source: path.clone(),
                    target: target.clone(),
                },
            ],
            metadata(),
        )
        .unwrap();
        let after = std::fs::read(&target).unwrap();
        assert!(!path.exists());
        assert!(h.pending_operations().unwrap().is_empty());
        let record =
            replay_record(&mut h, &fs, &record, HistoryMode::Undo).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!target.exists());
        replay_record(&mut h, &fs, &record, HistoryMode::Redo).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), after);
        assert!(!path.exists());
        assert!(!std::fs::read_dir(dir.path()).unwrap().any(|e| {
            e.unwrap().file_name().to_string_lossy().starts_with(".tfmt-")
        }));
    }
    #[test]
    fn database_failure_after_installation_retains_original_until_recovery() {
        let dir = TempDir::new().unwrap();
        let path = audio(&dir);
        let before = std::fs::read(&path).unwrap();
        let history_path =
            Utf8PathBuf::try_from(dir.path().join("tfmt.hist")).unwrap();
        let mut h = History::new(history_path.clone());
        h.load().unwrap();
        let action = edit(&path, "Nemo", "Changed");
        let id = h
            .begin_operation(OperationKind::Apply, None, Some(metadata()))
            .unwrap();
        h.set_operation_plan(id, &[StoredAction::from(&action)]).unwrap();
        let Action::EditTagValues { changes, .. } = &action else { panic!() };
        let entry = prepare_tag_edit(&path, changes).unwrap();
        h.append_prepared(id, &entry).unwrap();
        install_prepared(&entry).unwrap();
        let RecoveryDescriptor::FileSwitch { retained, .. } = &entry.recovery
        else {
            panic!()
        };
        let c = rusqlite::Connection::open(&history_path).unwrap();
        c.execute_batch("CREATE TRIGGER fail_progress BEFORE UPDATE ON progress BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
        assert!(run_operation(&mut h, id).is_err());
        assert_eq!(std::fs::read(retained).unwrap(), before);
        c.execute_batch("DROP TRIGGER fail_progress").unwrap();
        drop(c);
        drop(h);
        let mut h = History::new(history_path);
        h.load().unwrap();
        recover_pending(&mut h, &FsHandler::new(FSMode::Default)).unwrap();
        assert!(!std::path::Path::new(retained).exists());
        assert!(h.pending_operations().unwrap().is_empty());
    }
}
