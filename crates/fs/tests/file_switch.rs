use camino::Utf8PathBuf;
use lofty::file::TaggedFileExt;
use lofty::tag::ItemKey;
use tfmttools_core::action::{TagValueChange, TagValueKind};
use tfmttools_core::history::HistoryMode;
use tfmttools_fs::{prepare_tag_edit, prepare_tag_replay};
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
    let mut entry = prepare_tag_edit(&path, &[change]).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    entry.execute().unwrap();
    let after = std::fs::read(&path).unwrap();
    assert_ne!(before, after);
    let retained = entry.details().paths.last().unwrap().clone();
    assert_eq!(std::fs::read(&retained).unwrap(), before);
    assert!(entry.execute().is_err());
    entry.confirm().unwrap();
    assert!(!std::path::Path::new(&retained).exists());
    let mut undo = prepare_tag_replay(
        entry.action(),
        entry.patches().unwrap(),
        HistoryMode::Undo,
    )
    .unwrap();
    undo.execute().unwrap();
    undo.confirm().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let mut redo = prepare_tag_replay(
        entry.action(),
        entry.patches().unwrap(),
        HistoryMode::Redo,
    )
    .unwrap();
    redo.execute().unwrap();
    redo.confirm().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), after);
}
#[test]
fn install_failure_preserves_original_backup_for_manual_repair() {
    let (_dir, path, change) = fixture();
    let before = std::fs::read(&path).unwrap();
    let mut entry = prepare_tag_edit(&path, &[change]).unwrap();
    let paths = entry.details().paths.clone();
    entry.retain_artifacts();
    // Obstruct installation after the original is moved aside.
    // Replacing the candidate with a directory forces rename failure.
    std::fs::remove_file(&paths[2]).unwrap();
    std::fs::create_dir(&paths[2]).unwrap();
    assert!(entry.execute().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}
#[test]
fn source_change_before_switch_preserves_all_files() {
    let (_dir, path, change) = fixture();
    let mut entry = prepare_tag_edit(&path, &[change]).unwrap();
    std::fs::write(&path, b"external").unwrap();
    assert!(entry.execute().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"external");
}
#[cfg(unix)]
#[test]
fn symlink_is_preserved_hard_links_and_redirected_symlinks_are_rejected() {
    let (dir, path, change) = fixture();
    let link = Utf8PathBuf::from_path_buf(dir.path().join("link.mp3")).unwrap();
    std::os::unix::fs::symlink("audio.mp3", &link).unwrap();
    let mut entry =
        prepare_tag_edit(&link, std::slice::from_ref(&change)).unwrap();
    entry.execute().unwrap();
    entry.confirm().unwrap();
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
    let mut entry =
        prepare_tag_edit(&link, std::slice::from_ref(&change)).unwrap();
    std::fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink("elsewhere.mp3", &link).unwrap();
    assert!(entry.execute().is_err());
    std::fs::hard_link(&path, dir.path().join("hard.mp3")).unwrap();
    assert!(prepare_tag_edit(&path, &[change]).is_err());
}

#[test]
fn cleanup_failure_keeps_installed_bytes_and_reports_backup() {
    let (_dir, path, change) = fixture();
    let mut entry = prepare_tag_edit(&path, &[change]).unwrap();
    entry.execute().unwrap();
    let after = std::fs::read(&path).unwrap();
    let backup = entry.details().paths.last().unwrap().clone();
    std::fs::remove_file(&backup).unwrap();
    std::fs::create_dir(&backup).unwrap();
    assert!(entry.confirm().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), after);
    assert!(std::path::Path::new(&backup).is_dir());
}
#[cfg(unix)]
#[test]
fn preserves_original_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let (_dir, path, change) = fixture();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))
        .unwrap();
    let mut entry = prepare_tag_edit(&path, &[change]).unwrap();
    entry.execute().unwrap();
    entry.confirm().unwrap();
    assert_eq!(
        std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn identical_patch_bytes_switch_once_without_losing_original() {
    let (_dir, path, change) = fixture();
    let bytes = std::fs::read(&path).unwrap();
    let pair = tfmttools_fs::create_patch_pair(&bytes, &bytes).unwrap();
    let action = tfmttools_core::history::StoredAction::from(
        &tfmttools_core::action::Action::EditTagValues {
            path: path.clone(),
            changes: vec![change],
        },
    );
    let mut entry =
        prepare_tag_replay(&action, &pair, HistoryMode::Redo).unwrap();
    entry.execute().unwrap();
    assert!(entry.execute().is_err());
    let retained = entry.details().paths.last().unwrap().clone();
    assert_eq!(std::fs::read(&retained).unwrap(), bytes);
    entry.confirm().unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
