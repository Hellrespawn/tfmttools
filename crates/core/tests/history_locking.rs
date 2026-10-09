use camino::Utf8PathBuf;
use tempfile::TempDir;
use tfmttools_core::history::{
    ActionRecordMetadata, History, TemplateMetadata,
};

type TestHistory = History;

fn history_path(directory: &TempDir) -> Utf8PathBuf {
    Utf8PathBuf::try_from(directory.path().join("nested/history.json")).unwrap()
}

#[test]
fn loaded_history_excludes_other_sessions_until_dropped() {
    let directory = TempDir::new().unwrap();
    let path = history_path(&directory);
    let mut first = TestHistory::new(path.clone());
    first.load().unwrap();
    // Repeated loads must not attempt to lock an already locked handle.
    first.load().unwrap();

    let mut second = TestHistory::new(path);
    assert!(second.load().is_err());

    first
        .push(
            vec![],
            ActionRecordMetadata::new(
                TemplateMetadata::Validation { value: "test".into() },
                vec![],
                "first".into(),
            ),
        )
        .unwrap();
    first.save().unwrap();
    assert!(second.load().is_err());
    drop(first);

    second.load().unwrap();
    assert_eq!(second.records().len(), 1);
    assert_eq!(second.records()[0].metadata().run_id(), "first");
}

#[test]
fn removing_history_keeps_other_sessions_excluded() {
    let directory = TempDir::new().unwrap();
    let path = history_path(&directory);
    let mut first = TestHistory::new(path.clone());
    first.load().unwrap();
    first.save().unwrap();
    first.remove().unwrap();
    assert!(!path.exists());

    let mut second = TestHistory::new(path);
    assert!(second.load().is_err());
    drop(first);
    second.load().unwrap();
    assert!(second.is_empty());
}

#[test]
fn load_error_releases_lock_when_session_drops() {
    fn load(path: Utf8PathBuf) -> tfmttools_core::history::Result<()> {
        let mut history = TestHistory::new(path);
        history.load()?;
        Ok(())
    }

    let directory = TempDir::new().unwrap();
    let path = history_path(&directory);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "invalid json").unwrap();

    assert!(load(path.clone()).is_err());
    let lock = std::fs::File::options()
        .read(true)
        .write(true)
        .open(format!("{path}.lock"))
        .unwrap();
    lock.try_lock().unwrap();
}

#[test]
fn saving_new_history_acquires_lock() {
    let directory = TempDir::new().unwrap();
    let path = history_path(&directory);
    let mut first = TestHistory::new(path.clone());
    first.save().unwrap();
    let mut second = TestHistory::new(path);
    assert!(second.load().is_err());
}

#[cfg(unix)]
#[test]
fn symlink_and_direct_history_paths_share_the_same_lock() {
    let directory = TempDir::new().unwrap();
    let target =
        Utf8PathBuf::try_from(directory.path().join("history.json")).unwrap();
    let link =
        Utf8PathBuf::try_from(directory.path().join("alias.json")).unwrap();
    std::os::unix::fs::symlink("history.json", &link).unwrap();
    let mut first = History::new(link);
    first.load().unwrap();
    let mut second = History::new(target);
    assert!(second.load().is_err());
    first.save().unwrap();
    assert!(second.load().is_err());
    drop(first);
    second.load().unwrap();
}

#[cfg(unix)]
#[test]
fn clearing_history_through_symlink_preserves_link_and_removes_database() {
    let directory = TempDir::new().unwrap();
    let target =
        Utf8PathBuf::try_from(directory.path().join("history.hist")).unwrap();
    let link =
        Utf8PathBuf::try_from(directory.path().join("alias.hist")).unwrap();
    std::os::unix::fs::symlink("history.hist", &link).unwrap();
    let mut history = History::new(link.clone());
    history.load().unwrap();
    history.save().unwrap();
    history.remove().unwrap();
    assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    assert!(!target.exists());
    history.save().unwrap();
    assert!(target.is_file());
}
