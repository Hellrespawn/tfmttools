use camino::Utf8PathBuf;
use rusqlite::Connection;
use tempfile::TempDir;
use tfmttools_core::history::{History, history_schema_sql};

fn database(dir: &TempDir) -> Utf8PathBuf {
    let path =
        Utf8PathBuf::from_path_buf(dir.path().join("tfmt.hist")).unwrap();
    let c = Connection::open(&path).unwrap();
    c.execute_batch(history_schema_sql()).unwrap();
    c.pragma_update(None, "user_version", 1).unwrap();
    path
}
#[test]
fn rejects_foreign_future_and_structurally_changed_databases() {
    for sql in [
        "PRAGMA application_id=0",
        "PRAGMA user_version=99",
        "DROP TABLE attempts",
        "CREATE TABLE unexpected(a TEXT)",
    ] {
        let dir = TempDir::new().unwrap();
        let path = database(&dir);
        Connection::open(&path)
            .unwrap()
            .execute_batch(&format!("PRAGMA foreign_keys=OFF; {sql}"))
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(History::new(path.clone()).load().is_err(), "{sql}");
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
#[test]
fn rejects_invalid_existing_values_and_foreign_keys() {
    for sql in [
        "INSERT INTO actions VALUES(99,0,'{}')",
        "INSERT INTO records(id,state,timestamp,metadata,complete) VALUES(0,'applied','bad','{}',1)",
        "INSERT INTO records(id,state,timestamp,metadata,complete) VALUES(0,'applied','2026-01-01T00:00:00+02:00','{}',1)",
        "INSERT INTO records(id,state,timestamp,metadata,complete) VALUES(0,'applied','2026-01-01T00:00:00+02:00','{\"template\":{\"type\":\"inline_template\",\"value\":\"test\"},\"arguments\":[],\"run_id\":\"r\"}',1); INSERT INTO actions VALUES(0,1,'{\"type\":\"make_dir\",\"path\":\"x\"}')",
    ] {
        let dir = TempDir::new().unwrap();
        let path = database(&dir);
        Connection::open(&path)
            .unwrap()
            .execute_batch(&format!("PRAGMA foreign_keys=OFF; {sql}"))
            .unwrap();
        assert!(History::new(path).load().is_err(), "{sql}");
    }
}
#[test]
fn read_only_access_does_not_modify_database() {
    let dir = TempDir::new().unwrap();
    let path = database(&dir);
    let before = std::fs::read(&path).unwrap();
    let mut h = History::open_read_only(path.clone()).unwrap();
    assert!(
        h.begin_run(tfmttools_core::history::RunRequest::Replay {
            record_id: 0,
            direction: tfmttools_core::history::HistoryMode::Undo
        })
        .is_err()
    );
    drop(h);
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[test]
fn preserves_recorded_timestamp_offset() {
    let dir = TempDir::new().unwrap();
    let path = database(&dir);
    Connection::open(&path).unwrap().execute_batch("INSERT INTO records(id,state,timestamp,metadata,complete) VALUES(7,'undone','2026-01-01T00:00:00+05:30','{\"template\":{\"type\":\"inline_template\",\"value\":\"test\"},\"arguments\":[],\"run_id\":\"r\"}',1)").unwrap();
    let h = History::open_read_only(path).unwrap();
    assert_eq!(h.records()[0].id(), Some(7));
    assert_eq!(
        h.records()[0].timestamp().to_rfc3339(),
        "2026-01-01T00:00:00+05:30"
    );
}

#[test]
fn read_only_history_cannot_remove_database() {
    let dir = TempDir::new().unwrap();
    let path = database(&dir);
    let mut h = History::open_read_only(path.clone()).unwrap();
    assert!(h.remove().is_err());
    assert!(path.exists());
}

#[test]
fn rejects_previous_development_schema_without_rewriting() {
    for read_only in [false, true] {
        let dir = TempDir::new().unwrap();
        let path = Utf8PathBuf::try_from(dir.path().join("old.db")).unwrap();
        Connection::open(&path)
            .unwrap()
            .execute_batch(include_str!(
                "fixtures/history/previous-development-schema.sql"
            ))
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        if read_only {
            assert!(History::open_read_only(path.clone()).is_err());
        } else {
            assert!(History::new(path.clone()).load().is_err());
        }
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

#[test]
fn records_are_ordered_by_id_with_gaps() {
    let dir = TempDir::new().unwrap();
    let path = database(&dir);
    let db = Connection::open(&path).unwrap();
    for id in [11, 2, 7] {
        let metadata = r#"{"template":{"type":"validation","value":"test"},"arguments":[],"run_id":"r"}"#;
        db.execute("INSERT INTO records(id,state,timestamp,metadata,complete,applied_count) VALUES(?1,'applied','2026-01-01T00:00:00+05:30',?2,1,1)", rusqlite::params![id, metadata]).unwrap();
        db.execute("INSERT INTO actions(record_id,position,payload) VALUES(?1,0,'{\"type\":\"make_dir\",\"path\":\"a\"}')", [id]).unwrap();
    }
    let h = History::open_read_only(path).unwrap();
    assert_eq!(
        h.records().iter().map(|r| r.id().unwrap()).collect::<Vec<_>>(),
        vec![2, 7, 11]
    );
    assert_eq!(
        h.records_for(tfmttools_core::history::HistoryMode::Undo, None)
            .iter()
            .map(|r| r.id().unwrap())
            .collect::<Vec<_>>(),
        vec![11, 7, 2]
    );
}
