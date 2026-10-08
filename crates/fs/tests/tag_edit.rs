use camino::Utf8PathBuf;
use lofty::file::TaggedFileExt;
use lofty::tag::ItemKey;
use tfmttools_core::action::{TagValueChange, TagValueKind};
use tfmttools_fs::write_tag_candidate;

#[test]
fn verifies_written_text_and_rejects_missing_source() {
    let dir = tempfile::tempdir().unwrap();
    let path =
        Utf8PathBuf::from_path_buf(dir.path().join("candidate.mp3")).unwrap();
    std::fs::copy("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3", &path)
        .unwrap();
    let original = std::fs::read(&path).unwrap();
    let file = lofty::read_from_path(&path).unwrap();
    let title =
        file.primary_tag().unwrap().get_string(ItemKey::TrackTitle).unwrap();
    let change = TagValueChange::new(
        "track_title".into(),
        TagValueKind::Text,
        title.into(),
        "Verified title".into(),
    );
    write_tag_candidate(&path, &[change]).unwrap();
    let file = lofty::read_from_path(&path).unwrap();
    assert_eq!(
        file.primary_tag().unwrap().get_string(ItemKey::TrackTitle),
        Some("Verified title")
    );
    let pair = tfmttools_fs::create_patch_pair(
        &original,
        &std::fs::read(&path).unwrap(),
    )
    .unwrap();
    assert_eq!(
        tfmttools_fs::apply_patch(
            &std::fs::read(&path).unwrap(),
            &pair,
            tfmttools_core::history::HistoryMode::Undo
        )
        .unwrap(),
        original
    );
    assert!(
        write_tag_candidate(&path, &[TagValueChange::new(
            "track_title".into(),
            TagValueKind::Text,
            "missing".into(),
            "other".into()
        )])
        .is_err()
    );
}

#[test]
fn verifies_locator_and_id3_encoding_changes() {
    use lofty::config::WriteOptions;
    use lofty::file::AudioFile as _;
    use lofty::tag::{ItemValue, TagExt, TagItem};
    let dir = tempfile::tempdir().unwrap();
    let path =
        Utf8PathBuf::from_path_buf(dir.path().join("candidate.mp3")).unwrap();
    std::fs::copy("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3", &path)
        .unwrap();
    let mut file = lofty::read_from_path(&path).unwrap();
    file.primary_tag_mut().unwrap().push(TagItem::new(
        ItemKey::AudioFileUrl,
        ItemValue::Locator("https://old.example".into()),
    ));
    let title = file
        .primary_tag()
        .unwrap()
        .get_string(ItemKey::TrackTitle)
        .unwrap()
        .to_owned();
    file.save_to_path(&path, WriteOptions::default()).unwrap();
    write_tag_candidate(&path, &[
        TagValueChange::new(
            "track_title".into(),
            TagValueKind::Text,
            title,
            "Ω title".into(),
        )
        .with_encoding(None, Some("UTF16".into())),
        TagValueChange::new(
            "audio_file_url".into(),
            TagValueKind::Locator,
            "https://old.example".into(),
            "https://new.example".into(),
        ),
    ])
    .unwrap();
    let file = lofty::read_from_path(&path).unwrap();
    assert_eq!(
        file.primary_tag().unwrap().get_string(ItemKey::TrackTitle),
        Some("Ω title")
    );
    assert!(file.primary_tag().unwrap().items().any(|i| {
        i.key() == ItemKey::AudioFileUrl
            && i.value().locator() == Some("https://new.example")
    }));
}
