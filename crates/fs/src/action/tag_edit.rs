use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::id3::v2::{
    ExtendedUrlFrame, Frame, FrameId, Id3v2Tag, UrlLinkFrame,
};
use lofty::tag::{ItemKey, ItemValue, TagExt, TagItem, TagType};
use tfmttools_core::action::{TagValueChange, TagValueKind};
use tfmttools_core::id3::{frame_matches_item_key, text_encodings_for_key};
use tfmttools_core::item_keys::parse_item_key;
use tfmttools_core::tag_encoding::TagTextEncoding;

use crate::error::{FsError, FsResult};
use crate::read_native_id3v2;

#[allow(clippy::too_many_lines)] // Verify every requested value and native encoding before install.
pub fn write_tag_candidate(
    path: &camino::Utf8Path,
    changes: &[TagValueChange],
) -> FsResult {
    let mut tagged_file = lofty::read_from_path(path)
        .map_err(|err| FsError::Lofty(path.to_owned(), err))?;
    let tag = tagged_file.primary_tag_mut().ok_or_else(|| {
        tfmttools_core::error::TFMTError::NoPrimaryTag(path.to_owned())
    })?;

    for change in changes {
        let key = parse_item_key(change.key())?;
        let expected = change.old_value();
        if !tag
            .items()
            .any(|item| tag_item_matches(item, key, change.kind(), expected))
        {
            return Err(FsError::Execution(format!(
                "Requested source tag value missing: {}",
                change.key()
            )));
        }
    }
    for change in changes {
        apply_tag_change(tag, change)?;
    }
    let id3v2_tag_with_encoding_changes =
        tag_with_encoding_changes(tag, changes)?;

    tagged_file
        .save_to_path(path, WriteOptions::default())
        .map_err(|err| FsError::LoftyFileWrite(path.to_owned(), err))?;

    if let Some(id3v2_tag) = id3v2_tag_with_encoding_changes {
        id3v2_tag
            .save_to_path(path, WriteOptions::default())
            .map_err(|err| FsError::LoftyTagWrite(path.to_owned(), err))?;
    }

    let written = lofty::read_from_path(path)
        .map_err(|e| FsError::Lofty(path.to_owned(), e))?;
    let written_tag = written.primary_tag().ok_or_else(|| {
        FsError::Execution("Written candidate has no primary tag".into())
    })?;
    let native_tag =
        if changes.iter().any(|change| change.new_encoding().is_some()) {
            Some(read_native_id3v2(path, written.file_type())?.ok_or_else(
                || {
                    FsError::Execution(
                        "Cannot verify native ID3 encoding for this format"
                            .into(),
                    )
                },
            )?)
        } else {
            None
        };
    for change in changes {
        let key = parse_item_key(change.key())?;
        let (value, encoding) = (change.new_value(), change.new_encoding());
        if !written_tag
            .items()
            .any(|item| tag_item_matches(item, key, change.kind(), value))
        {
            return Err(FsError::Execution(format!(
                "Candidate tag verification failed: {}",
                change.key()
            )));
        }
        if let Some(encoding) = encoding {
            verify_native_encoding(
                native_tag
                    .as_ref()
                    .expect("Encoding requested native inspection"),
                key,
                encoding,
            )?;
        }
    }
    Ok(())
}

fn verify_native_encoding(
    tag: &Id3v2Tag,
    key: ItemKey,
    expected: TagTextEncoding,
) -> FsResult {
    let mut encodings = text_encodings_for_key(tag, key).peekable();
    if encodings.peek().is_none() {
        return Err(FsError::Execution(
            "Candidate encoding frame missing".into(),
        ));
    }
    if encodings.any(|actual| actual != expected) {
        return Err(FsError::Execution(
            "Candidate encoding verification failed".into(),
        ));
    }
    Ok(())
}

fn apply_tag_change(
    tag: &mut lofty::tag::Tag,
    change: &TagValueChange,
) -> FsResult {
    let key = parse_item_key(change.key())?;
    let (from, to) = (change.old_value(), change.new_value());
    let mut replacements = tag
        .take_filter(key, |item| {
            tag_item_matches(item, key, change.kind(), from)
        })
        .map(|item| replacement_item(&item, change.kind(), to.to_owned()))
        .collect::<Vec<_>>();

    for item in replacements.drain(..) {
        tag.push(item);
    }

    Ok(())
}

fn tag_with_encoding_changes(
    tag: &lofty::tag::Tag,
    changes: &[TagValueChange],
) -> FsResult<Option<Id3v2Tag>> {
    if tag.tag_type() != TagType::Id3v2 {
        return Ok(None);
    }

    let mut id3v2_tag = Id3v2Tag::from(tag.clone());
    let mut encoding_changed = false;
    // Lofty's generic ID3 conversion reads URL values as text even though
    // parsing produces Locator values. Preserve them through native frames.
    for item in tag.items() {
        if let Some(value) = item.value().locator()
            && let Some(id) = item.key().map_key(TagType::Id3v2)
        {
            if id == "WXXX" {
                id3v2_tag.insert(Frame::UserUrl(ExtendedUrlFrame::new(
                    TextEncoding::UTF8,
                    item.description().to_owned(),
                    value.to_owned(),
                )));
            } else if id.starts_with('W') && id.len() == 4 {
                let frame_id = FrameId::new(id)
                    .map_err(|e| FsError::Execution(e.to_string()))?;
                id3v2_tag.insert(Frame::Url(UrlLinkFrame::new(
                    frame_id,
                    value.to_owned(),
                )));
            } else {
                return Err(FsError::Execution(
                    "Unsupported ID3 locator mapping".into(),
                ));
            }
            encoding_changed = true;
        }
    }

    for change in changes {
        let Some(encoding) = change.new_encoding() else {
            continue;
        };
        let encoding = TextEncoding::from(encoding);
        let key = parse_item_key(change.key())?;
        let frames = id3v2_tag
            .into_iter()
            .map(|frame| {
                if !frame_matches_item_key(&frame, key) {
                    return frame;
                }
                match frame {
                    Frame::Text(mut frame) => {
                        frame.encoding = encoding;
                        encoding_changed = true;
                        Frame::Text(frame)
                    },
                    Frame::UserText(mut frame) => {
                        frame.encoding = encoding;
                        encoding_changed = true;
                        Frame::UserText(frame)
                    },
                    frame => frame,
                }
            })
            .collect::<Vec<_>>();

        id3v2_tag = Id3v2Tag::default();
        for frame in frames {
            id3v2_tag.insert(frame);
        }
    }

    Ok(encoding_changed.then_some(id3v2_tag))
}

fn tag_item_matches(
    item: &TagItem,
    key: ItemKey,
    kind: &TagValueKind,
    value: &str,
) -> bool {
    item.key() == key && item_value(item, kind) == Some(value)
}

fn item_value<'a>(item: &'a TagItem, kind: &TagValueKind) -> Option<&'a str> {
    match kind {
        TagValueKind::Text => item.value().text(),
        TagValueKind::Locator => item.value().locator(),
    }
}

fn replacement_item(
    item: &TagItem,
    kind: &TagValueKind,
    value: String,
) -> TagItem {
    let mut replacement = TagItem::new(item.key(), match kind {
        TagValueKind::Text => ItemValue::Text(value),
        TagValueKind::Locator => ItemValue::Locator(value),
    });

    replacement.set_description(item.description().to_owned());
    replacement.set_lang(*item.lang());

    replacement
}

#[cfg(test)]
mod tests {
    use lofty::id3::v2::ExtendedTextFrame;
    use tfmttools_core::tag_encoding::TagTextEncoding;

    use super::*;

    #[test]
    fn candidate_encoding_requires_every_matching_frame() {
        let mut tag = Id3v2Tag::default();
        tag.insert(Frame::UserText(ExtendedTextFrame::new(
            TextEncoding::UTF16,
            "ALBUMARTIST",
            "Artist",
        )));
        tag.insert(Frame::UserText(ExtendedTextFrame::new(
            TextEncoding::UTF8,
            "ALBUM ARTIST",
            "Artist",
        )));
        assert!(
            verify_native_encoding(
                &tag,
                ItemKey::AlbumArtist,
                TagTextEncoding::UTF16
            )
            .is_err()
        );
        assert!(
            verify_native_encoding(
                &tag,
                ItemKey::TrackTitle,
                TagTextEncoding::UTF16
            )
            .is_err()
        );
        tag.insert(Frame::UserText(ExtendedTextFrame::new(
            TextEncoding::UTF16,
            "ALBUM ARTIST",
            "Artist",
        )));
        assert!(
            verify_native_encoding(
                &tag,
                ItemKey::AlbumArtist,
                TagTextEncoding::UTF16
            )
            .is_ok()
        );
    }
}
