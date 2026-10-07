use std::process::{Command, Output};

use assert_fs::TempDir;
use lofty::TextEncoding;
use lofty::config::{ParseOptions, WriteOptions};
use lofty::file::AudioFile;
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame};
use lofty::mpeg::MpegFile;
use lofty::tag::TagExt;
use serde_json::Value;

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

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn saved(directory: &TempDir) -> Value {
    serde_json::from_slice(
        &std::fs::read(directory.path().join("config/tfmt.hist")).unwrap(),
    )
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

fn text_frame(directory: &TempDir, frame: &str) -> (String, TextEncoding) {
    let mut file =
        std::fs::File::open(directory.path().join("song.mp3")).unwrap();
    let mpeg = MpegFile::read_from(&mut file, ParseOptions::default()).unwrap();
    let Frame::Text(frame) =
        mpeg.id3v2().unwrap().get(&FrameId::new(frame).unwrap()).unwrap()
    else {
        panic!("text frame")
    };
    (frame.value.to_string(), frame.encoding)
}

#[test]
fn pre_canonical_tag_history_undo_redo_restores_values_and_encoding() {
    let directory = setup(TAG_HISTORY);
    audio(&directory, "TPE1", "New artist", TextEncoding::UTF16);
    assert_eq!(
        text_frame(&directory, "TPE1"),
        ("New artist".into(), TextEncoding::UTF16)
    );
    success(&run(&directory, &["undo"]));
    assert_eq!(
        text_frame(&directory, "TPE1"),
        ("Old artist".into(), TextEncoding::UTF8)
    );
    assert_eq!(saved(&directory)["records"][0]["state"], "undone");
    assert_eq!(saved(&directory)["schema_version"], 1);
    assert_eq!(
        saved(&directory)["records"][0]["actions"][0]["changes"][0]["key"],
        "track_artist"
    );
    assert_eq!(
        std::fs::read(directory.path().join("config/tfmt.hist.v0.bak"))
            .unwrap(),
        TAG_HISTORY.as_bytes()
    );
    success(&run(&directory, &["redo"]));
    assert_eq!(
        text_frame(&directory, "TPE1"),
        ("New artist".into(), TextEncoding::UTF16)
    );
    assert_eq!(saved(&directory)["records"][0]["state"], "redone");
}

#[test]
fn legacy_missing_encodings_replay_without_inventing_storage_information() {
    let directory = setup(NO_ENCODINGS);
    audio(&directory, "TIT2", "New title", TextEncoding::UTF16BE);
    success(&run(&directory, &["undo"]));
    assert_eq!(text_frame(&directory, "TIT2").0, "Old title");
    let document = saved(&directory);
    let change = &document["records"][0]["actions"][0]["changes"][0];
    assert!(change["old_encoding"].is_null());
    assert!(change["new_encoding"].is_null());
    success(&run(&directory, &["redo"]));
    assert_eq!(text_frame(&directory, "TIT2").0, "New title");
}

#[test]
fn legacy_filesystem_actions_preserve_replay_order() {
    let directory = setup(FILE_HISTORY);
    std::fs::write(directory.path().join("result.txt"), b"original bytes")
        .unwrap();
    success(&run(&directory, &["undo", "2"]));
    assert_eq!(
        std::fs::read(directory.path().join("original.txt")).unwrap(),
        b"original bytes"
    );
    assert!(!directory.path().join("result.txt").exists());
    assert!(!directory.path().join("stage").exists());
    assert_eq!(saved(&directory)["records"][0]["state"], "undone");
    assert_eq!(saved(&directory)["records"][1]["state"], "undone");
    success(&run(&directory, &["redo", "2"]));
    assert_eq!(
        std::fs::read(directory.path().join("result.txt")).unwrap(),
        b"original bytes"
    );
    assert!(!directory.path().join("original.txt").exists());
    assert!(!directory.path().join("stage").exists());
    assert_eq!(
        std::fs::read(directory.path().join("config/tfmt.hist.v0.bak"))
            .unwrap(),
        FILE_HISTORY.as_bytes()
    );
}

#[test]
fn show_history_is_read_only_for_v0_and_counts_stored_actions() {
    let fixture =
        include_str!("../../core/tests/fixtures/history/v0-all-variants.json");
    let directory = setup(fixture);
    let output = run(&directory, &["show-history"]);
    success(&output);
    let text = String::from_utf8_lossy(&output.stdout);
    for count in [
        "file moved",
        "file copied",
        "file removed",
        "directory created",
        "directory removed",
        "tag edited",
    ] {
        assert!(text.contains(count), "{text}");
    }
    assert_eq!(
        std::fs::read(directory.path().join("config/tfmt.hist")).unwrap(),
        fixture.as_bytes()
    );
    assert!(!directory.path().join("config/tfmt.hist.v0.bak").exists());
}

#[test]
fn invalid_history_stops_commands_before_actions() {
    for future in [false, true] {
        for command in [
            vec!["undo"],
            vec!["redo"],
            vec!["rename", "--script", "path: (\"Renamed\")"],
            vec!["validate", "characters", "--fix"],
            vec!["validate", "id3-encoding", "--fix"],
        ] {
            let history = if future {
                "{\"schema_version\":99,\"records\":[]}"
            } else {
                INVALID
            };
            let directory = setup(history);
            audio(&directory, "TPE1", "New ? Ärtist", TextEncoding::UTF8);
            std::fs::write(
                directory.path().join("result.txt"),
                b"original bytes",
            )
            .unwrap();
            let original_audio =
                std::fs::read(directory.path().join("song.mp3")).unwrap();
            let output = run(&directory, &command);
            assert!(
                !output.status.success(),
                "{command:?}: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            assert!(
                std::fs::read(directory.path().join("song.mp3")).unwrap()
                    == original_audio,
                "audio changed: {command:?}"
            );
            assert_eq!(
                std::fs::read(directory.path().join("result.txt")).unwrap(),
                b"original bytes"
            );
            assert_eq!(
                std::fs::read(directory.path().join("config/tfmt.hist"))
                    .unwrap(),
                history.as_bytes()
            );
            assert!(!directory.path().join("original.txt").exists());
            assert!(!directory.path().join("Renamed.mp3").exists());
        }
    }
}
