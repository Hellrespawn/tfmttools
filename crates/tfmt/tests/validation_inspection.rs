use std::process::{Command, Output};

use assert_fs::TempDir;
use camino::Utf8PathBuf;
use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame};
use lofty::tag::TagExt;
use tfmttools_core::history::History;

fn run(dir: &TempDir, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tfmt"))
        .current_dir(dir.path())
        .args(["--simple", "--yes", "-c", "config"])
        .args(args)
        .output()
        .unwrap()
}

fn setup(title: &str, encoding: TextEncoding) -> TempDir {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("song.mp3");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
        &path,
    )
    .unwrap();
    let mut tag = Id3v2Tag::default();
    tag.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new("TIT2").unwrap(),
        encoding,
        title.to_owned(),
    )));
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    dir
}

#[test]
fn malformed_audio_blocks_all_fixes() {
    for scope in ["characters", "id3-encoding"] {
        for dry_run in [false, true] {
            let dir = setup("Bad ? Ω title", TextEncoding::UTF8);
            let song = dir.path().join("song.mp3");
            let before = std::fs::read(&song).unwrap();
            let broken = dir.path().join("broken.mp3");
            std::fs::write(&broken, b"invalid audio").unwrap();
            let mut args = vec!["validate", scope, "--fix"];
            if dry_run {
                args.insert(0, "--dry-run");
            }
            let output = run(&dir, &args);
            assert!(!output.status.success(), "{output:?}");
            let report = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(report.contains("broken.mp3"), "{report}");
            assert!(!report.contains("No tag values changed"), "{report}");
            assert!(!report.contains("Updated "), "{report}");
            assert_eq!(std::fs::read(song).unwrap(), before);
            assert_eq!(std::fs::read(broken).unwrap(), b"invalid audio");
            let history = dir.path().join("config/tfmt.hist");
            if history.exists() {
                let h = History::open_read_only(
                    Utf8PathBuf::try_from(history).unwrap(),
                )
                .unwrap();
                assert!(h.records().is_empty());
                assert!(h.current_attempt().unwrap().is_none());
            }
        }
    }
}

#[test]
fn check_and_fix_agree_on_findings() {
    for (scope, title, encoding, needs_fix) in [
        ("characters", "Bad ? title", TextEncoding::UTF8, true),
        ("characters", "Clean title", TextEncoding::UTF8, false),
        ("id3-encoding", "Ω title", TextEncoding::UTF8, true),
        ("id3-encoding", "ASCII title", TextEncoding::UTF8, false),
        ("id3-encoding", "Ω title", TextEncoding::UTF16, false),
    ] {
        let dir = setup(title, encoding);
        let before = std::fs::read(dir.path().join("song.mp3")).unwrap();
        let check = run(&dir, &["validate", scope]);
        assert_eq!(check.status.success(), !needs_fix, "{check:?}");
        let fix = run(&dir, &["validate", scope, "--fix"]);
        assert!(fix.status.success(), "{fix:?}");
        assert_eq!(
            std::fs::read(dir.path().join("song.mp3")).unwrap() != before,
            needs_fix
        );
        assert!(run(&dir, &["validate", scope]).status.success());
    }
}

#[test]
fn characters_only_does_not_require_native_inspection() {
    use lofty::file::{AudioFile, TaggedFileExt};
    use lofty::tag::ItemKey;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("song.m4a");
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
            "../../tests/fixtures/cli/audio/Van Pletzen - Benaaihilism.m4a",
        ),
        &path,
    )
    .unwrap();
    let mut file = lofty::read_from_path(&path).unwrap();
    file.primary_tag_mut()
        .unwrap()
        .insert_text(ItemKey::TrackTitle, "Bad ? title".into());
    file.save_to_path(&path, WriteOptions::default()).unwrap();
    let before = std::fs::read(&path).unwrap();
    let output = run(&dir, &["validate", "characters", "--fix"]);
    assert!(output.status.success(), "{output:?}");
    assert!(run(&dir, &["validate", "characters"]).status.success());
    assert_ne!(std::fs::read(dir.path().join("song.m4a")).unwrap(), before);
}

#[test]
fn mp3_without_native_id3v2_needs_no_encoding_fix() {
    let dir = setup("Title", TextEncoding::UTF8);
    let path = dir.path().join("song.mp3");
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&bytes[..3], b"ID3");
    let tag_size = bytes[6..10]
        .iter()
        .fold(0usize, |size, byte| (size << 7) | usize::from(*byte));
    let mut audio = bytes[10 + tag_size..].to_vec();
    let mut legacy = [0u8; 128];
    legacy[..3].copy_from_slice(b"TAG");
    legacy[3..14].copy_from_slice(b"ASCII title");
    audio.extend_from_slice(&legacy);
    std::fs::write(&path, audio).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(run(&dir, &["validate", "id3-encoding"]).status.success());
    assert!(run(&dir, &["validate", "id3-encoding", "--fix"]).status.success());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn character_inspection_preserves_locator_value_kind() {
    use lofty::file::TaggedFileExt;
    use lofty::id3::v2::UrlLinkFrame;
    use lofty::tag::ItemKey;
    let dir = setup("Title", TextEncoding::UTF8);
    let path = dir.path().join("song.mp3");
    let mut tag = Id3v2Tag::default();
    tag.insert(Frame::Url(UrlLinkFrame::new(
        FrameId::new("WOAF").unwrap(),
        "bad?url",
    )));
    tag.save_to_path(&path, WriteOptions::default()).unwrap();
    assert!(!run(&dir, &["validate", "characters"]).status.success());
    assert!(run(&dir, &["validate", "characters", "--fix"]).status.success());
    let file = lofty::read_from_path(&path).unwrap();
    assert!(file.primary_tag().unwrap().items().any(|item| {
        item.key() == ItemKey::AudioFileUrl
            && item.value().locator() == Some("badurl")
    }));
}
