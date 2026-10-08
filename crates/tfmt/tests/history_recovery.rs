use std::process::{Command, Output};

use assert_fs::TempDir;
use camino::Utf8PathBuf;
use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame};
use lofty::tag::TagExt;
use tfmttools_core::history::{History, RecordState};
fn setup() -> TempDir {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("config")).unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
        dir.path().join("song.mp3"),
    )
    .unwrap();
    let mut tag = Id3v2Tag::default();
    tag.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new("TIT2").unwrap(),
        TextEncoding::UTF8,
        "Old ? title",
    )));
    tag.save_to_path(dir.path().join("song.mp3"), WriteOptions::default())
        .unwrap();
    dir
}
fn run(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tfmt"))
        .current_dir(dir.path())
        .args(["--simple", "--yes", "--config-directory", "config"])
        .args(args)
        .output()
        .unwrap()
}
fn success(out: Output) {
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
fn state(dir: &TempDir) -> RecordState {
    History::open_read_only(
        Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist")).unwrap(),
    )
    .unwrap()
    .records()[0]
        .state()
}
#[test]
fn tag_fix_undo_redo_restores_complete_recorded_bytes_and_rejects_changed_files()
 {
    let dir = setup();
    let path = dir.path().join("song.mp3");
    let before = std::fs::read(&path).unwrap();
    success(run(&dir, &["validate", "characters", "--fix"]));
    let after = std::fs::read(&path).unwrap();
    assert_ne!(before, after);
    success(run(&dir, &["undo"]));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(state(&dir), RecordState::Undone);
    success(run(&dir, &["redo"]));
    assert_eq!(std::fs::read(&path).unwrap(), after);
    assert_eq!(state(&dir), RecordState::Redone);
    std::fs::write(&path, b"external edit").unwrap();
    assert!(!run(&dir, &["undo"]).status.success());
    assert_eq!(std::fs::read(path).unwrap(), b"external edit");
    assert_eq!(state(&dir), RecordState::Redone);
    let h = History::open_read_only(
        Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist")).unwrap(),
    )
    .unwrap();
    assert!(h.pending_operations().unwrap().is_empty());
}
#[test]
fn dry_run_tag_fix_leaves_audio_and_history_untouched() {
    let dir = setup();
    let path = dir.path().join("song.mp3");
    let before = std::fs::read(&path).unwrap();
    success(run(&dir, &["--dry-run", "validate", "characters", "--fix"]));
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert!(!dir.path().join("config/tfmt.hist").exists());
}

#[test]
fn recovers_each_durable_switch_boundary_and_reports_pending_work_read_only() {
    use tfmttools_core::action::{Action, TagValueChange, TagValueKind};
    use tfmttools_core::history::{
        ActionRecordMetadata, OperationKind, RecoveryDescriptor, StoredAction,
        TemplateMetadata,
    };
    use tfmttools_fs::{
        cleanup_completed_artifacts, install_prepared, prepare_tag_edit,
    };
    for stage in 0..6 {
        let dir = setup();
        let path = Utf8PathBuf::try_from(dir.path().join("song.mp3")).unwrap();
        let history_path =
            Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist")).unwrap();
        let mut h = History::new(history_path.clone());
        h.load().unwrap();
        let changes = vec![TagValueChange::new(
            "track_title".into(),
            TagValueKind::Text,
            "Old ? title".into(),
            "Recorded title".into(),
        )];
        let action = Action::EditTagValues {
            path: path.clone(),
            changes: changes.clone(),
        };
        let id = h
            .begin_operation(
                OperationKind::Apply,
                None,
                Some(ActionRecordMetadata::new(
                    TemplateMetadata::Validation {
                        value: "interrupted".into(),
                    },
                    vec![],
                    "recovery".into(),
                )),
            )
            .unwrap();
        h.set_operation_plan(id, vec![StoredAction::from(&action)]).unwrap();
        let entry = prepare_tag_edit(&path, &changes).unwrap();
        h.append_prepared(id, entry.clone()).unwrap();
        let RecoveryDescriptor::FileSwitch {
            resolved,
            retained,
            candidate,
            ..
        } = &entry.recovery
        else {
            panic!()
        };
        let after = std::fs::read(candidate).unwrap();
        if stage == 1 {
            std::fs::rename(resolved, retained).unwrap();
        }
        if stage >= 2 {
            install_prepared(&entry).unwrap();
        }
        if stage >= 3 {
            h.complete_action(id, 0).unwrap();
        }
        if stage >= 4 {
            h.finish_operation(id).unwrap();
        }
        if stage >= 5 {
            cleanup_completed_artifacts(&entry).unwrap();
        }
        drop(h);
        let before_db = std::fs::read(&history_path).unwrap();
        let shown = run(&dir, &["show-history"]);
        assert!(shown.status.success());
        assert!(
            String::from_utf8_lossy(&shown.stdout).contains("Pending recovery")
        );
        assert_eq!(std::fs::read(&history_path).unwrap(), before_db);
        assert!(!run(&dir, &["clear-history"]).status.success());
        success(run(&dir, &["validate", "characters", "--fix"]));
        assert_eq!(std::fs::read(&path).unwrap(), after, "stage {stage}");
        let h = History::open_read_only(history_path).unwrap();
        assert!(h.pending_operations().unwrap().is_empty());
        assert_eq!(h.records().len(), 1);
        assert!(!std::path::Path::new(retained).exists());
    }
}

#[test]
fn already_utf16_id3_fix_is_a_noop() {
    let dir = setup();
    let path = dir.path().join("song.mp3");
    let mut tag = Id3v2Tag::default();
    tag.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new("TIT2").unwrap(),
        TextEncoding::UTF16,
        "Ä title",
    )));
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    let before = std::fs::read(&path).unwrap();
    success(run(&dir, &["validate", "id3-encoding", "--fix"]));
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert!(!dir.path().join("config/tfmt.hist").exists());
}
