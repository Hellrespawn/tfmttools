use camino::Utf8PathBuf;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::history::OperationKind;
use tfmttools_core::util::{FSMode, MoveMode, Utf8File};
use tfmttools_fs::{
    ActionExecutor, FsHandler, cleanup_prepared, install_prepared,
    prepare_action, recover_prepared,
};
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
            let entry = prepare_action(action, OperationKind::Apply).unwrap();
            install_prepared(&entry).unwrap();
            recover_prepared(&entry).unwrap();
            cleanup_prepared(&entry).unwrap();
        }
        assert_eq!(std::fs::read(&a).unwrap(), b"B");
        assert_eq!(std::fs::read(&b).unwrap(), b"A");
        for action in actions.iter().rev() {
            let entry = prepare_action(action, OperationKind::Undo).unwrap();
            install_prepared(&entry).unwrap();
            cleanup_prepared(&entry).unwrap();
        }
        assert_eq!(std::fs::read(&a).unwrap(), b"A");
        assert_eq!(std::fs::read(&b).unwrap(), b"B");
    }
}
#[test]
fn rejects_changed_target_and_resumes_copy_before_source_removal() {
    let dir = tempfile::tempdir().unwrap();
    let a = path(&dir, "a");
    let b = path(&dir, "b");
    std::fs::write(&a, b"A").unwrap();
    let copy = Action::CopyFile { source: a.clone(), target: b.clone() };
    let entry = prepare_action(&copy, OperationKind::Apply).unwrap();
    install_prepared(&entry).unwrap();
    recover_prepared(&entry).unwrap();
    let remove =
        prepare_action(&Action::RemoveFile(a.clone()), OperationKind::Apply)
            .unwrap();
    install_prepared(&remove).unwrap();
    recover_prepared(&remove).unwrap();
    assert!(!a.exists());
    let undo = prepare_action(&copy, OperationKind::Undo).unwrap();
    install_prepared(&undo).unwrap();
    recover_prepared(&undo).unwrap();
    assert_eq!(std::fs::read(&a).unwrap(), b"A");
    assert!(!b.exists());
    let move_action = prepare_action(
        &Action::MoveFile { source: a.clone(), target: b.clone() },
        OperationKind::Apply,
    )
    .unwrap();
    std::fs::write(&b, b"external").unwrap();
    assert!(install_prepared(&move_action).is_err());
    assert_eq!(std::fs::read(b).unwrap(), b"external");
}
#[test]
fn directory_recovery_preserves_unexpected_contents() {
    let dir = tempfile::tempdir().unwrap();
    let p = path(&dir, "nested");
    let action = Action::MakeDir(p.clone());
    let entry = prepare_action(&action, OperationKind::Apply).unwrap();
    install_prepared(&entry).unwrap();
    recover_prepared(&entry).unwrap();
    let undo = prepare_action(&action, OperationKind::Undo).unwrap();
    std::fs::write(p.join("external"), b"keep").unwrap();
    assert!(install_prepared(&undo).is_err());
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
    let entry = prepare_action(&action, OperationKind::Apply).unwrap();
    install_prepared(&entry).unwrap();
    cleanup_prepared(&entry).unwrap();
    std::fs::write(&source, b"external").unwrap();
    assert!(prepare_action(&action, OperationKind::Undo).is_err());
    assert_eq!(std::fs::read(&source).unwrap(), b"external");
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    std::fs::write(&source, b"original").unwrap();
    let undo = prepare_action(&action, OperationKind::Undo).unwrap();
    install_prepared(&undo).unwrap();
    cleanup_prepared(&undo).unwrap();
    assert_eq!(std::fs::read(&source).unwrap(), b"original");
    assert!(!target.exists());
    let redo = prepare_action(&action, OperationKind::Redo).unwrap();
    install_prepared(&redo).unwrap();
    cleanup_prepared(&redo).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
}
