use tfmttools_core::history::history_schema_sql;
#[test]
fn schema_matches_snapshot() {
    assert_eq!(
        history_schema_sql(),
        include_str!("../../../docs/history/schema-v1.sql")
    );
}

#[test]
fn new_history_uses_initial_schema_and_opens_read_only() {
    let directory = tempfile::tempdir().unwrap();
    let path = camino::Utf8PathBuf::try_from(directory.path().join("history"))
        .unwrap();
    let mut history = tfmttools_core::history::History::new(path.clone());
    let run = history
        .begin_run(
            tfmttools_core::history::OperationKind::Apply,
            None,
            Some(tfmttools_core::history::ActionRecordMetadata::new(
                tfmttools_core::history::TemplateMetadata::Validation {
                    value: "test".into(),
                },
                vec![],
                "initial".into(),
            )),
        )
        .unwrap();
    history.close_run(run, true).unwrap();
    drop(history);
    let db = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    drop(db);
    assert!(tfmttools_core::history::History::open_read_only(path).is_ok());
}
