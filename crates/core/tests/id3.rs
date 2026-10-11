use lofty::TextEncoding;
use lofty::id3::v2::{
    ExtendedTextFrame, Frame, FrameId, Id3v2Tag, TextInformationFrame,
    UrlLinkFrame,
};
use lofty::tag::ItemKey;
use tfmttools_core::id3::{frame_matches_item_key, text_encodings_for_key};
use tfmttools_core::tag_encoding::TagTextEncoding;

#[test]
fn text_and_user_text_matching_preserves_all_encodings_in_order() {
    let mut tag = Id3v2Tag::default();
    tag.insert(Frame::Text(TextInformationFrame::new(
        FrameId::new("TPE2").unwrap(),
        TextEncoding::UTF8,
        "Artist",
    )));
    tag.insert(Frame::UserText(ExtendedTextFrame::new(
        TextEncoding::UTF16,
        "ALBUMARTIST",
        "Artist",
    )));
    tag.insert(Frame::UserText(ExtendedTextFrame::new(
        TextEncoding::Latin1,
        "ALBUM ARTIST",
        "Artist",
    )));
    let expected = (&tag)
        .into_iter()
        .filter_map(|frame| {
            match frame {
                Frame::Text(frame) => {
                    Some(TagTextEncoding::from(frame.encoding))
                },
                Frame::UserText(frame) => {
                    Some(TagTextEncoding::from(frame.encoding))
                },
                _ => None,
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(expected.len(), 3);
    assert_eq!(
        text_encodings_for_key(&tag, ItemKey::AlbumArtist).collect::<Vec<_>>(),
        expected
    );
    assert!(text_encodings_for_key(&tag, ItemKey::TrackTitle).next().is_none());
    let by_id = Frame::UserText(ExtendedTextFrame::new(
        TextEncoding::UTF8,
        "TPE2",
        "Artist",
    ));
    assert!(frame_matches_item_key(&by_id, ItemKey::AlbumArtist));
    let url = Frame::Url(UrlLinkFrame::new(
        FrameId::new("WOAF").unwrap(),
        "https://example.com",
    ));
    assert!(!frame_matches_item_key(&url, ItemKey::AudioFileUrl));
    assert!(!frame_matches_item_key(&by_id, ItemKey::TrackTitle));
}
