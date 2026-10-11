use lofty::TextEncoding;
use tfmttools_core::tag_encoding::TagTextEncoding;

#[test]
fn canonical_names_round_trip_through_lofty() {
    for (name, encoding, lofty) in [
        ("Latin1", TagTextEncoding::Latin1, TextEncoding::Latin1),
        ("UTF16", TagTextEncoding::UTF16, TextEncoding::UTF16),
        ("UTF16BE", TagTextEncoding::UTF16BE, TextEncoding::UTF16BE),
        ("UTF8", TagTextEncoding::UTF8, TextEncoding::UTF8),
    ] {
        assert_eq!(TagTextEncoding::parse(name), Some(encoding));
        assert_eq!(encoding.name(), name);
        assert_eq!(TextEncoding::from(encoding), lofty);
        assert_eq!(TagTextEncoding::from(lofty), encoding);
    }
}

#[test]
fn parsing_rejects_noncanonical_or_unknown_names() {
    for name in ["latin1", "utf8", "UTF-8", "", "unknown", " UTF8", "UTF8 "] {
        assert_eq!(TagTextEncoding::parse(name), None, "{name:?}");
    }
}

#[test]
fn runtime_encodings_survive_the_stored_boundary() {
    use tfmttools_core::action::{TagValueChange, TagValueKind};
    use tfmttools_core::history::StoredTagValueChange;
    for (encoding, name) in [
        (TagTextEncoding::Latin1, "Latin1"),
        (TagTextEncoding::UTF16, "UTF16"),
        (TagTextEncoding::UTF16BE, "UTF16BE"),
        (TagTextEncoding::UTF8, "UTF8"),
    ] {
        let change = TagValueChange::new(
            "track_title".into(),
            TagValueKind::Text,
            "old".into(),
            "new".into(),
        )
        .with_encoding(Some(encoding), Some(encoding));
        let stored = StoredTagValueChange::from(&change);
        assert_eq!(stored.old_encoding.as_deref(), Some(name));
        assert_eq!(stored.new_encoding.as_deref(), Some(name));
        let restored = TagValueChange::try_from(&stored).unwrap();
        assert_eq!(restored.old_encoding(), Some(encoding));
        assert_eq!(restored.new_encoding(), Some(encoding));
    }
}
