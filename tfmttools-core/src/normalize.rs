use color_eyre::Result;
use lofty::{ItemKey, TagExt};
use serde::Deserialize;

use crate::audiofile::AudioFile;
use crate::item_keys::ItemKeys;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct NormalizeProgram {
    directives: Vec<NormalizeDirective>,
}

impl NormalizeProgram {
    pub fn process_files(&self, files: &mut [AudioFile]) -> Result<()> {
        for directive in &self.directives {
            directive.apply(files)?;
        }

        for file in files {
            file.tag().save_to_path(file.path())?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NormalizeDirective {
    Retain(Retain),
    Collapse(Collapse),
}

impl NormalizeDirective {
    fn apply(&self, files: &mut [AudioFile]) -> Result<()> {
        match self {
            NormalizeDirective::Retain(retain) => retain.apply(files),
            NormalizeDirective::Collapse(collapse) => collapse.apply(files),
        }
    }
}

// Retain only the specified tags.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Retain {
    tags: Vec<String>,
}

impl Retain {
    fn get_item_keys(&self) -> Result<Vec<&ItemKey>> {
        let retained_tags = self
            .tags
            .iter()
            .map(|tag| {
                let tags = match tag.as_str() {
                    "album" => vec![&ItemKey::AlbumTitle],
                    "date" => {
                        vec![
                            &ItemKey::RecordingDate,
                            &ItemKey::Year,
                            &ItemKey::OriginalReleaseDate,
                        ]
                    },
                    other => vec![ItemKeys::from_string(other)?],
                };

                Ok(tags)
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        Ok(retained_tags)
    }

    fn apply(&self, files: &mut [AudioFile]) -> Result<()> {
        let retained_tags = self.get_item_keys()?;

        let tags_to_remove = ItemKeys::all()
            .iter()
            .filter(|k| !retained_tags.contains(k))
            .collect::<Vec<_>>();

        for file in files {
            for item_key in &tags_to_remove {
                file.tag_mut().remove_key(item_key)
            }
        }

        Ok(())
    }
}

// Remove `optional_tags` from file if they match `predicate(main_tag, optional_tag)`
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Collapse {
    main_tag: String,
    optional_tags: Vec<String>,
    predicate: Predicate,
}

impl Collapse {
    fn apply(&self, files: &mut [AudioFile]) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Predicate {
    Equal,
}

#[cfg(test)]
mod test {
    use color_eyre::Result;

    use super::*;

    fn parse_test_program(
        input: &str,
        reference: &NormalizeProgram,
    ) -> Result<NormalizeProgram> {
        let program: NormalizeProgram = serde_json::from_str(input)?;

        assert_eq!(program, *reference);

        Ok(program)
    }

    fn parse_deserialize_test_program() -> Result<NormalizeProgram> {
        let string = include_str!("../tests/normalize-deserialize.json");

        let reference = NormalizeProgram {
            directives: vec![NormalizeDirective::Retain(Retain {
                tags: vec![
                    "genre".to_owned(),
                    "albumartist".to_owned(),
                    "artist".to_owned(),
                    "album".to_owned(),
                    "date".to_owned(),
                    "year".to_owned(),
                    "albumsort".to_owned(),
                    "discnumber".to_owned(),
                    "tracknumber".to_owned(),
                    "title".to_owned(),
                    "*replaygain*".to_owned(),
                ],
            })],
        };

        let program = parse_test_program(string, &reference)?;

        Ok(program)
    }

    #[test]
    fn test_deserialize() -> Result<()> {
        let program = parse_deserialize_test_program()?;

        assert_eq!(program.directives.len(), 1);

        let directive = &program.directives[0];

        let NormalizeDirective::Retain(retain) = directive else {
            panic!("Expected directive 'Retain', found {directive:?}",)
        };

        let retained_tags = retain.get_item_keys()?;

        Ok(())
    }
}
