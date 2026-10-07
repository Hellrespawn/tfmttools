use camino::{Utf8Path, Utf8PathBuf};
use lofty::file::{TaggedFile, TaggedFileExt};
use lofty::tag::Tag;

use crate::error::{TFMTError, TFMTResult};
use crate::templates::Template;
use crate::util::{Utf8Directory, Utf8File, Utf8PathExt, normalize_separators};
use crate::warning::Warning;

#[derive(Clone)]
pub struct AudioFile {
    file: Utf8File,
    tag: Tag,
}

impl std::fmt::Debug for AudioFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioFile")
            .field("path", &self.file)
            .finish_non_exhaustive()
    }
}

impl AudioFile {
    pub const SUPPORTED_EXTENSIONS: [&'static str; 3] = ["mp3", "ogg", "m4a"];

    pub fn from_tagged_file(
        file: Utf8File,
        tagged_file: &TaggedFile,
    ) -> TFMTResult<AudioFile> {
        match tagged_file.primary_tag() {
            Some(tag) => Ok(AudioFile { file: file.clone(), tag: tag.clone() }),
            None => Err(TFMTError::NoPrimaryTag(file.into_path_buf())),
        }
    }

    #[must_use]
    pub fn file(&self) -> &Utf8File {
        &self.file
    }

    #[must_use]
    pub fn extension(&self) -> &str {
        self.file.extension().expect("Audio file should always have extension.")
    }

    #[must_use]
    pub fn tag(&self) -> &Tag {
        &self.tag
    }

    pub fn construct_target_path(
        &self,
        template: &Template,
        relative_path: &Utf8Directory,
    ) -> TFMTResult<(Utf8File, Vec<Warning>)> {
        let (string, warnings) = template.render(self)?;

        let string = normalize_separators(&string);

        let target_path =
            Utf8PathBuf::from(format!("{string}.{}", self.extension()));

        // If target_path is an absolute path, join will clobber the
        // relative_path, so this is always safe.
        let target_path = relative_path.join_file(target_path);

        Ok((target_path, warnings))
    }

    pub fn tag_mut(&mut self) -> &mut Tag {
        &mut self.tag
    }

    #[must_use]
    pub fn path_predicate(path: &Utf8Path) -> bool {
        path.extension().is_some_and(|extension| {
            for supported_extension in AudioFile::SUPPORTED_EXTENSIONS {
                if extension == supported_extension {
                    return true;
                }
            }

            false
        })
    }
}

#[cfg(test)]
mod path_template_tests {
    use lofty::tag::{ItemKey, ItemValue, TagItem, TagType};

    use super::*;
    use crate::templates::CompiledTemplate;

    fn audio(values: &[(ItemKey, &str)]) -> AudioFile {
        let mut tag = Tag::new(TagType::Id3v2);
        for (key, value) in values {
            tag.insert_unchecked(TagItem::new(
                *key,
                ItemValue::Text((*value).to_owned()),
            ));
        }
        AudioFile { file: Utf8File::new("input/song.mp3"), tag }
    }

    fn render(
        source: &str,
        values: &[(ItemKey, &str)],
    ) -> (Vec<String>, Vec<Warning>) {
        let template =
            CompiledTemplate::compile("test", source.to_owned()).unwrap();
        let (path, warnings) =
            template.bind(&[]).unwrap().render(&audio(values)).unwrap();
        (path.components().to_vec(), warnings)
    }

    #[test]
    fn path_language_sanitizes_tags_and_reports_whitespace() {
        let (components, warnings) = render("path: ({$artist} / {$title})", &[
            (ItemKey::TrackArtist, " AC/DC: Live. "),
            (ItemKey::TrackTitle, "Song"),
        ]);
        assert_eq!(components, ["AC-DC Live", "Song"]);
        assert_eq!(warnings, [Warning::WhitespaceInTag {
            file: "song.mp3".to_owned(),
            tag_name: "track_artist".to_owned(),
        }]);
    }

    #[test]
    fn path_language_retains_dates_numbers_and_aliases() {
        let (components, _) = render(
            "path: ({$DATE | year} \"-\" {$disk_number} \"-\" {$tracknumber} \"-\" {$tracktotal} \"-\" {$movementtotal})",
            &[
                (ItemKey::RecordingDate, "2024-03-10"),
                (ItemKey::Year, "1999"),
                (ItemKey::DiscNumber, "1/2"),
                (ItemKey::TrackNumber, "3/12"),
                (ItemKey::MovementNumber, "4/5"),
                (ItemKey::TrackTotal, "3/12"),
                (ItemKey::MovementTotal, "4/5"),
            ],
        );
        assert_eq!(components, ["2024-1-3-12-5"]);
        let (components, _) = render("path: ({$date | year})", &[
            (ItemKey::Year, "1999"),
            (ItemKey::OriginalReleaseDate, "1980"),
        ]);
        assert_eq!(components, ["1999"]);
        assert_eq!(
            render("path: ({$date})", &[(
                ItemKey::OriginalReleaseDate,
                "1980"
            )])
            .0,
            ["1980"]
        );
    }

    #[test]
    fn path_language_presence_and_malformed_numbers() {
        assert_eq!(render(r#"path: ([$tracknumber? {$tracknumber}] [!$artist? "missing"] )"#,
            &[(ItemKey::TrackNumber, "0"), (ItemKey::TrackArtist, " ")]).0, ["0missing"]);
        for value in ["bad", "3/bad", "3/12/15", "18446744073709551616"] {
            assert_eq!(
                render(r#"path: ({$tracknumber ?? "absent"})"#, &[(
                    ItemKey::TrackNumber,
                    value
                )])
                .0,
                ["absent"]
            );
        }
        assert_eq!(
            render("path: ({$title})", &[(
                ItemKey::TrackTitle,
                "9223372036854775808"
            )])
            .0,
            ["9223372036854775808"]
        );
        assert_eq!(
            render("path: ({$tracknumber})", &[(
                ItemKey::TrackNumber,
                "9223372036854775808"
            )])
            .0,
            ["9223372036854775808"]
        );
    }

    #[test]
    fn path_language_checks_skipped_unknown_tags() {
        let error = CompiledTemplate::compile(
            "my-script",
            "path: (\n [$album? {$not_a_tag}] \"Song\"\n)".to_owned(),
        )
        .unwrap_err();
        let message = error.to_string();
        assert!(message.contains("my-script"));
        assert!(message.contains("2:"));
        assert!(message.contains("not_a_tag"));
    }

    #[test]
    fn path_language_legacy_syntax_has_migration_hint() {
        for source in [
            "{{ artist }}/{{ title }}",
            "+++\nname = \"Old\"\n+++\n{{ title }}",
        ] {
            let message = CompiledTemplate::compile("old", source.to_owned())
                .unwrap_err()
                .to_string();
            assert!(message.contains("migrat"), "{message}");
            assert!(message.contains("path:"), "{message}");
        }
    }
}
