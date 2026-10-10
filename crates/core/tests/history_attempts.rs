mod support;
use camino::Utf8PathBuf;
use tfmttools_core::history::{
    AttemptDetails, AttemptOutcome, History, OperationKind, StoredAction,
};

fn setup() -> (tempfile::TempDir, History) {
    let dir = tempfile::tempdir().unwrap();
    let h = History::new(
        Utf8PathBuf::try_from(dir.path().join("history")).unwrap(),
    );
    (dir, h)
}
fn action(name: &str) -> StoredAction {
    StoredAction::MakeDir { path: name.into() }
}
fn attempt(
    h: &mut History,
    run: tfmttools_core::history::RunId,
    pos: usize,
    name: &str,
) -> tfmttools_core::history::AttemptId {
    h.begin_attempt(
        run,
        pos,
        &action(name),
        &AttemptDetails {
            paths: vec![name.into()],
            instructions: "Check directory".into(),
        },
        None,
    )
    .unwrap()
}
#[test]
fn confirmed_prefix_survives_error_and_not_applied_resolution() {
    let (_dir, mut h) = setup();
    let run = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(support::history::metadata("partial")),
        )
        .unwrap();
    for (pos, name) in ["a", "b"].iter().enumerate() {
        let id = attempt(&mut h, run, pos, name);
        h.confirm_attempt(id).unwrap();
    }
    let failed = attempt(&mut h, run, 2, "c");
    h.load().unwrap();
    assert_eq!(h.records()[0].len(), 2);
    assert!(
        h.begin_run(
            OperationKind::Apply,
            None,
            Some(support::history::metadata("blocked"))
        )
        .is_err()
    );
    let r = h.resolve_attempt(failed, AttemptOutcome::NotApplied).unwrap();
    assert_eq!(r.applied_count(), 2);
    assert!(!r.is_complete());
    assert!(!r.redo_allowed());
    assert!(h.current_attempt().unwrap().is_none());
    assert_eq!(h.get_all_records_to_undo().unwrap().len(), 1);
}
#[test]
fn manual_applied_resolution_and_partial_undo_use_cursor() {
    let (_dir, mut h) = setup();
    let run = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(support::history::metadata("apply")),
        )
        .unwrap();
    let id = attempt(&mut h, run, 0, "a");
    let r = h.resolve_attempt(id, AttemptOutcome::Applied).unwrap();
    assert_eq!(r.len(), 1);
    let undo = h.begin_run(OperationKind::Undo, r.id(), None).unwrap();
    let id = attempt(&mut h, undo, 0, "a");
    h.resolve_attempt(id, AttemptOutcome::Applied).unwrap();
    assert_eq!(h.records()[0].applied_count(), 0);
    assert!(h.get_all_records_to_redo().unwrap().is_empty());
}
#[test]
fn abandoned_run_closes_without_actions_or_attempt() {
    let (_dir, mut h) = setup();
    let run = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(support::history::metadata("abandoned")),
        )
        .unwrap();
    let id = attempt(&mut h, run, 0, "a");
    h.confirm_attempt(id).unwrap();
    h.load().unwrap();
    let r = h.close_abandoned_run().unwrap().unwrap();
    assert_eq!(r.applied_count(), 1);
    assert!(!r.is_complete());
    assert!(h.close_abandoned_run().unwrap().is_none());
}
#[test]
fn failed_confirmation_is_atomic_and_stale_attempts_cannot_resolve_new_work() {
    let (dir, mut h) = setup();
    let run = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(support::history::metadata("failure")),
        )
        .unwrap();
    let id = attempt(&mut h, run, 0, "a");
    let c = rusqlite::Connection::open(dir.path().join("history")).unwrap();
    c.execute_batch("CREATE TRIGGER fail_confirmation BEFORE DELETE ON attempts BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(h.confirm_attempt(id).is_err());
    assert!(h.records()[0].is_empty());
    assert_eq!(h.current_attempt().unwrap().unwrap().id, id);
    c.execute_batch("DROP TRIGGER fail_confirmation").unwrap();
    h.resolve_attempt(id, AttemptOutcome::NotApplied).unwrap();
    let run = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(support::history::metadata("next")),
        )
        .unwrap();
    let next = attempt(&mut h, run, 0, "b");
    assert_ne!(id, next);
    assert!(h.resolve_attempt(id, AttemptOutcome::Applied).is_err());
    assert_eq!(h.current_attempt().unwrap().unwrap().id, next);
}
#[test]
fn partial_undo_remains_selectable_and_failed_replay_disables_redo() {
    let (_dir, mut h) = setup();
    let r = support::history::apply(
        &mut h,
        vec![action("a"), action("b")],
        support::history::metadata("full"),
    );
    let run = h.begin_run(OperationKind::Undo, r.id(), None).unwrap();
    let id = attempt(&mut h, run, 1, "b");
    h.confirm_attempt(id).unwrap();
    let r = h.close_run(run, false).unwrap();
    assert_eq!(r.applied_count(), 1);
    assert_eq!(h.get_all_records_to_undo().unwrap()[0].applied_count(), 1);
    assert!(h.get_all_records_to_redo().unwrap().is_empty());
    let run = h.begin_run(OperationKind::Undo, r.id(), None).unwrap();
    let id = attempt(&mut h, run, 0, "a");
    h.confirm_attempt(id).unwrap();
    h.close_run(run, true).unwrap();
    assert!(h.get_all_records_to_undo().unwrap().is_empty());
    assert!(h.get_all_records_to_redo().unwrap().is_empty());
}
#[test]
fn migration_preserves_completed_v1_and_refuses_pending_v1_without_writes() {
    for pending in [false, true] {
        let (dir, mut h) = setup();
        let path = dir.path().join("history");
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(include_str!("../src/history/schema-v1.sql")).unwrap();
        c.pragma_update(None, "user_version", 1).unwrap();
        let metadata =
            serde_json::to_string(&support::history::metadata("old")).unwrap();
        c.execute("INSERT INTO records VALUES(7,0,'applied','2026-01-01T00:00:00+05:30',?1,1)",[metadata]).unwrap();
        c.execute("INSERT INTO actions VALUES(7,0,?1)", [
            serde_json::to_string(&action("a")).unwrap(),
        ])
        .unwrap();
        if pending {
            c.execute("INSERT INTO operations VALUES(1,7,'undo',0,'[]')", [])
                .unwrap();
        }
        drop(c);
        let before = std::fs::read(&path).unwrap();
        if pending {
            assert!(h.load().unwrap_err().to_string().contains("compatible"));
            assert_eq!(std::fs::read(path).unwrap(), before);
        } else {
            h.load().unwrap();
            assert_eq!(h.records()[0].id(), Some(7));
            assert_eq!(h.records()[0].applied_count(), 1);
            assert!(h.records()[0].is_complete());
            assert_eq!(
                h.records()[0].timestamp().to_rfc3339(),
                "2026-01-01T00:00:00+05:30"
            );
        }
    }
}
#[test]
fn loading_rejects_attempt_that_disagrees_with_record_cursor() {
    let (dir, mut h) = setup();
    let run = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(support::history::metadata("tampered")),
        )
        .unwrap();
    attempt(&mut h, run, 0, "a");
    drop(h);
    let path = Utf8PathBuf::try_from(dir.path().join("history")).unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE attempts SET action_position=3", []).unwrap();
    drop(c);
    assert!(History::new(path).load().is_err());
}
#[test]
fn invalid_v1_payload_is_rejected_before_migration_writes() {
    let (dir, mut h) = setup();
    let path = dir.path().join("history");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(include_str!("../src/history/schema-v1.sql")).unwrap();
    c.pragma_update(None, "user_version", 1).unwrap();
    c.execute("INSERT INTO records VALUES(0,0,'applied','invalid','{}',1)", [])
        .unwrap();
    drop(c);
    let before = std::fs::read(&path).unwrap();
    assert!(h.load().is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[test]
fn resolving_unknown_attempt_does_not_create_history() {
    let (dir, mut h) = setup();
    assert!(
        h.resolve_attempt(
            tfmttools_core::history::AttemptId(42),
            AttemptOutcome::NotApplied
        )
        .is_err()
    );
    assert!(!dir.path().join("history").exists());
}
