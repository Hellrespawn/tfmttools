mod support;

use camino::Utf8PathBuf;
use tempfile::TempDir;
use tfmttools_core::action::Action;
use tfmttools_core::history::*;
fn metadata() -> ActionRecordMetadata {
    ActionRecordMetadata::new(
        TemplateMetadata::Validation { value: "test".into() },
        vec![],
        "run".into(),
    )
}
#[test]
fn committed_intent_survives_reopen_and_finalizes_only_after_actions() {
    let dir = TempDir::new().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("h.hist")).unwrap();
    let mut h = History::new(path.clone());
    h.load().unwrap();
    let id = h
        .begin_operation(OperationKind::Apply, None, Some(metadata()))
        .unwrap();
    h.set_operation_plan(id, &[StoredAction::from(&Action::MakeDir(
        "dir".into(),
    ))])
    .unwrap();
    assert!(h.finish_operation(id).is_err());
    assert!(
        h.begin_operation(OperationKind::Apply, None, Some(metadata()))
            .is_err()
    );
    let entry = PreparedAction {
        action: StoredAction::from(&Action::MakeDir("dir".into())),
        recovery: RecoveryDescriptor::Directory {
            path: "dir".into(),
            before_exists: false,
            after_exists: true,
        },
        patches: None,
    };
    h.append_prepared(id, &entry).unwrap();
    drop(h);
    let mut h = History::new(path);
    h.load().unwrap();
    assert!(h.records().is_empty());
    assert_eq!(h.pending_operations().unwrap().len(), 1);
    assert!(h.remove().is_err());
    assert!(
        h.begin_operation(OperationKind::Apply, None, Some(metadata()))
            .is_err()
    );
    h.complete_action(id, 0).unwrap();
    h.complete_action(id, 0).unwrap();
    let record = h.finish_operation(id).unwrap();
    assert_eq!(record.id(), Some(0));
    assert_eq!(record.state(), RecordState::Applied);
    assert_eq!(h.finish_operation(id).unwrap().id(), record.id());
    h.complete_cleanup(id, 0).unwrap();
    assert!(h.pending_operations().unwrap().is_empty());
}
#[test]
fn replay_finalization_updates_state_and_new_run_supersedes_redo() {
    let dir = TempDir::new().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("h.hist")).unwrap();
    let mut h = History::new(path);
    h.load().unwrap();
    support::history::apply(&mut h, vec![], metadata());
    let id = h.begin_operation(OperationKind::Undo, Some(0), None).unwrap();
    h.set_operation_plan(id, &[]).unwrap();
    h.finish_operation(id).unwrap();
    assert_eq!(h.records()[0].state(), RecordState::Undone);
    assert!(h.begin_operation(OperationKind::Undo, Some(0), None).is_err());
    let id = h
        .begin_operation(OperationKind::Apply, None, Some(metadata()))
        .unwrap();
    h.set_operation_plan(id, &[]).unwrap();
    h.finish_operation(id).unwrap();
    assert_eq!(h.records()[0].state(), RecordState::Superseded);
    assert_eq!(h.records()[1].state(), RecordState::Applied);
}

#[test]
fn failed_finalization_preserves_committed_progress_and_retries() {
    let dir = TempDir::new().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("h.hist")).unwrap();
    let mut h = History::new(path.clone());
    h.load().unwrap();
    let id = h
        .begin_operation(OperationKind::Apply, None, Some(metadata()))
        .unwrap();
    let action = StoredAction::from(&Action::MakeDir("dir".into()));
    h.set_operation_plan(id, std::slice::from_ref(&action)).unwrap();
    h.append_prepared(id, &PreparedAction {
        action,
        recovery: RecoveryDescriptor::Directory {
            path: "dir".into(),
            before_exists: false,
            after_exists: true,
        },
        patches: None,
    })
    .unwrap();
    h.complete_action(id, 0).unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch("CREATE TRIGGER reject_commit BEFORE UPDATE ON records BEGIN SELECT RAISE(ABORT,'injected'); END").unwrap();
    assert!(h.finish_operation(id).is_err());
    assert!(h.records().is_empty());
    assert!(h.pending_operations().unwrap()[0].entries[0].completed);
    assert!(!h.pending_operations().unwrap()[0].finalized);
    c.execute_batch("DROP TRIGGER reject_commit").unwrap();
    drop(c);
    drop(h);
    let mut h = History::new(path);
    h.load().unwrap();
    h.finish_operation(id).unwrap();
    h.complete_cleanup(id, 0).unwrap();
    assert_eq!(h.records().len(), 1);
}

#[test]
fn patch_blobs_and_descriptors_survive_reopen() {
    let dir = TempDir::new().unwrap();
    let path = Utf8PathBuf::from_path_buf(dir.path().join("h.hist")).unwrap();
    let mut h = History::new(path.clone());
    h.load().unwrap();
    let action = StoredAction::EditTagValues {
        path: "audio.mp3".into(),
        changes: vec![],
    };
    let pair = BinaryPatchPair {
        format: "bsdiff40-v1".into(),
        before: ByteIdentity { length: 3, sha256: [1; 32] },
        after: ByteIdentity { length: 3, sha256: [2; 32] },
        forward: include_bytes!("fixtures/history/forward.bsdiff").to_vec(),
        reverse: include_bytes!("fixtures/history/reverse.bsdiff").to_vec(),
    };
    let id = h
        .begin_operation(OperationKind::Apply, None, Some(metadata()))
        .unwrap();
    h.set_operation_plan(id, std::slice::from_ref(&action)).unwrap();
    h.append_prepared(id, &PreparedAction {
        action,
        recovery: RecoveryDescriptor::FileSwitch {
            path: "audio.mp3".into(),
            resolved: "/tmp/audio.mp3".into(),
            candidate: "/tmp/candidate.mp3".into(),
            retained: "/tmp/retained.mp3".into(),
            before: pair.before.clone(),
            after: pair.after.clone(),
        },
        patches: Some(pair.clone()),
    })
    .unwrap();
    drop(h);
    let mut h = History::new(path.clone());
    h.load().unwrap();
    assert_eq!(h.patches(0, 0).unwrap().unwrap().forward, pair.forward);
    assert_eq!(
        h.pending_operations().unwrap()[0].entries[0]
            .prepared
            .patches
            .as_ref()
            .unwrap()
            .reverse,
        pair.reverse
    );
    let c = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        c.query_row("SELECT typeof(forward) FROM patches", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        "blob"
    );
    h.complete_action(id, 0).unwrap();
    h.finish_operation(id).unwrap();
    h.complete_cleanup(id, 0).unwrap();
}

#[test]
fn rejects_recovery_descriptors_that_reverse_action_semantics() {
    let dir = TempDir::new().unwrap();
    let path = Utf8PathBuf::try_from(dir.path().join("h.hist")).unwrap();
    let mut h = History::new(path);
    h.load().unwrap();
    for (action, recovery) in [
        (
            StoredAction::from(&Action::CopyFile {
                source: "source".into(),
                target: "target".into(),
            }),
            RecoveryDescriptor::Copy {
                source: "source".into(),
                target: "target".into(),
                identity: ByteIdentity { length: 3, sha256: [1; 32] },
                remove_source: true,
                candidate: ".tfmt-copy-test".into(),
            },
        ),
        (
            StoredAction::from(&Action::MakeDir("dir".into())),
            RecoveryDescriptor::Directory {
                path: "dir".into(),
                before_exists: true,
                after_exists: false,
            },
        ),
        (
            StoredAction::from(&Action::RemoveDir("dir".into())),
            RecoveryDescriptor::Directory {
                path: "dir".into(),
                before_exists: false,
                after_exists: true,
            },
        ),
    ] {
        let id = h
            .begin_operation(OperationKind::Apply, None, Some(metadata()))
            .unwrap();
        h.set_operation_plan(id, std::slice::from_ref(&action)).unwrap();
        assert!(
            h.append_prepared(id, &PreparedAction {
                action,
                recovery,
                patches: None
            })
            .is_err()
        );
        h.cancel_unstarted(id).unwrap();
    }
}

#[test]
fn rejects_tampered_recovery_effects_on_reopen_without_writing() {
    let dir = TempDir::new().unwrap();
    let path = Utf8PathBuf::try_from(dir.path().join("h.hist")).unwrap();
    let mut h = History::new(path.clone());
    h.load().unwrap();
    let action = StoredAction::from(&Action::CopyFile {
        source: "source".into(),
        target: "target".into(),
    });
    let id = h
        .begin_operation(OperationKind::Apply, None, Some(metadata()))
        .unwrap();
    h.set_operation_plan(id, std::slice::from_ref(&action)).unwrap();
    h.append_prepared(id, &PreparedAction {
        action,
        recovery: RecoveryDescriptor::Copy {
            source: "source".into(),
            target: "target".into(),
            identity: ByteIdentity { length: 3, sha256: [1; 32] },
            remove_source: false,
            candidate: ".tfmt-copy-test".into(),
        },
        patches: None,
    })
    .unwrap();
    drop(h);
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE progress SET recovery=json_set(recovery,'$.remove_source',json('true'))", []).unwrap();
    drop(c);
    let before = std::fs::read(&path).unwrap();
    assert!(History::open_read_only(path.clone()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
