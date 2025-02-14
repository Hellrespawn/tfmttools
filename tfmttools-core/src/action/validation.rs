use std::collections::HashMap;
use std::sync::LazyLock;

use camino::{Utf8Component, Utf8Path};

use crate::action::RenameAction;
use crate::MAX_PATH_LENGTH;

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
    TargetExists(&'e RenameAction),
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
            ValidationError::TargetExists(action) => {
                writeln!(f, "The target file already exists.")?;
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

pub fn validate_rename_actions(
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_double_separators(rename_actions));
    errors.extend(validate_collisions(rename_actions));
    errors.extend(validate_existing_files(rename_actions));
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
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
    rename_actions
        .iter()
        .filter(|rename_action| {
            rename_action
                .target()
                .to_string()
                .contains(&std::path::MAIN_SEPARATOR_STR.repeat(2))
        })
        .map(ValidationError::DoubleSeparators)
        .collect()
}

fn validate_collisions(
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
    let mut map = HashMap::new();

    for rename_action in rename_actions {
        let _source = rename_action.source();

        map.entry(rename_action.target())
            .or_insert_with(Vec::new)
            .push(rename_action);
    }

    let collisions: HashMap<&Utf8Path, Vec<&RenameAction>> =
        map.into_iter().filter(|(_, v)| v.len() > 1).collect();

    collisions.into_values().map(ValidationError::Collision).collect()
}

// Impossible to unit test, therefore not included below.
fn validate_existing_files(
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
    rename_actions
        .iter()
        .filter(|m| m.target().exists() && m.target() != m.source())
        .map(ValidationError::TargetExists)
        .collect()
}

fn validate_forbidden_leading_or_trailing_characters_in_path_component<'a>(
    rename_actions: &'a [RenameAction],
    forbidden_characters: &'static [ForbiddenLeadingOrTrailingChar],
) -> Vec<ValidationError<'a>> {
    let forbidden_leading_characters: Vec<&str> = forbidden_characters
        .iter()
        .filter(|f| f.leading())
        .map(|f| f.char())
        .collect();

    let forbidden_trailing_characters: Vec<&str> = forbidden_characters
        .iter()
        .filter(|f| f.trailing())
        .map(|f| f.char())
        .collect();

    rename_actions
        .iter()
        .flat_map(|action| {
            action.target().components().filter_map(|component| {
                if let Utf8Component::Normal(component_name) = component {

                    let leading = forbidden_leading_characters.iter().any(|c| component_name.starts_with(c));
                    let trailing = forbidden_trailing_characters.iter().any(|c| component_name.ends_with(c));

                    if leading || trailing {
                        Some(ValidationError::ForbiddenCharacterLeadingOrTrailingPathComponent  {
                            action,
                            forbidden_leading_characters: if leading { Some(forbidden_leading_characters.clone()) } else { None},
                            forbidden_trailing_characters: if trailing { Some(forbidden_trailing_characters.clone()) } else { None},
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

fn validate_target_path_too_long(
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
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

    use camino::Utf8PathBuf;

    use super::*;

    fn assert_valid(rename_actions: &[RenameAction]) {
        assert!(validate_rename_actions(rename_actions).is_empty());
    }

    fn assert_single_error(rename_actions: &[RenameAction]) -> ValidationError {
        let mut errors = validate_rename_actions(rename_actions);

        assert!(errors.len() == 1);

        errors.pop().unwrap()
    }

    fn assert_n_errors(
        rename_actions: &[RenameAction],
        n: usize,
    ) -> Vec<ValidationError> {
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
            Utf8PathBuf::from("/a/b/c/"),
            Utf8PathBuf::from("/d/e/f/"),
        )];

        assert_valid(&valid);

        let leading = [RenameAction::new(
            Utf8PathBuf::from("/a/b/c/"),
            Utf8PathBuf::from("//d/e/f/"),
        )];

        assert_double_separator_error(&leading);

        let middle = [RenameAction::new(
            Utf8PathBuf::from("/a/b/c/"),
            Utf8PathBuf::from("/d//e/f/"),
        )];

        assert_double_separator_error(&middle);

        let trailing = [RenameAction::new(
            Utf8PathBuf::from("/a/b/c/"),
            Utf8PathBuf::from("/d/e/f//"),
        )];

        assert_double_separator_error(&trailing);
    }

    #[test]
    fn test_validate_collision() {
        let valid = [
            RenameAction::new(
                Utf8PathBuf::from("/a/b/c/"),
                Utf8PathBuf::from("/d/e/f/"),
            ),
            RenameAction::new(
                Utf8PathBuf::from("/g/h/i/"),
                Utf8PathBuf::from("/j/k/l/"),
            ),
        ];

        assert_valid(&valid);

        let colliding = [
            RenameAction::new(
                Utf8PathBuf::from("/a/b/c/"),
                Utf8PathBuf::from("/d/e/f/"),
            ),
            RenameAction::new(
                Utf8PathBuf::from("/g/h/i/"),
                Utf8PathBuf::from("/d/e/f/"),
            ),
        ];

        let error = assert_single_error(&colliding);
        assert!(matches!(error, ValidationError::Collision(..)));
    }

    #[test]
    fn test_validate_forbidden() {
        let valid = [
            RenameAction::new(
                Utf8PathBuf::from("/a/b/c/"),
                Utf8PathBuf::from("/d/e/f/"),
            ),
            RenameAction::new(
                Utf8PathBuf::from("/a/b/c/"),
                Utf8PathBuf::from("/d/.e/f/"),
            ),
        ];

        assert_valid(&valid);

        let forbidden_leading = [
            RenameAction::new(
                Utf8PathBuf::from("/a/b/c/"),
                Utf8PathBuf::from("/d/ e/f/"),
            ),
            RenameAction::new(
                Utf8PathBuf::from("/a/b/c/"),
                Utf8PathBuf::from("/d/e /f/"),
            ),
            RenameAction::new(
                Utf8PathBuf::from("/a/b/c/"),
                Utf8PathBuf::from("/d/e./f/"),
            ),
        ];

        let errors = assert_n_errors(&forbidden_leading, 3);

        assert!(errors.into_iter().all(|e| matches!(e, ValidationError::ForbiddenCharacterLeadingOrTrailingPathComponent { .. })))
    }

    #[test]
    fn validate_path_too_long() {
        let valid = [RenameAction::new(
            Utf8PathBuf::from("/a/b/c/"),
            Utf8PathBuf::from("/d/e/f/"),
        )];

        assert_valid(&valid);

        let too_long = [RenameAction::new(
            Utf8PathBuf::from("/a/b/c/"),
            Utf8PathBuf::from(format!("/d{}/f/", "/e".repeat(128))),
        )];

        let error = assert_single_error(&too_long);

        assert!(matches!(error, ValidationError::PathTooLong { .. }));

        let exact = [RenameAction::new(
            Utf8PathBuf::from("/a/b/c/"),
            Utf8PathBuf::from(format!("/d{}/f", "/e".repeat(126))),
        )];

        let error = assert_single_error(&exact);

        assert!(matches!(error, ValidationError::PathTooLong {
            actual_length: 256,
            ..
        }));
    }
}
