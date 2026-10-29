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
        strict: bool,
    ) -> TFMTResult<(Utf8File, Vec<Warning>)> {
        let (path, warnings) = render_audio_path(template, self, strict)?;
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
        let (path, warnings) = render_audio_path(
            &template.bind(&[]).unwrap(),
            &audio(values),
            false,
        )
        .unwrap();
        (path.components().to_vec(), warnings)
    }

    #[test]
    fn template_tags_require_canonical_names_or_explicit_aliases() {
        for name in [
            "track_artist",
            "album_artist",
            "artist",
            "title",
            "album",
            "album_sort",
            "disk_number",
            "date",
        ] {
            assert!(
                compile_audio_template(&format!("path: ({{${name}}})")).is_ok(),
                "{name}"
            );
        }
        for name in [
            "TrackArtist",
            "TRACK_ARTIST",
            "trackartist",
            "track-artist",
            "albumartist",
            "DATE",
        ] {
            assert!(
                compile_audio_template(&format!("path: ({{${name}}})"))
                    .is_err(),
                "{name}"
            );
        }
    }

    #[test]
    fn numeric_tags_preserve_normalization_and_overflow() {
        for (raw, expected) in [
            ("0003", "3"),
            ("0", "0"),
            #[cfg(target_pointer_width = "64")]
            ("9223372036854775807", "9223372036854775807"),
            #[cfg(target_pointer_width = "64")]
            ("9223372036854775808", "9223372036854775808"),
        ] {
            for key in [ItemKey::TrackNumber, ItemKey::TrackTitle] {
                let name = if key == ItemKey::TrackNumber {
                    "track_number"
                } else {
                    "title"
                };
                assert_eq!(
                    render(&format!("path: ({{${name}}})"), &[(key, raw)]).0,
                    [expected]
                );
            }
        }
    }

    #[test]
    fn overflowing_text_keeps_leading_zeros() {
        assert_eq!(
            render("path: ({$title})", &[(
                ItemKey::TrackTitle,
                "009223372036854775808"
            )])
            .0,
            ["009223372036854775808"]
        );
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
            "path: ({$date | year} \"-\" {$disk_number} \"-\" {$track_number} \"-\" {$track_total} \"-\" {$movement_total})",
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
        assert_eq!(render(r#"path: ([$track_number? {$track_number}] [!$artist? "missing"] )"#,
            &[(ItemKey::TrackNumber, "0"), (ItemKey::TrackArtist, " ")]).0, ["0"]);
        for value in ["bad", "3/bad", "3/12/15", "18446744073709551616"] {
            assert_eq!(
                render(r#"path: ({$track_number ?? "absent"})"#, &[(
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
            render(r#"path: ({$track_number ?? "absent"})"#, &[(
                ItemKey::TrackNumber,
                "9223372036854775808"
            )])
            .0,
            [if cfg!(target_pointer_width = "64") {
                "9223372036854775808"
            } else {
                "absent"
            }]
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
            .construct_target_path(&bound, &Utf8Directory::new("work"), false)
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
                false,
            )
            .unwrap();
        assert_eq!(
            target.as_path(),
            camino::Utf8Path::new(std::path::MAIN_SEPARATOR_STR)
                .join("Song.mp3")
        );
    }

    #[test]
    fn character_policy_is_shared_by_literals_tags_and_arguments() {
        for (source, arguments) in [
            (r#"path: (" A/B\\C?:. ")"#, vec![]),
            ("path: ({$title})", vec![]),
            ("arg value: string path: ({value})", vec![
                " A/B\\C?:. ".to_owned(),
            ]),
        ] {
            let bound = compile_audio_template(source)
                .unwrap()
                .bind(&arguments)
                .unwrap();
            let file = audio(&[(ItemKey::TrackTitle, " A/B\\C?:. ")]);
            assert_eq!(
                render_audio_path(&bound, &file, false).unwrap().0.components(),
                ["A-B-C"]
            );
            let error =
                render_audio_path(&bound, &file, true).unwrap_err().to_string();
            assert!(error.contains("input/song.mp3"));
        }
    }

    #[test]
    fn character_policy_runs_after_interpolation() {
        let bound = compile_audio_template(
            r#"arg suffix: string path: ("prefix" {$title} {suffix} "suffix")"#,
        )
        .unwrap()
        .bind(&[". ".to_owned()])
        .unwrap();
        let file = audio(&[(ItemKey::TrackTitle, " Title. ")]);
        for strict in [false, true] {
            assert_eq!(
                render_audio_path(&bound, &file, strict)
                    .unwrap()
                    .0
                    .components(),
                ["prefix Title. . suffix"]
            );
        }
    }

    #[test]
    fn only_path_arguments_introduce_boundaries() {
        let bound = compile_audio_template(
            r#"arg prefix: path path: ({prefix} "A/B")"#,
        )
        .unwrap()
        .bind(&[" Music\\Artists? /Album. ".to_owned()])
        .unwrap();
        assert_eq!(
            render_audio_path(&bound, &audio(&[]), false)
                .unwrap()
                .0
                .components(),
            ["Music", "Artists", "Album", "A-B"]
        );
        assert!(render_audio_path(&bound, &audio(&[]), true).is_err());
    }

    #[test]
    fn prohibited_characters_are_rejected_before_normalization() {
        for character in [
            '\0', '\n', '\t', '\u{7f}', '\u{85}', '\u{2028}', '\u{2029}',
            '\u{202e}', '\u{2066}',
        ] {
            for strict in [false, true] {
                for (name, formatter) in [
                    ("title", ""),
                    ("track_number", ""),
                    ("date", ""),
                    ("date", " | year"),
                ] {
                    let key = match name {
                        "title" => ItemKey::TrackTitle,
                        "date" => ItemKey::RecordingDate,
                        _ => ItemKey::TrackNumber,
                    };
                    let bound = compile_audio_template(&format!(
                        "path: ({{${name} ?? \"fallback\"{formatter}}})"
                    ))
                    .unwrap()
                    .bind(&[])
                    .unwrap();
                    assert!(
                        render_audio_path(
                            &bound,
                            &audio(&[(key, &format!("2024{character}"))]),
                            strict
                        )
                        .is_err()
                    );
                }
            }
            for kind in ["string", "path", "int"] {
                let template = compile_audio_template(&format!(
                    "arg value: {kind} path: (\"Song\")"
                ))
                .unwrap();
                assert!(template.bind(&[format!("3{character}")]).is_err());
            }
            assert!(
                compile_audio_template(&format!("path: (\"Song{character}\")"))
                    .is_err()
            );
        }
        assert_eq!(
            render("path: ({$title})", &[(
                ItemKey::TrackTitle,
                "Cafe\u{301} 👩\u{200d}💻 ✈\u{fe0f}"
            )])
            .0,
            ["Cafe\u{301} 👩\u{200d}💻 ✈\u{fe0f}"]
        );
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
            false,
        )
        .unwrap_err()
        .to_string();
        assert!(message.contains("input/song.mp3"));

        assert!(message.contains("2:"));
        let template = compile_audio_template(r#"path: ({$artist} / "Song")"#)
            .unwrap()
            .bind(&[])
            .unwrap();
        assert!(render_audio_path(&template, &audio(&[]), false).is_err());
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
        assert_eq!(
            render_audio_path(&bound, &file, false).unwrap().0.components(),
            [
                "Music",
                "Artists",
                "Example Artist",
                "2024.02 - Example Album",
                "103 - Example Artist - Example Song"
            ]
        );
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
                    &audio(&values),
                    false
                )
                .unwrap()
                .0
                .components(),
                ["Album Artist", "00 - Song"]
            );
        }
    }
    #[test]
    fn character_replacement_cannot_leave_empty_components() {
        for source in [
            r#"path: (".")"#,
            r#"path: ("..")"#,
            r#"path: ("???" / "Song")"#,
            r#"path: (" ")"#,
        ] {
            let bound =
                compile_audio_template(source).unwrap().bind(&[]).unwrap();
            for strict in [false, true] {
                assert!(
                    render_audio_path(&bound, &audio(&[]), strict).is_err()
                );
            }
        }
    }

    #[test]
    fn path_language_empty_dates_do_not_block_fallback() {
        assert_eq!(
            render("path: ({$date})", &[
                (ItemKey::RecordingDate, ""),
                (ItemKey::Year, "1999"),
                (ItemKey::OriginalReleaseDate, "1980"),
            ])
            .0,
            ["1999"]
        );
        assert_eq!(
            render("path: ({$date})", &[
                (ItemKey::RecordingDate, ""),
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
            assert_eq!(render(r#"path: ({$track_total ?? "missing"} "-" {$disc_total ?? "missing"} "-" {$movement_total ?? "missing"})"#, &values).0, ["12-2-5"]);
        }
    }
}
