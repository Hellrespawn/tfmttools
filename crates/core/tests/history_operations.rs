use camino::Utf8PathBuf;
use tfmttools_core::history::{
    ActionRecordMetadata, History, Record, RecordState, TemplateMetadata,
};

fn history() -> History {
    History::new(Utf8PathBuf::from("unused.hist"))
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
    let mut history = history();
    for _ in 0..4 {
        history.push(vec![], metadata()).unwrap();
    }
    for index in [0, 2] {
        history
            .set_record_state(
                history.records()[index].clone(),
                RecordState::Undone,
            )
            .unwrap();
    }
    history
        .set_record_state(history.records()[3].clone(), RecordState::Redone)
        .unwrap();
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
fn push_supersedes_only_undone_records() {
    let mut history = history();
    for _ in 0..3 {
        history.push(vec![], metadata()).unwrap();
    }
    history
        .set_record_state(history.records()[0].clone(), RecordState::Undone)
        .unwrap();
    history
        .set_record_state(history.records()[1].clone(), RecordState::Redone)
        .unwrap();
    history.push(vec![], metadata()).unwrap();
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
    let mut history = history();
    history.push(vec![], metadata()).unwrap();
    history.push(vec![], metadata()).unwrap();
    assert_eq!(ids(history.records()), vec![Some(0), Some(1)]);
}

#[test]
fn state_updates_require_unique_saved_id() {
    let mut history = history();
    history.push(vec![], metadata()).unwrap();
    let record = history.records()[0].clone();
    let mut unsaved = record.clone();
    *unsaved.id_mut() = None;
    assert!(
        history
            .set_record_state(unsaved, RecordState::Undone)
            .unwrap_err()
            .to_string()
            .contains("unsaved")
    );
    let mut missing = record;
    *missing.id_mut() = Some(8);
    assert!(
        history
            .set_record_state(missing, RecordState::Undone)
            .unwrap_err()
            .to_string()
            .contains("Unable to find")
    );
    history.push(vec![], metadata()).unwrap();
    let mut duplicate = history.records()[1].clone();
    *duplicate.id_mut() = Some(0);
    // A duplicate loaded ID is covered through persistence below; mutation
    // itself must reject ambiguous saved IDs rather than changing one silently.
    let doc = r#"{"schema_version":1,"records":[{"id":0,"actions":[],"state":"applied","timestamp":"2026-10-07T12:00:00+02:00","metadata":{"template":{"type":"inline_template","value":"text"},"arguments":[],"run_id":"a"}},{"id":0,"actions":[],"state":"applied","timestamp":"2026-10-07T12:00:00+02:00","metadata":{"template":{"type":"inline_template","value":"text"},"arguments":[],"run_id":"b"}}]}"#;
    let path = std::env::temp_dir()
        .join(format!("tfmt-duplicate-{}.json", std::process::id()));
    std::fs::write(&path, doc).unwrap();
    let mut loaded =
        History::new(Utf8PathBuf::from_path_buf(path.clone()).unwrap());
    loaded.load().unwrap();
    assert!(
        loaded
            .set_record_state(duplicate, RecordState::Undone)
            .unwrap_err()
            .to_string()
            .contains("multiple")
    );
    std::fs::remove_file(path).unwrap();
}
