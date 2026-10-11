#![cfg(unix)]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::process::Command;

use assert_fs::TempDir;
use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame};
use lofty::tag::TagExt;

#[test]
fn incomplete_scan_does_not_rename_or_fix() {
    for args in [vec!["rename", "--script", "path: (\"renamed\")"], vec![
        "validate",
        "characters",
        "--fix",
    ]] {
        let dir = TempDir::new().unwrap();
        let original = dir.path().join("a-song.mp3");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
            &original,
        )
        .unwrap();
        let mut tag = Id3v2Tag::default();
        tag.insert(Frame::Text(TextInformationFrame::new(
            FrameId::new("TIT2").unwrap(),
            TextEncoding::UTF8,
            "Bad ? title",
        )));
        tag.save_to_path(&original, WriteOptions::default()).unwrap();
        let before = std::fs::read(&original).unwrap();
        let invalid =
            dir.path().join(OsString::from_vec(b"z-\xff.mp3".to_vec()));
        std::fs::write(&invalid, b"unreadable name").unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
            .current_dir(dir.path())
            .args(["--simple", "--yes", "-c", "config"])
            .args(&args)
            .output()
            .unwrap();
        assert!(!output.status.success(), "{args:?}: {output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Unable to discover audio files in"),
            "{stderr}"
        );
        assert_eq!(std::fs::read(&original).unwrap(), before);
        assert_eq!(std::fs::read(&invalid).unwrap(), b"unreadable name");
        assert!(!dir.path().join("renamed.mp3").exists());
        let history_path = dir.path().join("config/tfmt.hist");
        if history_path.exists() {
            let history = tfmttools_core::history::History::open_read_only(
                camino::Utf8PathBuf::try_from(history_path).unwrap(),
            )
            .unwrap();
            assert!(history.records().is_empty());
            assert!(history.current_attempt().unwrap().is_none());
        }
    }
}
