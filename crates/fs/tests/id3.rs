use camino::Utf8PathBuf;
use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::file::FileType;
use lofty::id3::v2::{Frame, FrameId, Id3v2Tag, TextInformationFrame};
use lofty::tag::TagExt;
use tfmttools_fs::read_native_id3v2;

#[test]
fn native_reader_distinguishes_missing_tags_and_failed_reads() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::try_from(dir.path().to_owned()).unwrap();
    let mut tag = Id3v2Tag::default();
    tag.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new("TIT2").unwrap(),
        TextEncoding::UTF8,
        "Title",
    )));
    for (format, name, header) in [
        (FileType::Wav, "audio.wav", b"RIFF\x04\x00\x00\x00WAVE".as_slice()),
        (FileType::Aiff, "audio.aiff", b"FORM\x00\x00\x00\x04AIFF".as_slice()),
    ] {
        let path = root.join(name);
        std::fs::write(&path, header).unwrap();
        assert!(read_native_id3v2(&path, format).unwrap().is_none());
        tag.save_to_path(&path, WriteOptions::default()).unwrap();
        let native = read_native_id3v2(&path, format).unwrap().unwrap();
        assert_eq!(
            native.get_text(&FrameId::new("TIT2").unwrap()),
            Some("Title")
        );
    }
    let mp3 = root.join("audio.mp3");
    std::fs::copy("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3", &mp3)
        .unwrap();
    assert!(read_native_id3v2(&mp3, FileType::Mpeg).unwrap().is_some());
    let broken = root.join("broken");
    std::fs::write(&broken, b"invalid").unwrap();
    for format in [FileType::Mpeg, FileType::Wav, FileType::Aiff] {
        assert!(read_native_id3v2(&broken, format).is_err());
    }
    assert!(read_native_id3v2(&root.join("missing"), FileType::Mpeg).is_err());
    assert!(read_native_id3v2(&broken, FileType::Flac).unwrap().is_none());
}
