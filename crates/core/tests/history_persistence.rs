mod support;

use camino::Utf8PathBuf;
use tempfile::TempDir;
use tfmttools_core::action::Action;
use tfmttools_core::history::{
    ActionRecordMetadata, History, OperationKind, StoredAction,
    TemplateMetadata,
};

fn metadata(run: &str) -> ActionRecordMetadata {
    ActionRecordMetadata::new(
        TemplateMetadata::InlineTemplate { value: "opaque\n$artist".into() },
        vec!["--test".into()],
        run.into(),
    )
}

#[test]
fn saves_sqlite_and_preserves_order_metadata_and_replay_states() {
    let dir = TempDir::new().unwrap();
    let path =
        Utf8PathBuf::from_path_buf(dir.path().join("tfmt.hist")).unwrap();
    let mut h = History::new(path.clone());
    h.load().unwrap();
    support::history::apply(
        &mut h,
        vec![
            StoredAction::from(&Action::MakeDir("first".into())),
            StoredAction::from(&Action::RemoveDir("second".into())),
        ],
        metadata("a"),
    );
    support::history::apply(&mut h, vec![], metadata("b"));
    support::history::replay(&mut h, 0, OperationKind::Undo);
    let expected = serde_json::to_value(h.records()).unwrap();
    drop(h);
    assert!(std::fs::read(&path).unwrap().starts_with(b"SQLite format 3\0"));
    let mut loaded = History::new(path);
    loaded.load().unwrap();
    assert_eq!(serde_json::to_value(loaded.records()).unwrap(), expected);
    assert_eq!(loaded.get_all_records_to_redo().unwrap()[0].id(), Some(0));
}

#[test]
fn rejects_json_without_importing_or_rewriting() {
    let dir = TempDir::new().unwrap();
    let path =
        Utf8PathBuf::from_path_buf(dir.path().join("tfmt.hist")).unwrap();
    for bytes in [
        include_bytes!("fixtures/history/v0-pre-canonical-tags.json")
            .as_slice(),
        b"{\"schema_version\":1,\"records\":[]}".as_slice(),
    ] {
        std::fs::write(&path, bytes).unwrap();
        let mut h = History::new(path.clone());
        let error = h.load().unwrap_err().to_string();
        assert!(
            error.contains("JSON") && error.contains("unsupported"),
            "{error}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        assert!(
            h.begin_operation(
                OperationKind::Apply,
                None,
                Some(metadata("rejected"))
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}
