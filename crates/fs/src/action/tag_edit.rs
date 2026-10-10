use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile as LoftyAudioFile, TaggedFileExt};
use lofty::id3::v2::{
    ExtendedUrlFrame, Frame, FrameId, Id3v2Tag, UrlLinkFrame,
};
use lofty::tag::{ItemKey, ItemValue, TagExt, TagItem, TagType};
use tfmttools_core::action::{TagValueChange, TagValueKind};
use tfmttools_core::item_keys::parse_item_key;

use crate::error::{FsError, FsResult};

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
            let id = key.map_key(TagType::Id3v2).ok_or_else(|| {
                FsError::Execution("Requested encoding has no ID3 frame".into())
            })?;
            let mut file = std::fs::File::open(path)?;
            let options = lofty::config::ParseOptions::new();
            let frames = match written.file_type() {
                lofty::file::FileType::Mpeg => {
                    lofty::mpeg::MpegFile::read_from(&mut file, options)
                        .map_err(|e| FsError::Lofty(path.to_owned(), e))?
                        .id3v2()
                        .cloned()
                },
                lofty::file::FileType::Wav => {
                    lofty::iff::wav::WavFile::read_from(&mut file, options)
                        .map_err(|e| FsError::Lofty(path.to_owned(), e))?
                        .id3v2()
                        .cloned()
                },
                lofty::file::FileType::Aiff => {
                    lofty::iff::aiff::AiffFile::read_from(&mut file, options)
                        .map_err(|e| FsError::Lofty(path.to_owned(), e))?
                        .id3v2()
                        .cloned()
                },
                _ => None,
            }
            .ok_or_else(|| {
                FsError::Execution(
                    "Cannot verify native ID3 encoding for this format".into(),
                )
            })?;
            let expected =
                text_encoding_from_name(encoding).ok_or_else(|| {
                    FsError::Execution("Unknown requested encoding".into())
                })?;
            let mut found = false;
            for frame in frames {
                let actual = match frame {
                    Frame::Text(frame) if frame.id().as_str() == id => {
                        Some(frame.encoding)
                    },
                    Frame::UserText(frame)
                        if frame.description.as_ref() == id
                            || ItemKey::from_key(
                                TagType::Id3v2,
                                &frame.description,
                            ) == Some(key) =>
                    {
                        Some(frame.encoding)
                    },
                    _ => None,
                };
                if let Some(actual) = actual {
                    found = true;
                    if actual != expected {
                        return Err(FsError::Execution(
                            "Candidate encoding verification failed".into(),
                        ));
                    }
                }
            }
            if !found {
                return Err(FsError::Execution(
                    "Candidate encoding frame missing".into(),
                ));
            }
        }
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
        let Some(encoding) = text_encoding_from_name(encoding) else {
            continue;
        };
        let key = parse_item_key(change.key())?;
        let Some(id) = key.map_key(TagType::Id3v2) else {
            continue;
        };

        let frames = id3v2_tag
            .into_iter()
            .map(|frame| {
                match frame {
                    Frame::Text(mut frame) if frame.id().as_str() == id => {
                        frame.encoding = encoding;
                        encoding_changed = true;
                        Frame::Text(frame)
                    },
                    Frame::UserText(mut frame)
                        if frame.description.as_ref() == id
                            || ItemKey::from_key(
                                TagType::Id3v2,
                                &frame.description,
                            ) == Some(key) =>
                    {
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

fn text_encoding_from_name(encoding: &str) -> Option<TextEncoding> {
    match encoding {
        "Latin1" => Some(TextEncoding::Latin1),
        "UTF16" => Some(TextEncoding::UTF16),
        "UTF16BE" => Some(TextEncoding::UTF16BE),
        "UTF8" => Some(TextEncoding::UTF8),
        _ => None,
    }
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
