mod support;

use camino::Utf8PathBuf;
use tfmttools_core::history::{
    ActionRecordMetadata, History, OperationKind, Record, RecordState,
    TemplateMetadata,
};

fn history() -> (tempfile::TempDir, History) {
    let directory = tempfile::tempdir().unwrap();
    let history = History::new(
        Utf8PathBuf::try_from(directory.path().join("h.hist")).unwrap(),
    );
    (directory, history)
}

fn metadata() -> ActionRecordMetadata {
    ActionRecordMetadata::new(
        TemplateMetadata::InlineTemplate { value: "path: $title".into() },
        vec![],
        "test".into(),
    )
}

fn ids(records: &[Record]) -> Vec<Option<usize>> {
    records.iter().map(Record::id).collect()
}

#[test]
fn undo_is_reverse_and_redo_is_forward() {
    let (_directory, mut history) = history();
    for _ in 0..4 {
        support::history::apply(
            &mut history,
            vec![tfmttools_core::history::StoredAction::MakeDir {
                path: "test".into(),
            }],
            metadata(),
        );
    }
    for index in [0, 2, 3] {
        support::history::replay(
            &mut history,
            index,
            tfmttools_core::history::HistoryMode::Undo,
        );
    }
    support::history::replay(
        &mut history,
        3,
        tfmttools_core::history::HistoryMode::Redo,
    );
    assert_eq!(ids(&history.get_all_records_to_undo().unwrap()), vec![
        Some(3),
        Some(1)
    ]);
    assert_eq!(ids(&history.get_all_records_to_redo().unwrap()), vec![
        Some(0),
        Some(2)
    ]);
    assert_eq!(ids(&history.get_n_records_to_undo(1).unwrap()), vec![Some(3)]);
    assert_eq!(ids(&history.get_n_records_to_redo(1).unwrap()), vec![Some(0)]);
    assert!(history.get_n_records_to_undo(0).unwrap().is_empty());
    assert!(history.get_n_records_to_redo(0).unwrap().is_empty());
}

#[test]
fn apply_supersedes_only_undone_records() {
    let (_directory, mut history) = history();
    for _ in 0..3 {
        support::history::apply(
            &mut history,
            vec![tfmttools_core::history::StoredAction::MakeDir {
                path: "test".into(),
            }],
            metadata(),
        );
    }
    support::history::replay(
        &mut history,
        0,
        tfmttools_core::history::HistoryMode::Undo,
    );
    support::history::replay(
        &mut history,
        1,
        tfmttools_core::history::HistoryMode::Undo,
    );
    support::history::replay(
        &mut history,
        1,
        tfmttools_core::history::HistoryMode::Redo,
    );
    support::history::apply(
        &mut history,
        vec![tfmttools_core::history::StoredAction::MakeDir {
            path: "test".into(),
        }],
        metadata(),
    );
    assert_eq!(
        history.records().iter().map(Record::state).collect::<Vec<_>>(),
        vec![
            RecordState::Superseded,
            RecordState::Redone,
            RecordState::Applied,
            RecordState::Applied
        ]
    );
}

#[test]
fn ids_follow_existing_record_count() {
    let (_directory, mut history) = history();
    support::history::apply(
        &mut history,
        vec![tfmttools_core::history::StoredAction::MakeDir {
            path: "test".into(),
        }],
        metadata(),
    );
    support::history::apply(
        &mut history,
        vec![tfmttools_core::history::StoredAction::MakeDir {
            path: "test".into(),
        }],
        metadata(),
    );
    assert_eq!(ids(history.records()), vec![Some(0), Some(1)]);
}

#[test]
fn replay_requires_an_existing_complete_record() {
    let (_directory, mut history) = history();
    support::history::apply(
        &mut history,
        vec![tfmttools_core::history::StoredAction::MakeDir {
            path: "test".into(),
        }],
        metadata(),
    );
    assert!(
        history
            .begin_run(tfmttools_core::history::RunRequest::Replay {
                record_id: 8,
                direction: tfmttools_core::history::HistoryMode::Undo
            })
            .is_err()
    );
    assert!(history.current_attempt().unwrap().is_none());
    assert_eq!(history.records()[0].state(), RecordState::Applied);
}

#[test]
fn replay_eligibility_and_cursor_share_progress_rules() {
    use tfmttools_core::history::StoredAction;
    let base = serde_json::to_value(Record::new(
        vec![
            StoredAction::MakeDir { path: "a".into() },
            StoredAction::MakeDir { path: "b".into() },
        ],
        metadata(),
    ))
    .unwrap();
    for (state, applied, redo, undo_ok, redo_ok, undo_pos, redo_pos) in [
        ("applied", 2, true, true, false, Some(1), None),
        ("applied", 1, true, true, true, Some(0), Some(1)),
        ("undone", 0, true, false, true, None, Some(0)),
        ("redone", 1, true, true, true, Some(0), Some(1)),
        ("superseded", 0, false, false, false, None, None),
        ("applied", 1, false, true, false, Some(0), None),
    ] {
        let mut value = base.clone();
        value["state"] = state.into();
        value["applied_count"] = applied.into();
        value["complete"] = true.into();
        value["redo_allowed"] = redo.into();
        let record: Record = serde_json::from_value(value).unwrap();
        assert_eq!(record.can_undo(), undo_ok, "{state}/{applied}");
        assert_eq!(record.can_redo(), redo_ok, "{state}/{applied}");
        assert_eq!(record.next_action_position(OperationKind::Undo), undo_pos);
        assert_eq!(record.next_action_position(OperationKind::Redo), redo_pos);
        assert_eq!(record.next_action_position(OperationKind::Apply), Some(2));
    }
    let empty = Record::new(vec![], metadata());
    assert!(!empty.can_undo());
    assert!(!empty.can_redo());
}
