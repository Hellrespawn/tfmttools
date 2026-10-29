use camino::{Utf8Path, Utf8PathBuf};
use lofty::TextEncoding;
use lofty::file::{FileType, TaggedFileExt};
use lofty::tag::TagItem;
use tfmttools_core::action::{
    FORBIDDEN_CHARACTERS, TagValueChange, TagValueKind,
};
use tfmttools_core::id3::text_encodings_for_key;
use tfmttools_core::item_keys::canonical_tag_name;
use tfmttools_core::tag_encoding::TagTextEncoding;
use tfmttools_core::templates::sanitize_tag_value;
use tfmttools_fs::{FsError, FsResult, read_native_id3v2};

#[derive(Debug, Default)]
pub(super) struct FileInspection {
    pub(super) tag_issues: Vec<TagValueIssue>,
    pub(super) id3_encoding_issues: Vec<Id3EncodingIssue>,
    pub(super) changes: Vec<TagValueChange>,
}

pub(super) fn inspect_file(
    path: &Utf8Path,
    scope: ValidationScope,
) -> FsResult<FileInspection> {
    let audio = lofty::read_from_path(path)
        .map_err(|error| FsError::Lofty(path.to_owned(), error))?;
    let native =
        if scope.includes_id3_encoding() && path.extension() == Some("mp3") {
            read_native_id3v2(path, FileType::Mpeg)?
        } else {
            None
        };
    let mut result = FileInspection::default();
    if matches!(scope, ValidationScope::Id3Encoding) && native.is_none() {
        return Ok(result);
    }
    let tag = audio.primary_tag().ok_or_else(|| {
        tfmttools_core::error::TFMTError::NoPrimaryTag(path.to_owned())
    })?;
    for item in tag.items() {
        let Some((kind, value)) = tag_item_value(item) else {
            continue;
        };
        let mut fixed = None;
        if scope.includes_characters() {
            let characters = forbidden_characters_in(value);
            if !characters.is_empty() {
                result.tag_issues.push(TagValueIssue {
                    path: path.to_owned(),
                    key: format!("{:?}", item.key()),
                    value: value.to_owned(),
                    characters,
                });
            }
            let new_value = sanitize_tag_value(value);
            if new_value != value {
                fixed = Some(FieldFix {
                    new_value,
                    old_encoding: None,
                    new_encoding: None,
                });
            }
        }
        if let Some(encoding) = native
            .as_ref()
            .and_then(|tag| text_encodings_for_key(tag, item.key()).next())
            && let Some(mut encoding_fix) =
                rewrite_id3_text_as_utf16(value, encoding.into())
        {
            result.id3_encoding_issues.push(Id3EncodingIssue {
                path: path.to_owned(),
                key: format!("{:?}", item.key()),
                value: value.to_owned(),
                encoding: encoding.name().to_owned(),
            });
            if let Some(character_fix) = fixed {
                encoding_fix.new_value = character_fix.new_value;
            }
            fixed = Some(encoding_fix);
        }
        if let (Some(fixed), Some(key)) =
            (fixed, canonical_tag_name(item.key()))
        {
            result.changes.push(
                TagValueChange::new(
                    key.to_owned(),
                    kind,
                    value.to_owned(),
                    fixed.new_value,
                )
                .with_encoding(fixed.old_encoding, fixed.new_encoding),
            );
        }
    }
    Ok(result)
}

fn tag_item_value(item: &TagItem) -> Option<(TagValueKind, &str)> {
    item.value().text().map(|value| (TagValueKind::Text, value)).or_else(|| {
        item.value().locator().map(|value| (TagValueKind::Locator, value))
    })
}

#[derive(Debug, Clone, Copy)]
pub(super) enum ValidationScope {
    All,
    Characters,
    Id3Encoding,
}

impl ValidationScope {
    fn includes_characters(self) -> bool {
        matches!(self, Self::All | Self::Characters)
    }

    fn includes_id3_encoding(self) -> bool {
        matches!(self, Self::All | Self::Id3Encoding)
    }
}

fn forbidden_characters_in(value: &str) -> Vec<&'static str> {
    let mut forbidden_characters = Vec::new();

    for forbidden_character in FORBIDDEN_CHARACTERS.iter() {
        if ((!forbidden_character.leading() && !forbidden_character.trailing())
            && value.contains(forbidden_character.char()))
            || (forbidden_character.leading()
                && value.starts_with(forbidden_character.char()))
            || (forbidden_character.trailing()
                && value.ends_with(forbidden_character.char()))
        {
            forbidden_characters.push(forbidden_character.char());
        }
    }

    forbidden_characters
}

fn should_rewrite_id3_text_as_utf16(
    value: &str,
    encoding: TextEncoding,
) -> bool {
    encoding == TextEncoding::UTF8 && !value.is_ascii()
}

fn rewrite_id3_text_as_utf16(
    value: &str,
    source_encoding: TextEncoding,
) -> Option<FieldFix> {
    should_rewrite_id3_text_as_utf16(value, source_encoding).then(|| {
        FieldFix {
            new_value: value.to_owned(),
            old_encoding: Some(TagTextEncoding::from(source_encoding)),
            new_encoding: Some(TagTextEncoding::UTF16),
        }
    })
}

#[derive(Debug)]
struct FieldFix {
    new_value: String,
    old_encoding: Option<TagTextEncoding>,
    new_encoding: Option<TagTextEncoding>,
}

#[derive(Debug)]
pub(super) struct TagValueIssue {
    pub(super) path: Utf8PathBuf,
    pub(super) key: String,
    pub(super) value: String,
    pub(super) characters: Vec<&'static str>,
}

#[derive(Debug)]
pub(super) struct Id3EncodingIssue {
    pub(super) path: Utf8PathBuf,
    pub(super) key: String,
    pub(super) value: String,
    pub(super) encoding: String,
}

#[cfg(test)]
mod tests {
    use lofty::TextEncoding;
    use tfmttools_core::tag_encoding::TagTextEncoding;

    use super::{
        forbidden_characters_in, rewrite_id3_text_as_utf16, sanitize_tag_value,
        should_rewrite_id3_text_as_utf16,
    };

    #[test]
    fn finds_forbidden_characters() {
        assert_eq!(forbidden_characters_in("AC/DC: Live"), vec![":", "/"]);
    }

    #[test]
    fn replaces_forbidden_characters_like_templates() {
        assert_eq!(sanitize_tag_value("AC/DC: Live."), "AC-DC Live");
        assert_eq!(sanitize_tag_value(" AC/DC: Live. "), "AC-DC Live");
    }

    #[test]
    fn rewrites_non_ascii_utf8_text_as_utf16() {
        let fix =
            rewrite_id3_text_as_utf16("Ich Weiß Es Nicht", TextEncoding::UTF8)
                .unwrap();

        assert_eq!(fix.new_value, "Ich Weiß Es Nicht");
        assert_eq!(fix.old_encoding, Some(TagTextEncoding::UTF8));
        assert_eq!(fix.new_encoding, Some(TagTextEncoding::UTF16));
    }

    #[test]
    fn skips_ascii_id3_text() {
        assert!(
            rewrite_id3_text_as_utf16("Nothing To Fix", TextEncoding::UTF8)
                .is_none()
        );
    }

    #[test]
    fn skips_id3_text_that_is_already_utf16() {
        assert!(
            rewrite_id3_text_as_utf16("Ich Weiß Es Nicht", TextEncoding::UTF16)
                .is_none()
        );
    }

    #[test]
    fn uses_same_id3_encoding_predicate_for_check_and_fix() {
        assert!(should_rewrite_id3_text_as_utf16(
            "Ich Weiß Es Nicht",
            TextEncoding::UTF8
        ));
        assert!(!should_rewrite_id3_text_as_utf16(
            "Nothing To Fix",
            TextEncoding::UTF8
        ));
        assert!(!should_rewrite_id3_text_as_utf16(
            "Ich Weiß Es Nicht",
            TextEncoding::UTF16
        ));
    }
}
