use std::process::{Command, Output};

use assert_fs::TempDir;
use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame};
use lofty::tag::TagExt;

const TAG_HISTORY: &str =
    include_str!("../../../tests/fixtures/cli/history/pre-canonical-tags.json");
const NO_ENCODINGS: &str =
    include_str!("../../../tests/fixtures/cli/history/no-encodings.json");
const FILE_HISTORY: &str =
    include_str!("../../../tests/fixtures/cli/history/legacy-filesystem.json");
const INVALID: &str = include_str!(
    "../../../tests/fixtures/cli/history/invalid-late-change.json"
);

fn setup(history: &str) -> TempDir {
    let directory = TempDir::new().unwrap();
    std::fs::create_dir(directory.path().join("config")).unwrap();
    std::fs::write(directory.path().join("config/tfmt.hist"), history).unwrap();
    directory
}

fn run(directory: &TempDir, arguments: &[&str]) -> Output {
    let binary = std::env::var_os("TFMT_HISTORY_COMPAT_BINARY")
        .unwrap_or_else(|| env!("CARGO_BIN_EXE_tfmt").into());
    Command::new(binary)
        .current_dir(directory.path())
        .args(["--simple", "--yes", "--config-directory", "config"])
        .args(arguments)
        .output()
        .unwrap()
}

fn audio(
    directory: &TempDir,
    frame: &'static str,
    value: &str,
    encoding: TextEncoding,
) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3");
    let path = directory.path().join("song.mp3");
    std::fs::copy(fixture, &path).unwrap();
    let mut tag = Id3v2Tag::default();
    tag.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new(frame).unwrap(),
        encoding,
        value.to_owned(),
    )));
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
}

#[test]
fn rejects_all_old_json_histories_without_importing_or_changing_audio() {
    for history in [
        TAG_HISTORY,
        NO_ENCODINGS,
        FILE_HISTORY,
        INVALID,
        r#"{"schema_version":1,"records":[]}"#,
    ] {
        for args in [
            vec!["show-history"],
            vec!["undo"],
            vec!["redo"],
            vec!["rename", "--script", "path: (\"Renamed\")"],
            vec!["validate", "characters", "--fix"],
            vec!["validate", "id3-encoding", "--fix"],
        ] {
            let directory = setup(history);
            audio(&directory, "TPE1", "Old ? Ärtist", TextEncoding::UTF8);
            let before =
                std::fs::read(directory.path().join("song.mp3")).unwrap();
            let output = run(&directory, &args);
            assert!(!output.status.success(), "{args:?}");
            let error = String::from_utf8_lossy(&output.stderr);
            assert!(
                error.contains("JSON") && error.contains("unsupported"),
                "{error}"
            );
            assert_eq!(
                std::fs::read(directory.path().join("song.mp3")).unwrap(),
                before
            );
            assert_eq!(
                std::fs::read(directory.path().join("config/tfmt.hist"))
                    .unwrap(),
                history.as_bytes()
            );
            assert!(!directory.path().join("config/tfmt.hist.v0.bak").exists());
        }
    }
}
