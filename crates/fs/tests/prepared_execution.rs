use camino::Utf8PathBuf;
use tfmttools_core::action::{Action, RenameAction};
use tfmttools_core::history::OperationKind;
use tfmttools_core::util::{MoveMode, Utf8File};
use tfmttools_fs::{plan_actions, prepare_action};
fn path(dir: &tempfile::TempDir, name: &str) -> Utf8PathBuf {
    Utf8PathBuf::try_from(dir.path().join(name)).unwrap()
}
#[test]
fn planned_swap_and_forced_copy_replay_in_order() {
    for mode in [MoveMode::Auto, MoveMode::AlwaysCopy] {
        let dir = tempfile::tempdir().unwrap();
        let a = path(&dir, "a");
        let b = path(&dir, "b");
        std::fs::write(&a, b"A").unwrap();
        std::fs::write(&b, b"B").unwrap();
        let actions = plan_actions(
            vec![
                RenameAction::new(Utf8File::new(&a), Utf8File::new(&b)),
                RenameAction::new(Utf8File::new(&b), Utf8File::new(&a)),
            ],
            mode,
        )
        .unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), b"A");
        assert_eq!(std::fs::read(&b).unwrap(), b"B");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
        if matches!(mode, MoveMode::AlwaysCopy) {
            let (pairs, remainder) = actions.as_chunks::<2>();
            assert!(remainder.is_empty());
            for pair in pairs {
                assert!(matches!(&pair[0], Action::CopyFile { source, .. }
                    if matches!(&pair[1], Action::RemoveFile(path) if path == source)));
            }
        }
        for action in &actions {
            let copied = match action {
                Action::CopyFile { source, target } => {
                    Some((source, target, std::fs::read(source).unwrap()))
                },
                _ => None,
            };
            let mut entry =
                prepare_action(action, OperationKind::Apply).unwrap();
            entry.execute().unwrap();
            entry.confirm().unwrap();
            if let Some((source, target, original)) = copied {
                // Copy verification and confirmation precede the separate
                // source removal, even for temporary staging paths in a cycle.
                assert_eq!(std::fs::read(source).unwrap(), original);
                assert_eq!(std::fs::read(target).unwrap(), original);
            }
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
fn copy_preserves_read_only_permissions_without_reopening_for_writes() {
    let dir = tempfile::tempdir().unwrap();
    let source = path(&dir, "source");
    let target = path(&dir, "target");
    std::fs::write(&source, b"original").unwrap();
    let writable = std::fs::metadata(&source).unwrap().permissions();
    let mut read_only = writable.clone();
    read_only.set_readonly(true);
    std::fs::set_permissions(&source, read_only).unwrap();
    let action =
        Action::CopyFile { source: source.clone(), target: target.clone() };
    let mut entry = prepare_action(&action, OperationKind::Apply).unwrap();
    let result = entry.execute();
    let permissions = std::fs::metadata(&target).unwrap().permissions();
    std::fs::set_permissions(&source, writable.clone()).unwrap();
    std::fs::set_permissions(&target, writable).unwrap();
    result.unwrap();
    assert!(permissions.readonly());
    assert_eq!(std::fs::read(&source).unwrap(), b"original");
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
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

#[cfg(unix)]
#[test]
#[ignore = "helper subprocess for interrupted_copy_has_only_reported_paths"]
fn interrupted_copy_worker() {
    let root =
        Utf8PathBuf::from(std::env::var("TFMT_INTERRUPTED_COPY_ROOT").unwrap());
    let mut p = prepare_action(
        &Action::CopyFile {
            source: root.join("source"),
            target: root.join("target"),
        },
        OperationKind::Apply,
    )
    .unwrap();
    p.execute().unwrap();
}
#[cfg(unix)]
#[test]
fn interrupted_copy_has_only_reported_paths() {
    let dir = tempfile::tempdir().unwrap();
    let source = path(&dir, "source");
    let target = path(&dir, "target");
    let before = vec![b'x'; 64 * 1024];
    std::fs::write(&source, &before).unwrap();
    // A file-size limit terminates the child during the write, deterministically
    // leaving the same artifacts as process interruption. It is local to child.
    let status=std::process::Command::new("/bin/sh")
        .args(["-c","ulimit -c 0; ulimit -f 1; exec \"$1\" --ignored --exact interrupted_copy_worker", "copy-test"])
        .arg(std::env::current_exe().unwrap())
        .env("TFMT_INTERRUPTED_COPY_ROOT",dir.path())
        .stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .status().unwrap();
    assert!(!status.success());
    assert!(
        target.exists(),
        "Partial copy must be at its reported destination"
    );
    assert!(std::fs::metadata(&target).unwrap().len() < before.len() as u64);
    assert_eq!(std::fs::read(source).unwrap(), before);
    assert_eq!(
        std::fs::read_dir(dir.path()).unwrap().count(),
        2,
        "No unreported candidate artifacts"
    );
}

#[test]
fn planning_relative_missing_destination_uses_current_device_without_effects() {
    let dir = tempfile::tempdir().unwrap();
    let source = path(&dir, "source");
    std::fs::write(&source, b"original").unwrap();
    let target = Utf8PathBuf::from(format!(
        ".tfmt-missing-{}/nested/target",
        std::process::id()
    ));
    assert!(!target.ancestors().nth(2).unwrap().exists());
    let actions = plan_actions(
        vec![RenameAction::new(Utf8File::new(&source), Utf8File::new(&target))],
        MoveMode::Auto,
    )
    .unwrap();
    assert!(actions.iter().any(|action| matches!(action,
        Action::MoveFile { source: s, target: t } | Action::CopyFile { source: s, target: t }
            if s == &source && t == &target)));
    assert_eq!(std::fs::read(&source).unwrap(), b"original");
    assert!(!target.ancestors().nth(2).unwrap().exists());
}

#[cfg(unix)]
#[test]
fn cross_device_plan_copies_and_replays_when_two_devices_are_available() {
    use std::os::unix::fs::MetadataExt;

    let source_dir = tempfile::tempdir().unwrap();
    let target_dir =
        tempfile::tempdir_in(std::env::current_dir().unwrap()).unwrap();
    if source_dir.path().metadata().unwrap().dev()
        == target_dir.path().metadata().unwrap().dev()
    {
        return; // A second writable device is optional on developer machines.
    }
    let source = path(&source_dir, "source");
    let target = path(&target_dir, "target");
    std::fs::write(&source, b"across devices").unwrap();
    let actions = plan_actions(
        vec![RenameAction::new(Utf8File::new(&source), Utf8File::new(&target))],
        MoveMode::Auto,
    )
    .unwrap();
    assert!(matches!(actions.as_slice(), [
        Action::CopyFile { .. },
        Action::RemoveFile(_)
    ]));
    assert_eq!(std::fs::read(&source).unwrap(), b"across devices");
    assert!(!target.exists());
    for action in &actions {
        let mut prepared =
            prepare_action(action, OperationKind::Apply).unwrap();
        prepared.execute().unwrap();
        prepared.confirm().unwrap();
    }
    assert!(!source.exists());
    assert_eq!(std::fs::read(&target).unwrap(), b"across devices");
    for action in actions.iter().rev() {
        let mut prepared = prepare_action(action, OperationKind::Undo).unwrap();
        prepared.execute().unwrap();
        prepared.confirm().unwrap();
    }
    assert_eq!(std::fs::read(&source).unwrap(), b"across devices");
    assert!(!target.exists());
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "helper subprocess for large_file_actions_use_bounded_memory"]
fn large_move_worker() {
    let root =
        Utf8PathBuf::from(std::env::var("TFMT_LARGE_MOVE_ROOT").unwrap());
    let source = root.join("source");
    let target = root.join("target");
    let mut entry = prepare_action(
        &Action::MoveFile { source: source.clone(), target: target.clone() },
        OperationKind::Apply,
    )
    .unwrap();
    entry.execute().unwrap();
    entry.confirm().unwrap();
    assert!(!source.exists());
    assert_eq!(std::fs::metadata(&target).unwrap().len(), 64 * 1024 * 1024);
    let copied = root.join("copied");
    let mut copy = prepare_action(
        &Action::CopyFile { source: target.clone(), target: copied.clone() },
        OperationKind::Apply,
    )
    .unwrap();
    copy.execute().unwrap();
    copy.confirm().unwrap();
    assert_eq!(std::fs::metadata(copied).unwrap().len(), 64 * 1024 * 1024);
    let mut entry = prepare_action(
        &Action::RemoveFile(target.clone()),
        OperationKind::Apply,
    )
    .unwrap();
    entry.execute().unwrap();
    assert!(!target.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn large_file_actions_use_bounded_memory() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::File::create(dir.path().join("source"))
        .unwrap()
        .set_len(64 * 1024 * 1024)
        .unwrap();
    // The sparse file exceeds the child's address-space limit.
    let output = std::process::Command::new("/bin/sh")
        .args(["-c", "ulimit -c 0; ulimit -v 65536; exec \"$1\" --ignored --exact large_move_worker", "move-test"])
        .arg(std::env::current_exe().unwrap())
        .env("TFMT_LARGE_MOVE_ROOT", dir.path())
        .output().unwrap();
    assert!(output.status.success(), "{output:?}");
}
