//! Shared interpretation of native ID3 text frames.
use lofty::id3::v2::{Frame, Id3v2Tag};
use lofty::tag::{ItemKey, TagType};

use crate::tag_encoding::TagTextEncoding;

#[must_use]
pub fn frame_matches_item_key(frame: &Frame, key: ItemKey) -> bool {
    let Some(id) = key.map_key(TagType::Id3v2) else {
        return false;
    };
    match frame {
        Frame::Text(frame) => frame.id().as_str() == id,
        Frame::UserText(frame) => {
            frame.description.as_ref() == id
                || ItemKey::from_key(TagType::Id3v2, &frame.description)
                    == Some(key)
        },
        _ => false,
    }
}

pub fn text_encodings_for_key(
    tag: &Id3v2Tag,
    key: ItemKey,
) -> impl Iterator<Item = TagTextEncoding> + '_ {
    tag.into_iter()
        .filter(move |frame| frame_matches_item_key(frame, key))
        .filter_map(|frame| {
            match frame {
                Frame::Text(frame) => Some(frame.encoding.into()),
                Frame::UserText(frame) => Some(frame.encoding.into()),
                _ => None,
            }
        })
}
