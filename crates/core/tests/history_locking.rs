mod support;

use camino::Utf8PathBuf;
use tempfile::TempDir;
use tfmttools_core::history::History;

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

    support::history::apply(
        &mut first,
        vec![],
        support::history::metadata("first"),
    );
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
    support::history::apply(
        &mut first,
        vec![],
        support::history::metadata("first"),
    );
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
fn recording_new_history_acquires_lock() {
    let directory = TempDir::new().unwrap();
    let path = history_path(&directory);
    let mut first = TestHistory::new(path.clone());
    support::history::apply(
        &mut first,
        vec![],
        support::history::metadata("first"),
    );
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
    support::history::apply(
        &mut first,
        vec![],
        support::history::metadata("first"),
    );
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
    support::history::apply(
        &mut history,
        vec![],
        support::history::metadata("test"),
    );
    history.remove().unwrap();
    assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    assert!(!target.exists());
    support::history::apply(
        &mut history,
        vec![],
        support::history::metadata("test"),
    );
    assert!(target.is_file());
}

#[test]
fn read_only_reporting_keeps_the_loaded_snapshot_until_reload() {
    use tfmttools_core::history::{AttemptDetails, RunRequest, StoredAction};

    let directory = TempDir::new().unwrap();
    let path = history_path(&directory);
    let mut writer = History::new(path.clone());
    support::history::apply(
        &mut writer,
        vec![],
        support::history::metadata("first"),
    );
    let mut reader = History::open_read_only(path.clone()).unwrap();
    let run = writer
        .begin_run(RunRequest::Apply {
            metadata: support::history::metadata("second"),
        })
        .unwrap();
    let mut marker_free = History::open_read_only(path).unwrap();
    let attempt = writer
        .begin_attempt(
            run,
            0,
            &StoredAction::MakeDir { path: "pending".into() },
            &AttemptDetails {
                paths: vec!["pending".into()],
                instructions: "Inspect".into(),
            },
            None,
        )
        .unwrap();
    assert!(reader.current_attempt().unwrap().is_none());
    assert!(reader.open_run().unwrap().is_none());
    assert_eq!(reader.records().len(), 1);
    assert!(marker_free.current_attempt().unwrap().is_none());
    assert_eq!(marker_free.open_run().unwrap().unwrap().0, run);
    reader.load().unwrap();
    assert_eq!(reader.current_attempt().unwrap().unwrap().id, attempt);
    assert_eq!(reader.records().len(), 2);
    writer.confirm_attempt(attempt).unwrap();
    writer.close_run(run, true).unwrap();
    assert_eq!(reader.current_attempt().unwrap().unwrap().id, attempt);
    assert_eq!(reader.records()[1].applied_count(), 0);
    assert_eq!(reader.open_run().unwrap().unwrap().0, run);
    marker_free.load().unwrap();
    assert!(marker_free.open_run().unwrap().is_none());
    assert!(marker_free.current_attempt().unwrap().is_none());
    assert_eq!(marker_free.records()[1].applied_count(), 1);
    writer.remove().unwrap();
    marker_free.load().unwrap();
    assert!(marker_free.is_empty());
    assert!(marker_free.current_attempt().unwrap().is_none());
    assert!(marker_free.open_run().unwrap().is_none());
}
