use camino::Utf8PathBuf;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::history::OperationKind;
use tfmttools_core::util::{FSMode, MoveMode, Utf8File};
use tfmttools_fs::{ActionExecutor, FsHandler, prepare_action};
fn path(dir: &tempfile::TempDir, name: &str) -> Utf8PathBuf {
    Utf8PathBuf::try_from(dir.path().join(name)).unwrap()
}
#[test]
fn recorded_swap_and_forced_copy_replay_in_order() {
    for mode in [MoveMode::Auto, MoveMode::AlwaysCopy] {
        let dir = tempfile::tempdir().unwrap();
        let a = path(&dir, "a");
        let b = path(&dir, "b");
        std::fs::write(&a, b"A").unwrap();
        std::fs::write(&b, b"B").unwrap();
        let fs = FsHandler::new(FSMode::Default);
        let ex = ActionExecutor::new(&fs).move_mode(mode);
        let actions = ex
            .plan_actions(vec![
                RenameAction::new(Utf8File::new(&a), Utf8File::new(&b)),
                RenameAction::new(Utf8File::new(&b), Utf8File::new(&a)),
            ])
            .unwrap();
        for action in &actions {
            let mut entry =
                prepare_action(action, OperationKind::Apply).unwrap();
            entry.execute().unwrap();

            entry.confirm().unwrap();
        }
        assert_eq!(std::fs::read(&a).unwrap(), b"B");
        assert_eq!(std::fs::read(&b).unwrap(), b"A");
        for action in actions.iter().rev() {
            let mut entry =
                prepare_action(action, OperationKind::Undo).unwrap();
            entry.execute().unwrap();
            entry.confirm().unwrap();
        }
        assert_eq!(std::fs::read(&a).unwrap(), b"A");
        assert_eq!(std::fs::read(&b).unwrap(), b"B");
    }
}
#[test]
fn copy_then_delete_and_reverse_preserve_bytes_and_reject_collisions() {
    let dir = tempfile::tempdir().unwrap();
    let a = path(&dir, "a");
    let b = path(&dir, "b");
    std::fs::write(&a, b"A").unwrap();
    let copy = Action::CopyFile { source: a.clone(), target: b.clone() };
    let mut entry = prepare_action(&copy, OperationKind::Apply).unwrap();
    entry.execute().unwrap();

    let mut remove =
        prepare_action(&Action::RemoveFile(a.clone()), OperationKind::Apply)
            .unwrap();
    remove.execute().unwrap();

    assert!(!a.exists());
    let mut undo = prepare_action(&copy, OperationKind::Undo).unwrap();
    undo.execute().unwrap();

    assert_eq!(std::fs::read(&a).unwrap(), b"A");
    assert!(!b.exists());
    let mut move_action = prepare_action(
        &Action::MoveFile { source: a.clone(), target: b.clone() },
        OperationKind::Apply,
    )
    .unwrap();
    std::fs::write(&b, b"external").unwrap();
    assert!(move_action.execute().is_err());
    assert_eq!(std::fs::read(b).unwrap(), b"external");
}
#[test]
fn directory_removal_preserves_nonempty_contents() {
    let dir = tempfile::tempdir().unwrap();
    let p = path(&dir, "nested");
    let action = Action::MakeDir(p.clone());
    let mut entry = prepare_action(&action, OperationKind::Apply).unwrap();
    entry.execute().unwrap();

    let mut undo = prepare_action(&action, OperationKind::Undo).unwrap();
    std::fs::write(p.join("external"), b"keep").unwrap();
    undo.execute().unwrap();
    assert!(p.join("external").exists());
}

#[test]
fn standalone_copy_undo_preserves_existing_original_and_redo_restores_copy() {
    let dir = tempfile::tempdir().unwrap();
    let source = path(&dir, "source");
    let target = path(&dir, "target");
    std::fs::write(&source, b"original").unwrap();
    let action =
        Action::CopyFile { source: source.clone(), target: target.clone() };
    let mut entry = prepare_action(&action, OperationKind::Apply).unwrap();
    entry.execute().unwrap();
    entry.confirm().unwrap();
    std::fs::write(&source, b"external").unwrap();
    assert!(prepare_action(&action, OperationKind::Undo).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), b"external");
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    std::fs::write(&source, b"original").unwrap();
    let mut undo = prepare_action(&action, OperationKind::Undo).unwrap();
    undo.execute().unwrap();
    undo.confirm().unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), b"original");
    assert!(!target.exists());
    let mut redo = prepare_action(&action, OperationKind::Redo).unwrap();
    redo.execute().unwrap();
    redo.confirm().unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
}

#[cfg(unix)]
#[test]
fn move_preparation_rejects_distinct_case_sensitive_hard_links() {
    let dir = tempfile::tempdir().unwrap();
    let source = path(&dir, "Original");
    let target = path(&dir, "original");
    std::fs::write(&source, b"original").unwrap();
    if target.exists() {
        return; // This test requires distinct case-sensitive names.
    }
    std::fs::hard_link(&source, &target).unwrap();
    let action =
        Action::MoveFile { source: source.clone(), target: target.clone() };
    assert!(prepare_action(&action, OperationKind::Apply).is_err());
    assert_eq!(std::fs::read(source).unwrap(), b"original");
    assert_eq!(std::fs::read(target).unwrap(), b"original");
}

#[test]
fn file_action_preparation_rejects_directories_before_creating_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    let source = path(&dir, "source");
    let target = path(&dir, "target");
    std::fs::create_dir(&source).unwrap();
    for action in [
        Action::MoveFile { source: source.clone(), target: target.clone() },
        Action::CopyFile { source: source.clone(), target: target.clone() },
        Action::RemoveFile(source.clone()),
    ] {
        assert!(prepare_action(&action, OperationKind::Apply).is_err());
    }
    assert!(source.is_dir());
    assert!(!target.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
#[test]
fn failed_copy_preserves_source_and_reports_both_paths() {
    let dir = tempfile::tempdir().unwrap();
    let source = path(&dir, "source");
    let target = path(&dir, "missing-parent/target");
    std::fs::write(&source, b"original").unwrap();
    let mut prepared = prepare_action(
        &Action::CopyFile { source: source.clone(), target: target.clone() },
        OperationKind::Apply,
    )
    .unwrap();
    assert!(prepared.execute().is_err());
    assert_eq!(std::fs::read(&source).unwrap(), b"original");
    assert_eq!(prepared.details().paths, vec![
        source.to_string(),
        target.to_string()
    ]);
}
