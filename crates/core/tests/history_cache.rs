mod support;

use camino::Utf8PathBuf;
use tfmttools_core::history::{
    AttemptDetails, AttemptOutcome, History, HistoryMode, RunId, RunRequest,
    StoredAction,
};

fn action(name: &str) -> StoredAction {
    StoredAction::MakeDir { path: name.into() }
}
fn assert_reopened(history: &History, path: &Utf8PathBuf) {
    let reopened = History::open_read_only(path.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(history.records()).unwrap(),
        serde_json::to_value(reopened.records()).unwrap()
    );
}
fn attempt(
    history: &mut History,
    run: RunId,
    position: usize,
    name: &str,
) -> tfmttools_core::history::AttemptId {
    history
        .begin_attempt(
            run,
            position,
            &action(name),
            &AttemptDetails {
                paths: vec![name.into()],
                instructions: "Inspect directory".into(),
            },
            None,
        )
        .unwrap()
}

#[test]
fn cache_matches_reopen_through_apply_replay_and_both_resolutions() {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::try_from(dir.path().join("history")).unwrap();
    let mut history = History::new(path.clone());
    let run = history
        .begin_run(RunRequest::Apply {
            metadata: support::history::metadata("cache"),
        })
        .unwrap();
    assert_reopened(&history, &path);
    for (position, name) in ["a", "b"].into_iter().enumerate() {
        let id = attempt(&mut history, run, position, name);
        assert_reopened(&history, &path);
        history.confirm_attempt(id).unwrap();
        assert_reopened(&history, &path);
    }
    history.close_run(run, true).unwrap();
    assert_reopened(&history, &path);
    let undo = history
        .begin_run(RunRequest::Replay {
            record_id: 0,
            direction: HistoryMode::Undo,
        })
        .unwrap();
    let id = attempt(&mut history, undo, 1, "b");
    history.confirm_attempt(id).unwrap();
    assert_reopened(&history, &path);
    history.close_run(undo, true).unwrap();
    let redo = history
        .begin_run(RunRequest::Replay {
            record_id: 0,
            direction: HistoryMode::Redo,
        })
        .unwrap();
    let id = attempt(&mut history, redo, 1, "b");
    history.resolve_attempt(id, AttemptOutcome::Applied).unwrap();
    assert_reopened(&history, &path);
    assert_eq!(history.records()[0].applied_count(), 2);
    assert!(!history.records()[0].redo_allowed());
    let undo = history
        .begin_run(RunRequest::Replay {
            record_id: 0,
            direction: HistoryMode::Undo,
        })
        .unwrap();
    let id = attempt(&mut history, undo, 1, "b");
    history.resolve_attempt(id, AttemptOutcome::NotApplied).unwrap();
    assert_reopened(&history, &path);
    assert_eq!(history.records()[0].applied_count(), 2);
}

#[test]
fn first_applied_resolution_supersedes_redo_and_preserves_applied_prefix() {
    let dir = tempfile::tempdir().unwrap();
    let path = Utf8PathBuf::try_from(dir.path().join("history")).unwrap();
    let mut history = History::new(path.clone());
    for _ in 0..2 {
        support::history::apply(
            &mut history,
            vec![action("a"), action("b")],
            support::history::metadata("old"),
        );
    }
    support::history::replay(&mut history, 0, HistoryMode::Undo);
    let undo = history
        .begin_run(RunRequest::Replay {
            record_id: 1,
            direction: HistoryMode::Undo,
        })
        .unwrap();
    let id = attempt(&mut history, undo, 1, "b");
    history.confirm_attempt(id).unwrap();
    history.close_run(undo, true).unwrap();
    let run = history
        .begin_run(RunRequest::Apply {
            metadata: support::history::metadata("new"),
        })
        .unwrap();
    assert_eq!(history.records_for(HistoryMode::Redo, None).len(), 2);
    assert_reopened(&history, &path);
    let id = attempt(&mut history, run, 0, "new");
    history.resolve_attempt(id, AttemptOutcome::Applied).unwrap();
    assert_reopened(&history, &path);
    assert!(history.records_for(HistoryMode::Redo, None).is_empty());
    assert_eq!(
        history.records()[0].state(),
        tfmttools_core::history::RecordState::Superseded
    );
    assert_eq!(history.records()[1].applied_count(), 1);
    assert_eq!(
        history
            .records_for(HistoryMode::Undo, None)
            .iter()
            .map(|r| r.id().unwrap())
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
}
