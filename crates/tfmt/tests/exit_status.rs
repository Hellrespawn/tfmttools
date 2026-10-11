use std::process::Command;

use assert_fs::TempDir;
use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame};
use lofty::tag::TagExt;

#[test]
fn success_findings_and_operational_errors_have_distinct_diagnostics() {
    let dir = TempDir::new().unwrap();
    let audio = dir.path().join("song.mp3");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
        &audio,
    )
    .unwrap();
    let mut tag = Id3v2Tag::default();
    tag.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new("TIT2").unwrap(),
        TextEncoding::UTF8,
        "Bad ? title",
    )));
    tag.save_to_path(&audio, WriteOptions::default()).unwrap();
    for (args, status) in [
        (vec!["history"], 0),
        (vec!["validate", "characters"], 1),
        (vec!["validate", "--fix"], 1),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
            .current_dir(dir.path())
            .args(["--simple", "--yes", "-c", "config"])
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(status), "{args:?}: {output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if args == ["validate", "characters"] {
            assert!(stdout.contains("Bad ? title"), "{stdout}");
            assert!(stderr.is_empty(), "{stderr}");
            assert!(!dir.path().join("config/tfmt.hist").exists());
        } else if args == ["validate", "--fix"] {
            assert!(
                stderr.contains("--fix requires a validation type"),
                "{stderr}"
            );
            #[cfg(not(feature = "debug"))]
            assert!(stderr.contains("Usage: validate"), "{stderr}");
        }
    }
}

#[test]
fn check_only_validation_does_not_open_invalid_history() {
    let dir = TempDir::new().unwrap();
    std::fs::create_dir(dir.path().join("config")).unwrap();
    let history = dir.path().join("config/tfmt.hist");
    std::fs::write(&history, b"not a history database").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
        .current_dir(dir.path())
        .args(["--simple", "-c", "config", "validate"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(std::fs::read(history).unwrap(), b"not a history database");
    assert!(!dir.path().join("config/tfmt.hist.lock").exists());
}
