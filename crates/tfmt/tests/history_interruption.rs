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
fn success(out: &Output) {
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
    success(&run(&dir, &["validate", "characters", "--fix"]));
    let after = std::fs::read(&path).unwrap();
    assert_ne!(before, after);
    success(&run(&dir, &["undo"]));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(state(&dir), RecordState::Undone);
    success(&run(&dir, &["redo"]));
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
    assert!(h.current_attempt().unwrap().is_none());
}
#[test]
fn dry_run_tag_fix_leaves_audio_and_history_untouched() {
    let dir = setup();
    let path = dir.path().join("song.mp3");
    let before = std::fs::read(&path).unwrap();
    success(&run(&dir, &["--dry-run", "validate", "characters", "--fix"]));
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert!(!dir.path().join("config/tfmt.hist").exists());
}

#[test]
fn interrupted_tag_attempt_reports_and_resolves_without_touching_files() {
    use tfmttools_core::action::{TagValueChange, TagValueKind};
    use tfmttools_core::history::{
        ActionRecordMetadata, OperationKind, TemplateMetadata,
    };
    for installed in [false, true] {
        for outcome in ["applied", "not-applied"] {
            let dir = setup();
            let path =
                Utf8PathBuf::try_from(dir.path().join("song.mp3")).unwrap();
            let before = std::fs::read(&path).unwrap();
            let mut prepared =
                tfmttools_fs::prepare_tag_edit(&path, &[TagValueChange::new(
                    "track_title".into(),
                    TagValueKind::Text,
                    "Old ? title".into(),
                    "New title".into(),
                )])
                .unwrap();
            let details = prepared.details().clone();
            let mut h = History::new(
                Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist"))
                    .unwrap(),
            );
            let id = h
                .begin_run(
                    OperationKind::Apply,
                    None,
                    Some(ActionRecordMetadata::new(
                        TemplateMetadata::Validation {
                            value: "characters".into(),
                        },
                        vec!["validate".into()],
                        "interrupted".into(),
                    )),
                )
                .unwrap();
            let attempt = h
                .begin_attempt(
                    id,
                    0,
                    prepared.action(),
                    prepared.details(),
                    prepared.patches(),
                )
                .unwrap();
            prepared.retain_artifacts();
            if installed {
                prepared.execute().unwrap();
            }
            drop(h);
            drop(prepared);
            let expected = std::fs::read(&path).unwrap();
            let artifacts: Vec<_> = details
                .paths
                .iter()
                .map(|p| (p.clone(), std::fs::read(p).ok()))
                .collect();
            let shown = run(&dir, &["show-history"]);
            success(&shown);
            assert!(
                String::from_utf8_lossy(&shown.stdout)
                    .contains("Unresolved attempt")
            );
            for command in [
                vec!["undo"],
                vec!["redo"],
                vec!["clear-history"],
                vec!["validate", "characters", "--fix"],
                vec!["rename", "--script", "path: (\"Renamed\")"],
            ] {
                assert!(!run(&dir, &command).status.success(), "{command:?}");
                assert_eq!(std::fs::read(&path).unwrap(), expected);
            }
            // Deliberately do not repair first: resolution trusts the user's
            // explicit account and must not probe files to choose an outcome.
            success(&run(&dir, &[
                "resolve-history",
                "--attempt",
                &attempt.0.to_string(),
                "--outcome",
                outcome,
            ]));
            assert_eq!(std::fs::read(&path).unwrap(), expected);
            for (p, bytes) in artifacts {
                assert_eq!(std::fs::read(p).ok(), bytes);
            }
            let h = History::open_read_only(
                Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist"))
                    .unwrap(),
            )
            .unwrap();
            assert!(h.current_attempt().unwrap().is_none());
            assert_eq!(
                h.records()[0].applied_count(),
                usize::from(outcome == "applied")
            );
            assert!(!h.records()[0].redo_allowed());
            if !installed {
                assert_eq!(expected, before);
            }
        }
    }
}
#[test]
fn history_resolution_does_not_require_files_to_exist() {
    use tfmttools_core::history::{
        ActionRecordMetadata, AttemptDetails, OperationKind, StoredAction,
        TemplateMetadata,
    };
    let dir = setup();
    let mut h = History::new(
        Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist")).unwrap(),
    );
    let run_id = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(ActionRecordMetadata::new(
                TemplateMetadata::Validation { value: "test".into() },
                vec![],
                "manual".into(),
            )),
        )
        .unwrap();
    let a = h
        .begin_attempt(
            run_id,
            0,
            &StoredAction::MoveFile {
                source: "/missing/source".into(),
                target: "/missing/target".into(),
            },
            &AttemptDetails {
                paths: vec!["/missing/source".into(), "/missing/target".into()],
                instructions: "Check both paths".into(),
            },
            None,
        )
        .unwrap();
    drop(h);
    assert!(
        !run(&dir, &[
            "resolve-history",
            "--attempt",
            &(a.0 + 1).to_string(),
            "--outcome",
            "applied"
        ])
        .status
        .success()
    );
    assert!(
        !run(&dir, &[
            "--dry-run",
            "resolve-history",
            "--attempt",
            &a.0.to_string(),
            "--outcome",
            "applied"
        ])
        .status
        .success()
    );
    success(&run(&dir, &[
        "resolve-history",
        "--attempt",
        &a.0.to_string(),
        "--outcome",
        "not-applied",
    ]));
}
#[test]
fn read_only_history_reports_run_interrupted_between_actions() {
    use tfmttools_core::history::{
        ActionRecordMetadata, AttemptDetails, OperationKind, StoredAction,
        TemplateMetadata,
    };
    let dir = setup();
    let path =
        Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist")).unwrap();
    let mut h = History::new(path.clone());
    let id = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(ActionRecordMetadata::new(
                TemplateMetadata::Validation { value: "test".into() },
                vec![],
                "between".into(),
            )),
        )
        .unwrap();
    let a = h
        .begin_attempt(
            id,
            0,
            &StoredAction::MakeDir { path: "test".into() },
            &AttemptDetails {
                paths: vec!["test".into()],
                instructions: "Inspect".into(),
            },
            None,
        )
        .unwrap();
    h.confirm_attempt(a).unwrap();
    drop(h);
    let before = std::fs::read(&path).unwrap();
    let out = run(&dir, &["show-history"]);
    success(&out);
    assert!(String::from_utf8_lossy(&out.stdout).contains("Interrupted run"));
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[test]
fn dry_run_validation_reports_unresolved_history_without_writes() {
    use tfmttools_core::history::{
        ActionRecordMetadata, AttemptDetails, OperationKind, StoredAction,
        TemplateMetadata,
    };
    let dir = setup();
    let path =
        Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist")).unwrap();
    let mut h = History::new(path.clone());
    let run_id = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(ActionRecordMetadata::new(
                TemplateMetadata::Validation { value: "test".into() },
                vec![],
                "manual".into(),
            )),
        )
        .unwrap();
    h.begin_attempt(
        run_id,
        0,
        &StoredAction::RemoveFile { path: "missing".into() },
        &AttemptDetails {
            paths: vec!["missing".into()],
            instructions: "Inspect".into(),
        },
        None,
    )
    .unwrap();
    drop(h);
    let before = std::fs::read(&path).unwrap();
    let out = run(&dir, &["--dry-run", "validate", "characters", "--fix"]);
    success(&out);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Unresolved attempt")
    );
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[test]
fn interruption_during_tag_switch_preserves_backup_and_missing_original_path() {
    use tfmttools_core::action::{TagValueChange, TagValueKind};
    use tfmttools_core::history::{
        ActionRecordMetadata, OperationKind, TemplateMetadata,
    };
    let dir = setup();
    let path = Utf8PathBuf::try_from(dir.path().join("song.mp3")).unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut p = tfmttools_fs::prepare_tag_edit(&path, &[TagValueChange::new(
        "track_title".into(),
        TagValueKind::Text,
        "Old ? title".into(),
        "New title".into(),
    )])
    .unwrap();
    let details = p.details().clone();
    let mut h = History::new(
        Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist")).unwrap(),
    );
    let id = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(ActionRecordMetadata::new(
                TemplateMetadata::Validation { value: "test".into() },
                vec![],
                "switch".into(),
            )),
        )
        .unwrap();
    h.begin_attempt(id, 0, p.action(), p.details(), p.patches()).unwrap();
    p.retain_artifacts();
    std::fs::rename(&details.paths[1], &details.paths[3]).unwrap();
    drop(p);
    drop(h);
    success(&run(&dir, &["show-history"]));
    assert!(!run(&dir, &["undo"]).status.success());
    assert!(!path.exists());
    assert_eq!(std::fs::read(&details.paths[3]).unwrap(), before);
    assert!(std::path::Path::new(&details.paths[2]).exists());
}
#[test]
fn interruption_after_tag_confirmation_leaves_backup_as_manual_housekeeping() {
    use tfmttools_core::action::{TagValueChange, TagValueKind};
    use tfmttools_core::history::{
        ActionRecordMetadata, OperationKind, TemplateMetadata,
    };
    let dir = setup();
    let path = Utf8PathBuf::try_from(dir.path().join("song.mp3")).unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut p = tfmttools_fs::prepare_tag_edit(&path, &[TagValueChange::new(
        "track_title".into(),
        TagValueKind::Text,
        "Old ? title".into(),
        "New title".into(),
    )])
    .unwrap();
    let backup = p.details().paths.last().unwrap().clone();
    let mut h = History::new(
        Utf8PathBuf::try_from(dir.path().join("config/tfmt.hist")).unwrap(),
    );
    let id = h
        .begin_run(
            OperationKind::Apply,
            None,
            Some(ActionRecordMetadata::new(
                TemplateMetadata::Validation { value: "test".into() },
                vec![],
                "cleanup".into(),
            )),
        )
        .unwrap();
    let a =
        h.begin_attempt(id, 0, p.action(), p.details(), p.patches()).unwrap();
    p.execute().unwrap();
    h.confirm_attempt(a).unwrap();
    drop(p);
    drop(h);
    let after = std::fs::read(&path).unwrap();
    let shown = run(&dir, &["show-history"]);
    success(&shown);
    assert!(
        !String::from_utf8_lossy(&shown.stdout).contains("Unresolved attempt")
    );
    assert_eq!(std::fs::read(&backup).unwrap(), before);
    assert_eq!(std::fs::read(&path).unwrap(), after);
    success(&run(&dir, &["undo"]));
    assert_eq!(std::fs::read(path).unwrap(), before);
    assert_eq!(std::fs::read(&backup).unwrap(), before);
}
