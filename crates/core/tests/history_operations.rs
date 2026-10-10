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
        support::history::replay(&mut history, index, OperationKind::Undo);
    }
    support::history::replay(&mut history, 3, OperationKind::Redo);
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
    support::history::replay(&mut history, 0, OperationKind::Undo);
    support::history::replay(&mut history, 1, OperationKind::Undo);
    support::history::replay(&mut history, 1, OperationKind::Redo);
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
fn replay_requires_an_existing_finalized_record() {
    let (_directory, mut history) = history();
    support::history::apply(
        &mut history,
        vec![tfmttools_core::history::StoredAction::MakeDir {
            path: "test".into(),
        }],
        metadata(),
    );
    assert!(history.begin_run(OperationKind::Undo, None, None).is_err());
    assert!(history.begin_run(OperationKind::Undo, Some(8), None).is_err());
    assert!(history.current_attempt().unwrap().is_none());
    assert_eq!(history.records()[0].state(), RecordState::Applied);
}
