use camino::Utf8PathBuf;
use lofty::file::TaggedFileExt;
use lofty::tag::ItemKey;
use tfmttools_core::action::{TagValueChange, TagValueKind};
use tfmttools_core::history::{HistoryMode, RecoveryDescriptor};
use tfmttools_fs::{
    cleanup_prepared, install_prepared, prepare_tag_edit, prepare_tag_replay,
    recover_prepared,
};
fn fixture() -> (tempfile::TempDir, Utf8PathBuf, TagValueChange) {
    let dir = tempfile::tempdir().unwrap();
    let path =
        Utf8PathBuf::from_path_buf(dir.path().join("audio.mp3")).unwrap();
    std::fs::copy("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3", &path)
        .unwrap();
    let f = lofty::read_from_path(&path).unwrap();
    let title =
        f.primary_tag().unwrap().get_string(ItemKey::TrackTitle).unwrap();
    let change = TagValueChange::new(
        "track_title".into(),
        TagValueKind::Text,
        title.into(),
        "changed".into(),
    );
    (dir, path, change)
}
#[test]
fn prepares_separately_retains_original_and_replays_exact_bytes() {
    let (_dir, path, change) = fixture();
    let before = std::fs::read(&path).unwrap();
    let entry = prepare_tag_edit(&path, &[change]).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    install_prepared(&entry).unwrap();
    let after = std::fs::read(&path).unwrap();
    assert_ne!(before, after);
    let RecoveryDescriptor::FileSwitch { retained, .. } = &entry.recovery
    else {
        panic!()
    };
    assert_eq!(std::fs::read(retained).unwrap(), before);
    recover_prepared(&entry).unwrap();
    cleanup_prepared(&entry).unwrap();
    assert!(!std::path::Path::new(retained).exists());
    let undo = prepare_tag_replay(
        &entry.action,
        entry.patches.as_ref().unwrap(),
        HistoryMode::Undo,
    )
    .unwrap();
    install_prepared(&undo).unwrap();
    cleanup_prepared(&undo).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let redo = prepare_tag_replay(
        &entry.action,
        entry.patches.as_ref().unwrap(),
        HistoryMode::Redo,
    )
    .unwrap();
    install_prepared(&redo).unwrap();
    cleanup_prepared(&redo).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), after);
}
#[test]
fn resumes_after_original_is_retained_and_rejects_external_changes() {
    let (_dir, path, change) = fixture();
    let entry = prepare_tag_edit(&path, &[change]).unwrap();
    let RecoveryDescriptor::FileSwitch { resolved, retained, .. } =
        &entry.recovery
    else {
        panic!()
    };
    std::fs::rename(resolved, retained).unwrap();
    recover_prepared(&entry).unwrap();
    std::fs::write(&path, b"external").unwrap();
    assert!(recover_prepared(&entry).is_err());
    assert!(cleanup_prepared(&entry).is_err());
    assert!(std::path::Path::new(retained).exists());
}
#[test]
fn source_change_before_switch_preserves_all_files() {
    let (_dir, path, change) = fixture();
    let entry = prepare_tag_edit(&path, &[change]).unwrap();
    std::fs::write(&path, b"external").unwrap();
    assert!(install_prepared(&entry).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"external");
}
#[cfg(unix)]
#[test]
fn symlink_is_preserved_hard_links_and_redirected_symlinks_are_rejected() {
    let (dir, path, change) = fixture();
    let link = Utf8PathBuf::from_path_buf(dir.path().join("link.mp3")).unwrap();
    std::os::unix::fs::symlink("audio.mp3", &link).unwrap();
    let entry = prepare_tag_edit(&link, std::slice::from_ref(&change)).unwrap();
    install_prepared(&entry).unwrap();
    cleanup_prepared(&entry).unwrap();
    assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
    let original = lofty::read_from_path(&path).unwrap();
    let title = original
        .primary_tag()
        .unwrap()
        .get_string(ItemKey::TrackTitle)
        .unwrap();
    let change = TagValueChange::new(
        "track_title".into(),
        TagValueKind::Text,
        title.into(),
        "next".into(),
    );
    let entry = prepare_tag_edit(&link, std::slice::from_ref(&change)).unwrap();
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink("elsewhere.mp3", &link).unwrap();
    assert!(install_prepared(&entry).is_err());
    std::fs::hard_link(&path, dir.path().join("hard.mp3")).unwrap();
    assert!(prepare_tag_edit(&path, &[change]).is_err());
}

#[test]
fn retained_slot_collision_is_preserved_and_candidate_loss_restores_source() {
    let (_dir, path, change) = fixture();
    let before = std::fs::read(&path).unwrap();
    let entry = prepare_tag_edit(&path, &[change]).unwrap();
    let RecoveryDescriptor::FileSwitch { candidate, retained, .. } =
        &entry.recovery
    else {
        panic!()
    };
    std::fs::write(retained, b"unrelated").unwrap();
    assert!(install_prepared(&entry).is_err());
    assert_eq!(std::fs::read(retained).unwrap(), b"unrelated");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::write(retained, b"").unwrap();
    std::fs::remove_file(candidate).unwrap();
    assert!(install_prepared(&entry).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
#[cfg(unix)]
#[test]
fn preserves_original_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, path, change) = fixture();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))
        .unwrap();
    let entry = prepare_tag_edit(&path, &[change]).unwrap();
    install_prepared(&entry).unwrap();
    cleanup_prepared(&entry).unwrap();
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn identical_patch_bytes_switch_and_recover_without_losing_original() {
    let (_dir, path, change) = fixture();
    let bytes = std::fs::read(&path).unwrap();
    let pair = tfmttools_fs::create_patch_pair(&bytes, &bytes).unwrap();
    let action = tfmttools_core::history::StoredAction::from(
        &tfmttools_core::action::Action::EditTagValues {
            path: path.clone(),
            changes: vec![change],
        },
    );
    let entry = prepare_tag_replay(&action, &pair, HistoryMode::Redo).unwrap();
    install_prepared(&entry).unwrap();
    recover_prepared(&entry).unwrap();
    let RecoveryDescriptor::FileSwitch { retained, .. } = &entry.recovery
    else {
        panic!()
    };
    assert_eq!(std::fs::read(retained).unwrap(), bytes);
    cleanup_prepared(&entry).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
