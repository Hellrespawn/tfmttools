use std::collections::HashMap;
use std::sync::LazyLock;

use camino::Utf8Component;

use crate::MAX_PATH_LENGTH;
use crate::action::{CaseInsensitivePathKey, RenameAction};
use crate::util::Utf8PathExt;

pub struct ForbiddenCharacter<'f> {
    char: &'f str,
    replacement: Option<&'f str>,
}

impl ForbiddenCharacter<'_> {
    pub fn char(&self) -> &str {
        self.char
    }

    pub fn replacement(&self) -> Option<&str> {
        self.replacement
    }
}

pub static FORBIDDEN_CHARACTERS: LazyLock<Vec<ForbiddenCharacter>> =
    LazyLock::new(|| {
        vec![
            ForbiddenCharacter { char: "<", replacement: None },
            ForbiddenCharacter { char: "\"", replacement: None },
            ForbiddenCharacter { char: ">", replacement: None },
            ForbiddenCharacter { char: ":", replacement: None },
            ForbiddenCharacter { char: "|", replacement: None },
            ForbiddenCharacter { char: "?", replacement: None },
            ForbiddenCharacter { char: "*", replacement: None },
            ForbiddenCharacter { char: "~", replacement: Some("-") },
            ForbiddenCharacter { char: "/", replacement: Some("-") },
            ForbiddenCharacter { char: "\\", replacement: Some("-") },
        ]
    });

pub struct ForbiddenLeadingOrTrailingChar<'f> {
    char: &'f str,
    leading: bool,
    trailing: bool,
}

impl ForbiddenLeadingOrTrailingChar<'_> {
    pub fn char(&self) -> &str {
        self.char
    }

    pub fn leading(&self) -> bool {
        self.leading
    }

    pub fn trailing(&self) -> bool {
        self.trailing
    }
}

pub static FORBIDDEN_LEADING_OR_TRAILING_CHARACTERS: LazyLock<
    Vec<ForbiddenLeadingOrTrailingChar>,
> = LazyLock::new(|| {
    vec![
        ForbiddenLeadingOrTrailingChar {
            char: " ",
            leading: true,
            trailing: true,
        },
        ForbiddenLeadingOrTrailingChar {
            char: ".",
            leading: false,
            trailing: true,
        },
    ]
});

#[derive(Debug)]
pub enum ValidationError<'e> {
    DoubleSeparators(&'e RenameAction),
    Collision(Vec<&'e RenameAction>),
    CaseInsensitiveCollision(Vec<&'e RenameAction>),
    TargetExists(&'e RenameAction),
    ReservedName {
        action: &'e RenameAction,
        component: &'e str,
    },
    ForbiddenCharacterLeadingOrTrailingPathComponent {
        action: &'e RenameAction,
        component: &'e str,
        forbidden_leading_characters: Option<Vec<&'e str>>,
        forbidden_trailing_characters: Option<Vec<&'e str>>,
    },
    PathTooLong {
        action: &'e RenameAction,
        max_length: usize,
        actual_length: usize,
    },
}

impl std::fmt::Display for ValidationError<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::DoubleSeparators(action) => {
                writeln!(
                    f,
                    "The target file contains double path separators."
                )?;
                write_source_and_target(f, action)?;
            },
            ValidationError::Collision(actions) => {
                writeln!(f, "These files all evaluate to the same target.")?;
                for action in actions {
                    writeln!(f, "\tsource: {}", action.source())?;
                }

                writeln!(f, "\ttarget: {}", actions.first().unwrap().target())?;
            },
            ValidationError::CaseInsensitiveCollision(actions) => {
                writeln!(
                    f,
                    "These files evaluate to targets that differ only by case."
                )?;
                for action in actions {
                    writeln!(f, "\tsource: {}", action.source())?;
                    writeln!(f, "\ttarget: {}", action.target())?;
                }
            },
            ValidationError::TargetExists(action) => {
                writeln!(f, "The target file already exists.")?;
                write_source_and_target(f, action)?;
            },
            ValidationError::ReservedName {
                action,
                component
            } => {
                writeln!(
                    f,
                    "The '{component}'-component in the target path is a reserved Windows device name."
                )?;
                write_source_and_target(f, action)?;
            },
            ValidationError::ForbiddenCharacterLeadingOrTrailingPathComponent {
                action,
                component,
                forbidden_leading_characters,
                forbidden_trailing_characters,
            } => {
                write!(
                    f,
                    "The '{component}'-component in the target path has  "
                )?;

                match (forbidden_leading_characters, forbidden_trailing_characters) {
                    (None, None) => unreachable!("Action without forbidden leading or trailing characters should not fail validation."),
                    (Some(forbidden_leading_characters), None) => write!(f, "forbidden leading characters: '{}'", forbidden_leading_characters.join("', '")),
                    (None, Some(forbidden_trailing_characters)) => write!(f, "forbidden trailing characters: '{}'", forbidden_trailing_characters.join("', '")),
                    (Some(forbidden_leading_characters), Some(forbidden_trailing_characters)) => write!(f, "forbidden leading ('{}') and trailing ('{}') characters.", forbidden_leading_characters.join("', '"), forbidden_trailing_characters.join("', '")),
                }?;

                write_source_and_target(f, action)?;
            },
            ValidationError::PathTooLong {
                action,
                max_length,
                actual_length
            } => {
                writeln!(f, "The target path is too long (max: {max_length}, actual: {actual_length}) ")?;
                write_source_and_target(f, action)?;
            }
        }

        Ok(())
    }
}

fn write_source_and_target(
    f: &mut std::fmt::Formatter<'_>,
    action: &RenameAction,
) -> std::fmt::Result {
    writeln!(f, "\tsource: {}", action.source())?;
    writeln!(f, "\ttarget: {}", action.target())?;

    Ok(())
}

#[must_use]
pub fn validate_rename_actions(
    rename_actions: &'_ [RenameAction],
) -> Vec<ValidationError<'_>> {
    let mut errors = Vec::new();

    errors.extend(validate_double_separators(rename_actions));
    errors.extend(validate_collisions(rename_actions));
    errors.extend(validate_case_insensitive_collisions(rename_actions));
    errors.extend(validate_existing_files(rename_actions));
    errors.extend(validate_reserved_names(rename_actions));
    errors.extend(
        validate_forbidden_leading_or_trailing_characters_in_path_component(
            rename_actions,
            &FORBIDDEN_LEADING_OR_TRAILING_CHARACTERS,
        ),
    );
    errors.extend(validate_target_path_too_long(rename_actions));

    errors
}

fn validate_double_separators(
    rename_actions: &'_ [RenameAction],
) -> Vec<ValidationError<'_>> {
    let double_separator = std::path::MAIN_SEPARATOR_STR.repeat(2);

    rename_actions
        .iter()
        .filter(|rename_action| {
            rename_action.target().to_string().contains(&double_separator)
        })
        .map(ValidationError::DoubleSeparators)
        .collect()
}

fn validate_collisions(
    rename_actions: &'_ [RenameAction],
) -> Vec<ValidationError<'_>> {
    let mut map = HashMap::new();

    for rename_action in rename_actions {
        let _source = rename_action.source();

        map.entry(rename_action.target())
            .or_insert_with(Vec::new)
            .push(rename_action);
    }

    map.into_values()
        .filter(|actions| actions.len() > 1)
        .map(ValidationError::Collision)
        .collect()
}

fn validate_case_insensitive_collisions(
    rename_actions: &'_ [RenameAction],
) -> Vec<ValidationError<'_>> {
    let mut map = HashMap::new();

    for rename_action in rename_actions {
        map.entry(CaseInsensitivePathKey::new(rename_action.target()))
            .or_insert_with(Vec::new)
            .push(rename_action);
    }

    map.into_values()
        .filter(|actions| {
            actions.len() > 1
                && actions
                    .iter()
                    .map(|action| action.target().to_string())
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    > 1
        })
        .map(ValidationError::CaseInsensitiveCollision)
        .collect()
}

fn validate_existing_files(
    rename_actions: &'_ [RenameAction],
) -> Vec<ValidationError<'_>> {
    let sources =
        rename_actions.iter().map(RenameAction::source).collect::<Vec<_>>();

    rename_actions
        .iter()
        .filter(|m| {
            m.target().exists()
                && m.target() != m.source()
                && !sources.iter().any(|source| {
                    CaseInsensitivePathKey::new(source)
                        == CaseInsensitivePathKey::new(m.target())
                })
        })
        .map(ValidationError::TargetExists)
        .collect()
}

fn validate_reserved_names(
    rename_actions: &'_ [RenameAction],
) -> Vec<ValidationError<'_>> {
    rename_actions
        .iter()
        .flat_map(|action| {
            action.target().components().filter_map(|component| {
                if let Utf8Component::Normal(component_name) = component
                    && is_windows_reserved_name(component_name)
                {
                    Some(ValidationError::ReservedName {
                        action,
                        component: component_name,
                    })
                } else {
                    None
                }
            })
        })
        .collect()
}

fn is_windows_reserved_name(component_name: &str) -> bool {
    let stem =
        component_name.split_once('.').map_or(component_name, |(stem, _)| stem);
    let normalized = stem.to_ascii_uppercase();

    matches!(normalized.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || is_reserved_numbered_device_name(&normalized, "COM")
        || is_reserved_numbered_device_name(&normalized, "LPT")
}

fn is_reserved_numbered_device_name(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .and_then(|suffix| suffix.parse::<u8>().ok())
        .is_some_and(|number| (1..=9).contains(&number))
}

fn validate_forbidden_leading_or_trailing_characters_in_path_component<'a>(
    rename_actions: &'a [RenameAction],
    forbidden_characters: &'static [ForbiddenLeadingOrTrailingChar],
) -> Vec<ValidationError<'a>> {
    let forbidden_leading_characters: Vec<&str> = forbidden_characters
        .iter()
        .filter(|f| f.leading())
        .map(ForbiddenLeadingOrTrailingChar::char)
        .collect();

    let forbidden_trailing_characters: Vec<&str> = forbidden_characters
        .iter()
        .filter(|f| f.trailing())
        .map(ForbiddenLeadingOrTrailingChar::char)
        .collect();

    rename_actions
        .iter()
        .flat_map(|action| {
            action.target().components().filter_map(|component| {
                if let Utf8Component::Normal(component_name) = component {
                    let leading = matching_characters(
                        &forbidden_leading_characters,
                        component_name,
                        |component_name, character| {
                            component_name.starts_with(character)
                        },
                    );
                    let trailing = matching_characters(
                        &forbidden_trailing_characters,
                        component_name,
                        |component_name, character| {
                            component_name.ends_with(character)
                        },
                    );

                    if !leading.is_empty() || !trailing.is_empty() {
                        Some(ValidationError::ForbiddenCharacterLeadingOrTrailingPathComponent  {
                            action,
                            forbidden_leading_characters: if leading.is_empty() { None } else { Some(leading) },
                            forbidden_trailing_characters: if trailing.is_empty() { None } else { Some(trailing) },
                            component: component_name,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
        })
        .collect()
}

fn matching_characters<'a>(
    forbidden_characters: &[&'a str],
    component_name: &str,
    matches: impl Fn(&str, &str) -> bool,
) -> Vec<&'a str> {
    forbidden_characters
        .iter()
        .copied()
        .filter(|character| matches(component_name, character))
        .collect()
}

fn validate_target_path_too_long(
    rename_actions: &'_ [RenameAction],
) -> Vec<ValidationError<'_>> {
    rename_actions
        .iter()
        .map(|a| (a, a.target().to_string().len()))
        .filter(|(_, len)| *len >= MAX_PATH_LENGTH)
        .map(|(a, len)| {
            ValidationError::PathTooLong {
                action: a,
                max_length: MAX_PATH_LENGTH,
                actual_length: len,
            }
        })
        .collect()
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::util::Utf8File;

    fn assert_valid(rename_actions: &[RenameAction]) {
        assert!(validate_rename_actions(rename_actions).is_empty());
    }

    fn assert_single_error(
        rename_actions: &'_ [RenameAction],
    ) -> ValidationError<'_> {
        let mut errors = validate_rename_actions(rename_actions);

        assert!(errors.len() == 1);

        errors.pop().unwrap()
    }

    fn assert_n_errors(
        rename_actions: &'_ [RenameAction],
        n: usize,
    ) -> Vec<ValidationError<'_>> {
        let errors = validate_rename_actions(rename_actions);

        let len = errors.len();

        assert_eq!(len, n, "expected {n} errors, got {len}.");

        errors
    }

    fn assert_double_separator_error(rename_actions: &[RenameAction]) {
        let error = assert_single_error(rename_actions);

        assert!(matches!(error, ValidationError::DoubleSeparators(..)));
    }

    #[test]
    #[cfg(unix)]
    // TODO Test fails on Windows
    fn test_validate_double_separators() {
        let valid = [RenameAction::new(
            Utf8File::new("/a/b/c/").unwrap(),
            Utf8File::new("/d/e/f/").unwrap(),
        )];

        assert_valid(&valid);

        let leading = [RenameAction::new(
            Utf8File::new("/a/b/c/").unwrap(),
            Utf8File::new("//d/e/f/").unwrap(),
        )];

        assert_double_separator_error(&leading);

        let middle = [RenameAction::new(
            Utf8File::new("/a/b/c/").unwrap(),
            Utf8File::new("/d//e/f/").unwrap(),
        )];

        assert_double_separator_error(&middle);

        let trailing = [RenameAction::new(
            Utf8File::new("/a/b/c/").unwrap(),
            Utf8File::new("/d/e/f//").unwrap(),
        )];

        assert_double_separator_error(&trailing);
    }

    #[test]
    fn test_validate_collision() {
        let valid = [
            RenameAction::new(
                Utf8File::new("/a/b/c/").unwrap(),
                Utf8File::new("/d/e/f/").unwrap(),
            ),
            RenameAction::new(
                Utf8File::new("/g/h/i/").unwrap(),
                Utf8File::new("/j/k/l/").unwrap(),
            ),
        ];

        assert_valid(&valid);

        let colliding = [
            RenameAction::new(
                Utf8File::new("/a/b/c/").unwrap(),
                Utf8File::new("/d/e/f/").unwrap(),
            ),
            RenameAction::new(
                Utf8File::new("/g/h/i/").unwrap(),
                Utf8File::new("/d/e/f/").unwrap(),
            ),
        ];

        let error = assert_single_error(&colliding);
        assert!(matches!(error, ValidationError::Collision(..)));
    }

    #[test]
    fn test_validate_case_insensitive_collision() {
        let colliding = [
            RenameAction::new(
                Utf8File::new("input/a.mp3").unwrap(),
                Utf8File::new("music/Track.mp3").unwrap(),
            ),
            RenameAction::new(
                Utf8File::new("input/b.mp3").unwrap(),
                Utf8File::new("music/track.mp3").unwrap(),
            ),
        ];

        let error = assert_single_error(&colliding);
        assert!(matches!(error, ValidationError::CaseInsensitiveCollision(..)));
    }

    #[test]
    fn test_validate_reserved_windows_names() {
        let reserved = [
            RenameAction::new(
                Utf8File::new("input/a.mp3").unwrap(),
                Utf8File::new("music/CON.mp3").unwrap(),
            ),
            RenameAction::new(
                Utf8File::new("input/b.mp3").unwrap(),
                Utf8File::new("music/NUL/track.mp3").unwrap(),
            ),
            RenameAction::new(
                Utf8File::new("input/c.mp3").unwrap(),
                Utf8File::new("music/lpt1.flac").unwrap(),
            ),
        ];

        let errors = assert_n_errors(&reserved, 3);

        assert!(
            errors
                .into_iter()
                .all(|e| matches!(e, ValidationError::ReservedName { .. }))
        );
    }

    #[test]
    fn test_validate_forbidden() {
        let valid = [
            RenameAction::new(
                Utf8File::new("/a/b/c/").unwrap(),
                Utf8File::new("/d/e/f/").unwrap(),
            ),
            RenameAction::new(
                Utf8File::new("/a/b/c/").unwrap(),
                Utf8File::new("/d/.e/f/").unwrap(),
            ),
        ];

        assert_valid(&valid);

        let forbidden_leading = [
            RenameAction::new(
                Utf8File::new("/a/b/c/").unwrap(),
                Utf8File::new("/d/ e/f/").unwrap(),
            ),
            RenameAction::new(
                Utf8File::new("/a/b/c/").unwrap(),
                Utf8File::new("/d/e /f/").unwrap(),
            ),
            RenameAction::new(
                Utf8File::new("/a/b/c/").unwrap(),
                Utf8File::new("/d/e./f/").unwrap(),
            ),
        ];

        let errors = assert_n_errors(&forbidden_leading, 3);

        assert!(errors.into_iter().all(|e| matches!(e, ValidationError::ForbiddenCharacterLeadingOrTrailingPathComponent { .. })));
    }

    #[test]
    fn validate_path_too_long() {
        let valid = [RenameAction::new(
            Utf8File::new("/a/b/c/").unwrap(),
            Utf8File::new("/d/e/f/").unwrap(),
        )];

        assert_valid(&valid);

        let too_long = [RenameAction::new(
            Utf8File::new("/a/b/c/").unwrap(),
            Utf8File::new(format!("/d{}/f/", "/e".repeat(128))).unwrap(),
        )];

        let error = assert_single_error(&too_long);

        assert!(matches!(error, ValidationError::PathTooLong { .. }));

        let exact = [RenameAction::new(
            Utf8File::new("/a/b/c/").unwrap(),
            Utf8File::new(format!("/d{}/f", "/e".repeat(126))).unwrap(),
        )];

        let error = assert_single_error(&exact);

        assert!(matches!(error, ValidationError::PathTooLong {
            actual_length: 256,
            ..
        }));
    }
}
