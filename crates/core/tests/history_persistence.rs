use camino::Utf8PathBuf;
use serde_json::{Value, json};
use tempfile::TempDir;
use tfmttools_core::history::{History, HistoryError, LoadHistoryResult};

const LEGACY: &[u8] =
    include_bytes!("fixtures/history/v0-pre-canonical-tags.json");

fn path(directory: &TempDir) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(directory.path().join("tfmt.hist")).unwrap()
}

#[test]
fn legacy_load_is_read_only_and_save_preserves_exact_original_backup() {
    let directory = TempDir::new().unwrap();
    let path = path(&directory);
    std::fs::write(&path, LEGACY).unwrap();
    let mut history = History::new(path.clone());
    assert!(matches!(history.load().unwrap(), LoadHistoryResult::Loaded));
    let backup = directory.path().join("tfmt.hist.v0.bak");
    assert_eq!(std::fs::read(&path).unwrap(), LEGACY);
    assert!(!backup.exists());
    history.save().unwrap();
    assert_eq!(std::fs::read(&backup).unwrap(), LEGACY);
    let saved: Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(saved["schema_version"], 1);
    assert_eq!(
        saved["records"][0]["actions"][0]["changes"][0]["key"],
        "track_artist"
    );
    history.save().unwrap();
    assert_eq!(std::fs::read(backup).unwrap(), LEGACY);
}

#[test]
fn backup_collision_preserves_source_and_retry_uses_retained_original() {
    for directory_collision in [false, true] {
        let directory = TempDir::new().unwrap();
        let path = path(&directory);
        std::fs::write(&path, LEGACY).unwrap();
        let mut history = History::new(path.clone());
        history.load().unwrap();
        let backup = directory.path().join("tfmt.hist.v0.bak");
        if directory_collision {
            std::fs::create_dir(&backup).unwrap();
        } else {
            std::fs::write(&backup, b"different bytes").unwrap();
        }
        assert!(history.save().is_err());
        assert_eq!(std::fs::read(&path).unwrap(), LEGACY);
        if directory_collision {
            std::fs::remove_dir(&backup).unwrap();
        } else {
            assert_eq!(std::fs::read(&backup).unwrap(), b"different bytes");
            std::fs::remove_file(&backup).unwrap();
        }
        std::fs::write(&backup, LEGACY).unwrap();
        history.save().unwrap();
        assert_eq!(std::fs::read(&backup).unwrap(), LEGACY);
    }
}

#[test]
fn failed_load_preserves_live_records_and_pending_upgrade_backup() {
    let directory = TempDir::new().unwrap();
    let path = path(&directory);
    std::fs::write(&path, LEGACY).unwrap();
    let mut history = History::new(path.clone());
    history.load().unwrap();
    std::fs::write(&path, br#"{"schema_version":99,"records":[]}"#).unwrap();
    assert!(history.load().is_err());
    assert_eq!(history.records().len(), 1);
    assert_eq!(history.records()[0].metadata().run_id(), "old-tags");
    history.save().unwrap();
    assert_eq!(
        std::fs::read(directory.path().join("tfmt.hist.v0.bak")).unwrap(),
        LEGACY
    );
}

#[test]
fn current_and_new_history_do_not_create_upgrade_backups() {
    let directory = TempDir::new().unwrap();
    let path = path(&directory);
    let mut history = History::new(path.clone());
    assert!(matches!(history.load().unwrap(), LoadHistoryResult::New));
    history.save().unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap())
            .unwrap(),
        json!({"schema_version":1,"records":[]})
    );
    history.load().unwrap();
    history.save().unwrap();
    assert!(!directory.path().join("tfmt.hist.v0.bak").exists());
}

#[test]
fn parent_creation_failure_preserves_loaded_source_and_can_retry() {
    let directory = TempDir::new().unwrap();
    let parent = directory.path().join("parent");
    std::fs::create_dir(&parent).unwrap();
    let path = Utf8PathBuf::from_path_buf(parent.join("history.json")).unwrap();
    std::fs::write(&path, LEGACY).unwrap();
    let mut history = History::new(path.clone());
    history.load().unwrap();
    std::fs::rename(&parent, directory.path().join("original")).unwrap();
    std::fs::write(&parent, b"block directory creation").unwrap();
    assert!(history.save().is_err());
    assert_eq!(
        std::fs::read(directory.path().join("original/history.json")).unwrap(),
        LEGACY
    );
    std::fs::remove_file(&parent).unwrap();
    std::fs::rename(directory.path().join("original"), &parent).unwrap();
    history.save().unwrap();
    assert_eq!(
        std::fs::read(parent.join("history.json.v0.bak")).unwrap(),
        LEGACY
    );
}

#[test]
fn directory_destination_produces_v1_recovery_without_losing_upgrade_state() {
    let directory = TempDir::new().unwrap();
    let name = format!("tfmt-history-recovery-{}.json", std::process::id());
    let path =
        Utf8PathBuf::from_path_buf(directory.path().join(&name)).unwrap();
    std::fs::write(&path, LEGACY).unwrap();
    let mut history = History::new(path.clone());
    history.load().unwrap();
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    let HistoryError::SaveErrorWithBackup(_, recovery) =
        history.save().unwrap_err()
    else {
        panic!("recovery error")
    };
    let output: Value =
        serde_json::from_slice(&std::fs::read(&recovery).unwrap()).unwrap();
    assert_eq!(output["schema_version"], 1);
    std::fs::remove_file(recovery).unwrap();
    std::fs::remove_dir(&path).unwrap();
    std::fs::write(&path, LEGACY).unwrap();
    history.save().unwrap();
    assert_eq!(
        std::fs::read(directory.path().join(format!("{name}.v0.bak"))).unwrap(),
        LEGACY
    );
}

#[test]
fn new_history_creates_nested_parents_and_saves_relative_filenames() {
    let directory = TempDir::new().unwrap();
    let nested = Utf8PathBuf::from_path_buf(
        directory.path().join("new/nested/history.json"),
    )
    .unwrap();
    History::new(nested.clone()).save().unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(nested).unwrap())
            .unwrap()["schema_version"],
        1
    );
    let relative = Utf8PathBuf::from(format!(
        "tfmt-relative-history-{}.json",
        std::process::id()
    ));
    History::new(relative.clone()).save().unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(&relative).unwrap())
            .unwrap()["schema_version"],
        1
    );
    std::fs::remove_file(relative).unwrap();
}
