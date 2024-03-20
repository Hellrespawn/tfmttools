use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct NormalizeProgram {
    commands: Vec<NormalizeCommand>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum NormalizeCommand {
    Retain(Retain),
    Collapse(Collapse),
}

// Retain only the specified tags.
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Retain {
    tags: Vec<String>,
}

// Remove `optional_tags` from file if they match `predicate(main_tag, optional_tag)`
#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Collapse {
    main_tag: String,
    optional_tags: Vec<String>,
    predicate: Predicate,
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
