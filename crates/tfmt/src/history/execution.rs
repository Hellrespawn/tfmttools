use camino::Utf8PathBuf;
use color_eyre::Result;
use color_eyre::eyre::{WrapErr, bail};
use tfmttools_core::action::Action;
use tfmttools_core::history::{
    ActionRecordMetadata, History, HistoryMode, OperationKind, Record, RunId,
    StoredAction,
};
use tfmttools_fs::{
    FsHandler, PreparedAction, prepare_action, prepare_tag_edit,
    prepare_tag_replay,
};

pub(crate) fn interruption_report(history: &History) -> Result<Option<String>> {
    let Some(a) = history.current_attempt()? else {
        return Ok(history.open_run()?.map(|(_,rid,kind)|format!("Interrupted run {rid} ({kind:?}) has no unconfirmed action. History contains the confirmed actions only. A writable invocation will close it as partial; no remaining actions will run.")));
    };
    let record = history
        .records()
        .iter()
        .find(|r| r.id() == Some(a.record_id))
        .ok_or_else(|| color_eyre::eyre::eyre!("Attempt record missing"))?;
    Ok(Some(format!(
        "Unresolved attempt {}: {:?} record {} (run {}).\nHistory confirms {} actions; {} currently applied.\nCommand: {:?}\nLast attempted action {}: {:?}\nPaths:\n  {}\n{}\nNo remaining actions will run. Repair the files manually, then run:\n  tfmt resolve-history --attempt {} --outcome <applied|not-applied>\nResolution changes history only; use the same configuration directory.",
        a.id.0,
        a.kind,
        a.record_id,
        record.metadata().run_id(),
        record.len(),
        record.applied_count(),
        record.metadata().arguments(),
        a.action_position + 1,
        a.action,
        a.details.paths.join("\n  "),
        a.details.instructions,
        a.id.0
    )))
}
pub(crate) fn check_interrupted(
    history: &mut History,
    fs: &FsHandler,
) -> Result<()> {
    if let Some(report) = interruption_report(history)? {
        println!("{report}");
    }
    if !fs.is_dry_run() {
        if history.current_attempt()?.is_some() {
            bail!(
                "Resolve the attempted action before changing files or history"
            );
        }
        if let Some(record) = history.close_abandoned_run()? {
            println!(
                "Closed interrupted run {} as partial; {} actions remain applied.",
                record.id().unwrap(),
                record.applied_count()
            );
        }
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
    check_interrupted(history, fs)?;
    if fs.is_dry_run() {
        return Ok(Record::new(
            actions.iter().map(StoredAction::from).collect(),
            metadata,
        ));
    }
    let run = history.begin_run(OperationKind::Apply, None, Some(metadata))?;
    let result = execute_actions(
        history,
        run,
        OperationKind::Apply,
        actions
            .into_iter()
            .enumerate()
            .map(|(p, a)| (p, StoredAction::from(&a)))
            .collect(),
        None,
    );
    finish_run(history, run, result)
}
pub(crate) fn replay_record(
    history: &mut History,
    fs: &FsHandler,
    record: &Record,
    direction: HistoryMode,
) -> Result<Record> {
    check_interrupted(history, fs)?;
    if fs.is_dry_run() {
        return Ok(record.clone());
    }
    let kind = match direction {
        HistoryMode::Undo => OperationKind::Undo,
        HistoryMode::Redo => OperationKind::Redo,
    };
    let positions: Vec<_> = match direction {
        HistoryMode::Undo => (0..record.applied_count()).rev().collect(),
        HistoryMode::Redo => (record.applied_count()..record.len()).collect(),
    };
    let actions = positions
        .into_iter()
        .map(|p| (p, record.actions()[p].clone()))
        .collect();
    let run = history.begin_run(kind, record.id(), None)?;
    let result = execute_actions(history, run, kind, actions, record.id());
    finish_run(history, run, result)
}
fn finish_run(
    history: &mut History,
    run: RunId,
    result: Result<()>,
) -> Result<Record> {
    match result {
        Ok(()) => Ok(history.close_run(run, true)?),
        Err(error) => {
            // Read committed database state; never infer the result from files.
            // If reading fails, leave the durable run/attempt untouched.
            match history.current_attempt() {
                Ok(Some(_)) => {
                    if let Ok(Some(report)) = interruption_report(history) {
                        eprintln!("{report}");
                    }
                },
                Ok(None) => {
                    history.close_run(run, false).wrap_err_with(|| {
                        format!("Original execution error: {error:#}")
                    })?;
                },
                Err(read_error) => {
                    return Err(error.wrap_err(format!("Could not read committed attempt: {read_error}; inspect history on the next invocation")));
                },
            }
            Err(error)
        },
    }
}
fn prepare(
    history: &History,
    kind: OperationKind,
    position: usize,
    action: &StoredAction,
    record_id: Option<usize>,
) -> Result<PreparedAction> {
    let executable = Action::try_from(action)?;
    Ok(match &executable {
        Action::EditTagValues { path, changes }
            if kind == OperationKind::Apply =>
        {
            prepare_tag_edit(path, changes)?
        },
        Action::EditTagValues { .. } => {
            let pair = history
                .patches(record_id.unwrap(), position)?
                .ok_or_else(|| {
                    color_eyre::eyre::eyre!("Recorded binary patches missing")
                })?;
            prepare_tag_replay(
                action,
                &pair,
                if kind == OperationKind::Undo {
                    HistoryMode::Undo
                } else {
                    HistoryMode::Redo
                },
            )?
        },
        _ => prepare_action(&executable, kind)?,
    })
}
fn execute_actions(
    history: &mut History,
    run: RunId,
    kind: OperationKind,
    actions: Vec<(usize, StoredAction)>,
    record_id: Option<usize>,
) -> Result<()> {
    for (position, action) in actions {
        let mut prepared =
            prepare(history, kind, position, &action, record_id)?;
        // Retain before committing intent: even an ambiguous commit cannot leave
        // durable reporting paths pointing to automatically deleted artifacts.
        prepared.retain_artifacts();
        let attempt = history.begin_attempt(run,position,prepared.action(),prepared.details(),prepared.patches())
            .wrap_err_with(||format!("Preparing history intent failed. Inspect these preparation paths if present: {}",prepared.details().paths.join(", ")))?;
        prepared.execute()?;
        history.confirm_attempt(attempt).wrap_err_with(||format!("History completion could not be confirmed. Inspect the recorded history and retained paths: {}",prepared.details().paths.join(", ")))?;
        prepared.confirm().wrap_err_with(||format!("Action is confirmed in history. Remove leftover artifacts manually: {}",prepared.details().paths.join(", ")))?;
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
mod manual_tests {
    use tfmttools_core::history::{AttemptOutcome, TemplateMetadata};
    use tfmttools_core::util::FSMode;

    use super::*;
    #[test]
    fn ordinary_error_preserves_confirmed_prefix_and_never_continues() {
        let d = assert_fs::TempDir::new().unwrap();
        let root = Utf8PathBuf::try_from(d.path().to_owned()).unwrap();
        let mut h = History::new(root.join("history"));
        let fs = FsHandler::new(FSMode::Default);
        let metadata = ActionRecordMetadata::new(
            TemplateMetadata::Validation { value: "test".into() },
            vec![],
            "test".into(),
        );
        assert!(
            execute_recorded(
                &mut h,
                &fs,
                vec![
                    Action::MakeDir(root.join("first")),
                    Action::MoveFile {
                        source: root.join("missing"),
                        target: root.join("target")
                    },
                    Action::MakeDir(root.join("last"))
                ],
                metadata
            )
            .is_err()
        );
        assert!(root.join("first").is_dir());
        assert!(!root.join("last").exists());
        assert_eq!(h.records()[0].applied_count(), 1);
        if let Some(a) = h.current_attempt().unwrap() {
            h.resolve_attempt(a.id, AttemptOutcome::NotApplied).unwrap();
        }
        let record = h.get_all_records_to_undo().unwrap().remove(0);
        replay_record(&mut h, &fs, &record, HistoryMode::Undo).unwrap();
        assert!(!root.join("first").exists());
        assert!(h.get_all_records_to_redo().unwrap().is_empty());
    }
}

#[cfg(test)]
mod failure_tests {
    use tfmttools_core::history::{AttemptOutcome, TemplateMetadata};
    use tfmttools_core::util::FSMode;

    use super::*;
    fn metadata() -> ActionRecordMetadata {
        ActionRecordMetadata::new(
            TemplateMetadata::Validation { value: "test".into() },
            vec![],
            "failure".into(),
        )
    }
    #[test]
    fn execution_error_keeps_attempt_and_blocks_new_mutations() {
        let dir = assert_fs::TempDir::new().unwrap();
        let root = Utf8PathBuf::try_from(dir.path().to_owned()).unwrap();
        let mut h = History::new(root.join("history"));
        let obstruction = root.join("obstruction");
        std::fs::write(&obstruction, b"original").unwrap();
        let fs = FsHandler::new(FSMode::Default);
        assert!(
            execute_recorded(
                &mut h,
                &fs,
                vec![
                    Action::MakeDir(root.join("first")),
                    Action::RemoveDir(obstruction.clone()),
                    Action::MakeDir(root.join("last"))
                ],
                metadata()
            )
            .is_err()
        );
        assert!(h.current_attempt().unwrap().is_some());
        assert_eq!(h.records()[0].applied_count(), 1);
        assert!(!root.join("last").exists());
        assert_eq!(std::fs::read(&obstruction).unwrap(), b"original");
        assert!(check_interrupted(&mut h, &fs).is_err());
        let a = h.current_attempt().unwrap().unwrap();
        h.resolve_attempt(a.id, AttemptOutcome::NotApplied).unwrap();
        let record = h.records()[0].clone();
        replay_record(&mut h, &fs, &record, HistoryMode::Undo).unwrap();
        assert!(!root.join("first").exists());
    }
    #[test]
    fn database_failure_after_install_retains_original_and_attempt() {
        use lofty::file::TaggedFileExt;
        use lofty::tag::ItemKey;
        use tfmttools_core::action::{TagValueChange, TagValueKind};
        let dir = assert_fs::TempDir::new().unwrap();
        let root = Utf8PathBuf::try_from(dir.path().to_owned()).unwrap();
        let path = root.join("song.mp3");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
            &path,
        )
        .unwrap();
        let before = std::fs::read(&path).unwrap();
        let audio = lofty::read_from_path(&path).unwrap();
        let title = audio
            .primary_tag()
            .unwrap()
            .get_string(ItemKey::TrackTitle)
            .unwrap();
        let action = Action::EditTagValues {
            path: path.clone(),
            changes: vec![TagValueChange::new(
                "track_title".into(),
                TagValueKind::Text,
                title.into(),
                "changed".into(),
            )],
        };
        let mut h = History::new(root.join("history"));
        let run =
            h.begin_run(OperationKind::Apply, None, Some(metadata())).unwrap();
        h.close_run(run, false).unwrap();
        let db = rusqlite::Connection::open(root.join("history")).unwrap();
        db.execute_batch("CREATE TRIGGER fail_confirmation BEFORE DELETE ON attempts BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
        assert!(
            execute_recorded(
                &mut h,
                &FsHandler::new(FSMode::Default),
                vec![action],
                metadata()
            )
            .is_err()
        );
        let a = h.current_attempt().unwrap().unwrap();
        assert_eq!(
            std::fs::read(a.details.paths.last().unwrap()).unwrap(),
            before
        );
        assert_ne!(std::fs::read(&path).unwrap(), before);
        assert_eq!(h.records().last().unwrap().applied_count(), 0);
        db.execute_batch("DROP TRIGGER fail_confirmation").unwrap();
        h.resolve_attempt(a.id, AttemptOutcome::Applied).unwrap();
        assert_eq!(h.records().last().unwrap().applied_count(), 1);
        assert_eq!(
            std::fs::read(a.details.paths.last().unwrap()).unwrap(),
            before
        );
    }
}
