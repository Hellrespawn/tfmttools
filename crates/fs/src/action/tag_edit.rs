use lofty::TextEncoding;
use lofty::config::WriteOptions;
use lofty::file::{AudioFile as LoftyAudioFile, TaggedFileExt};
use lofty::id3::v2::{Frame, Id3v2Tag};
use lofty::tag::{ItemKey, ItemValue, TagExt, TagItem, TagType};
use tfmttools_core::action::{TagValueChange, TagValueKind};
use tfmttools_core::item_keys::parse_item_key;

use crate::error::{FsError, FsResult};

pub fn write_tag_candidate(
    path: &camino::Utf8Path,
    changes: &[TagValueChange],
) -> FsResult {
    apply_tag_changes(path, changes, TagChangeDirection::Forward)
}

#[derive(Clone, Copy)]
pub(super) enum TagChangeDirection {
    Forward,
    Backward,
}

pub(super) fn apply_tag_changes(
    path: &camino::Utf8Path,
    changes: &[TagValueChange],
    direction: TagChangeDirection,
) -> FsResult {
    let mut tagged_file = lofty::read_from_path(path)
        .map_err(|err| FsError::Lofty(path.to_owned(), err))?;
    let tag = tagged_file.primary_tag_mut().ok_or_else(|| {
        tfmttools_core::error::TFMTError::NoPrimaryTag(path.to_owned())
    })?;

    for change in changes {
        let key = parse_item_key(change.key())?;
        let expected = match direction {
            TagChangeDirection::Forward => change.old_value(),
            TagChangeDirection::Backward => change.new_value(),
        };
        if !tag
            .items()
            .any(|item| tag_item_matches(item, key, change.kind(), expected))
        {
            return Err(FsError::Recovery(format!(
                "Requested source tag value missing: {}",
                change.key()
            )));
        }
    }
    for change in changes {
        apply_tag_change(tag, change, direction)?;
    }
    let id3v2_tag_with_encoding_changes =
        tag_with_encoding_changes(tag, changes, direction)?;

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
        FsError::Recovery("Written candidate has no primary tag".into())
    })?;
    for change in changes {
        let key = parse_item_key(change.key())?;
        let (value, encoding) = match direction {
            TagChangeDirection::Forward => {
                (change.new_value(), change.new_encoding())
            },
            TagChangeDirection::Backward => {
                (change.old_value(), change.old_encoding())
            },
        };
        if !written_tag
            .items()
            .any(|item| tag_item_matches(item, key, change.kind(), value))
        {
            return Err(FsError::Recovery(format!(
                "Candidate tag verification failed: {}",
                change.key()
            )));
        }
        if let Some(encoding) = encoding {
            let id = key.map_key(TagType::Id3v2).ok_or_else(|| {
                FsError::Recovery("Requested encoding has no ID3 frame".into())
            })?;
            let frames = Id3v2Tag::from(written_tag.clone());
            let expected =
                text_encoding_from_name(encoding).ok_or_else(|| {
                    FsError::Recovery("Unknown requested encoding".into())
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
                        return Err(FsError::Recovery(
                            "Candidate encoding verification failed".into(),
                        ));
                    }
                }
            }
            if !found {
                return Err(FsError::Recovery(
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
    direction: TagChangeDirection,
) -> FsResult {
    let key = parse_item_key(change.key())?;
    let (from, to) = match direction {
        TagChangeDirection::Forward => (change.old_value(), change.new_value()),
        TagChangeDirection::Backward => {
            (change.new_value(), change.old_value())
        },
    };
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
    direction: TagChangeDirection,
) -> FsResult<Option<Id3v2Tag>> {
    if tag.tag_type() != TagType::Id3v2 {
        return Ok(None);
    }

    let mut id3v2_tag = Id3v2Tag::from(tag.clone());
    let mut encoding_changed = false;

    for change in changes {
        let Some(encoding) = (match direction {
            TagChangeDirection::Forward => change.new_encoding(),
            TagChangeDirection::Backward => change.old_encoding(),
        }) else {
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
