use camino::{Utf8Path, Utf8PathBuf};
use lofty::file::{TaggedFile, TaggedFileExt};
use lofty::tag::Tag;
use tfmttools_picotmpl::BoundTemplate;

use crate::error::{TFMTError, TFMTResult};
use crate::templates::render_audio_path;
use crate::util::{Utf8Directory, Utf8File, Utf8PathExt};
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
        template: &BoundTemplate,
        relative_path: &Utf8Directory,
    ) -> TFMTResult<(Utf8File, Vec<Warning>)> {
        let (path, warnings) = render_audio_path(template, self)?;
        let (filename, directories) = path
            .components()
            .split_last()
            .expect("Rendered path has a filename component");
        let mut target_path = if path.is_rooted() {
            Utf8PathBuf::from(std::path::MAIN_SEPARATOR_STR)
        } else {
            Utf8PathBuf::new()
        };
        for directory in directories {
            target_path.push(directory);
        }
        target_path.push(format!("{filename}.{}", self.extension()));

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
    use crate::templates::compile_audio_template;

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
        let template = compile_audio_template(source).unwrap();
        let (path, warnings) =
            render_audio_path(&template.bind(&[]).unwrap(), &audio(values))
                .unwrap();
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
        let error = compile_audio_template(
            "path: (\n [$album? {$not_a_tag}] \"Song\"\n)",
        )
        .unwrap_err();
        let message = error.to_string();

        assert!(message.contains("2:"));
        assert!(message.contains("not_a_tag"));
    }

    #[test]
    fn path_language_legacy_syntax_has_migration_hint() {
        for source in [
            "{{ artist }}/{{ title }}",
            "+++\nname = \"Old\"\n+++\n{{ title }}",
        ] {
            let message =
                compile_audio_template(source).unwrap_err().to_string();
            assert!(message.contains("migrat"), "{message}");
            assert!(message.contains("path:"), "{message}");
        }
    }
    #[test]
    fn path_language_native_destination_preserves_extension() {
        let template = compile_audio_template(
            r#"arg prefix: path path: ({prefix} "Song.part")"#,
        )
        .unwrap();
        let bound = template.bind(&[r"Music\Artists".to_owned()]).unwrap();
        let (target, _) = audio(&[])
            .construct_target_path(&bound, &Utf8Directory::new("work"))
            .unwrap();
        assert_eq!(
            target.as_path(),
            camino::Utf8Path::new("work")
                .join("Music")
                .join("Artists")
                .join("Song.part.mp3")
        );
        let template = compile_audio_template(r#"path: (/ "Song")"#).unwrap();
        let (target, _) = audio(&[])
            .construct_target_path(
                &template.bind(&[]).unwrap(),
                &Utf8Directory::new("work"),
            )
            .unwrap();
        assert_eq!(
            target.as_path(),
            camino::Utf8Path::new(std::path::MAIN_SEPARATOR_STR)
                .join("Song.mp3")
        );
    }

    #[test]
    fn path_language_argument_policy_validates_every_forbidden_character() {
        use crate::action::FORBIDDEN_CHARACTERS;
        let template =
            compile_audio_template(r#"arg unused: string path: ("Song")"#)
                .unwrap();
        for entry in FORBIDDEN_CHARACTERS.iter() {
            assert!(
                template.bind(&[format!("bad{}value", entry.char())]).is_err()
            );
        }
        let path = compile_audio_template(r#"arg unused: path path: ("Song")"#)
            .unwrap();
        for entry in FORBIDDEN_CHARACTERS
            .iter()
            .filter(|entry| !["/", "\\"].contains(&entry.char()))
        {
            let message = path
                .bind(&[format!("ok/bad{}value", entry.char())])
                .unwrap_err()
                .to_string();
            assert!(message.contains("unused"));
            assert!(message.contains("component"));
        }
        assert!(template.bind(&[]).is_err());
        assert!(template.bind(&["ok".to_owned(), "extra".to_owned()]).is_err());
        let optional = compile_audio_template(
            r#"arg unused: string(default: "") path: ("Song")"#,
        )
        .unwrap();
        assert!(optional.bind(&[]).is_ok());
        let invalid = compile_audio_template(
            r#"arg unused: string(default: "bad?") path: ("Song")"#,
        )
        .unwrap_err();
        assert!(invalid.to_string().contains("unused"));
    }

    #[test]
    fn path_language_render_errors_identify_file_and_source() {
        let template = compile_audio_template("path: (\n {$date | year}\n)")
            .unwrap()
            .bind(&[])
            .unwrap();
        let message = render_audio_path(
            &template,
            &audio(&[(ItemKey::RecordingDate, "invalid")]),
        )
        .unwrap_err()
        .to_string();
        assert!(message.contains("input/song.mp3"));

        assert!(message.contains("2:"));
        let template = compile_audio_template(r#"path: ({$artist} / "Song")"#)
            .unwrap()
            .bind(&[])
            .unwrap();
        assert!(render_audio_path(&template, &audio(&[])).is_err());
    }
    #[test]
    fn path_language_stef_example_with_audio_metadata() {
        let template =
            compile_audio_template(include_str!("../../../examples/stef.tfmt"))
                .unwrap();
        let bound = template.bind(&["Music/Artists".to_owned()]).unwrap();
        let file = audio(&[
            (ItemKey::AlbumArtist, "Example Artist"),
            (ItemKey::TrackArtist, "Example Artist"),
            (ItemKey::AlbumTitle, "Example Album"),
            (ItemKey::RecordingDate, "2024-03-10"),
            (ItemKey::AlbumTitleSortOrder, "2"),
            (ItemKey::DiscNumber, "1"),
            (ItemKey::TrackNumber, "3/12"),
            (ItemKey::TrackTitle, "Example Song"),
        ]);
        assert_eq!(render_audio_path(&bound, &file).unwrap().0.components(), [
            "Music",
            "Artists",
            "Example Artist",
            "2024.02 - Example Album",
            "103 - Example Artist - Example Song"
        ]);
        for artist in [None, Some("")] {
            let mut values = vec![
                (ItemKey::AlbumArtist, "Album Artist"),
                (ItemKey::TrackTitle, "Song"),
                (ItemKey::TrackNumber, "0"),
            ];
            if let Some(artist) = artist {
                values.push((ItemKey::TrackArtist, artist));
            }
            assert_eq!(
                render_audio_path(
                    &template.bind(&[]).unwrap(),
                    &audio(&values)
                )
                .unwrap()
                .0
                .components(),
                ["Album Artist", "00 - Song"]
            );
        }
    }
    #[test]
    fn path_language_appends_extension_before_dot_component_conversion() {
        for (source, expected) in [
            (r#"path: (".")"#, "..mp3"),
            (r#"path: ("..")"#, "...mp3"),
            (r#"path: ("Album" / ".")"#, "Album/..mp3"),
            (r#"path: ("Album" / "..")"#, "Album/...mp3"),
        ] {
            let template =
                compile_audio_template(source).unwrap().bind(&[]).unwrap();
            let (target, _) = audio(&[])
                .construct_target_path(&template, &Utf8Directory::new("work"))
                .unwrap();
            assert_eq!(
                target.as_path(),
                camino::Utf8Path::new("work").join(expected)
            );
        }
    }

    #[test]
    fn path_language_empty_dates_do_not_block_fallback() {
        for recording in ["", " ", "..."] {
            assert_eq!(
                render("path: ({$date})", &[
                    (ItemKey::RecordingDate, recording),
                    (ItemKey::Year, "1999"),
                    (ItemKey::OriginalReleaseDate, "1980"),
                ])
                .0,
                ["1999"]
            );
        }
        assert_eq!(
            render("path: ({$date})", &[
                (ItemKey::RecordingDate, " "),
                (ItemKey::Year, ""),
                (ItemKey::OriginalReleaseDate, "1980"),
            ])
            .0,
            ["1980"]
        );
    }

    #[test]
    fn path_language_totals_support_separate_fields_and_raw_pairs() {
        for values in [
            vec![
                (ItemKey::TrackNumber, "3"),
                (ItemKey::TrackTotal, "12"),
                (ItemKey::DiscNumber, "1"),
                (ItemKey::DiscTotal, "2"),
                (ItemKey::MovementNumber, "4"),
                (ItemKey::MovementTotal, "5"),
            ],
            vec![
                (ItemKey::TrackNumber, "3/12"),
                (ItemKey::DiscNumber, "1/2"),
                (ItemKey::MovementNumber, "4/5"),
            ],
        ] {
            assert_eq!(render(r#"path: ({$tracktotal ?? "missing"} "-" {$disctotal ?? "missing"} "-" {$movementtotal ?? "missing"})"#, &values).0, ["12-2-5"]);
        }
    }
}
