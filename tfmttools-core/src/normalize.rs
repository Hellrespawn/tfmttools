use color_eyre::{eyre::eyre, Result};
use lofty::{ItemKey, TagExt};
use serde::Deserialize;

use crate::{audiofile::AudioFile, item_keys::ItemKeys};

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct NormalizeProgram {
    commands: Vec<NormalizeCommand>,
}

impl NormalizeProgram {
    pub fn process_files(&self, files: &mut [AudioFile]) -> Result<()> {
        for command in &self.commands {
            command.apply(files)?;
        }

        for file in files {
            file.tag().save_to_path(file.path())?;
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NormalizeCommand {
    Retain(Retain),
    Collapse(Collapse),
}

impl NormalizeCommand {
    fn apply(&self, files: &mut [AudioFile]) -> Result<()> {
        match self {
            NormalizeCommand::Retain(retain) => retain.apply(files),
            NormalizeCommand::Collapse(collapse) => collapse.apply(files),
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
    fn apply(&self, files: &mut [AudioFile]) -> Result<()> {
        let retained_tags = self
            .tags
            .iter()
            .map(|tag| {
                let key = ItemKeys::from_string(tag)?;

                Ok(key)
            })
            .collect::<Result<Vec<&ItemKey>>>()?;

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

    #[test]
    fn test_deserialize() -> Result<()> {
        let string = include_str!("../../examples/normalize.json");

        let program: NormalizeProgram = serde_json::from_str(string)?;

        let reference = NormalizeProgram {
            commands: vec![
                NormalizeCommand::Retain(Retain {
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
                }),
                NormalizeCommand::Collapse(Collapse {
                    main_tag: "artist".to_owned(),
                    optional_tags: vec!["albumartist".to_owned()],
                    predicate: Predicate::Equal,
                }),
            ],
        };

        assert_eq!(program, reference);

        Ok(())
    }
}
