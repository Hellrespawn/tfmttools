use crate::action::RenameAction;

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
                component,
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

                match (
                    forbidden_leading_characters,
                    forbidden_trailing_characters,
                ) {
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
                actual_length,
            } => {
                writeln!(f, "The target path is too long (max: {max_length}, actual: {actual_length}) ")?;
                write_source_and_target(f, action)?;
            },
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
