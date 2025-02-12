use std::collections::HashMap;

use camino::{Utf8Component, Utf8Path};

use crate::action::RenameAction;

#[derive(Debug)]
pub enum ValidationError<'e> {
    DoubleSeparators(&'e RenameAction),
    Collision(Vec<&'e RenameAction>),
    TargetExists(&'e RenameAction),
    WhitespaceInDirectoryName {
        action: &'e RenameAction,
        component: &'e str,
        leading: bool,
        trailing: bool,
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
                writeln!(f, "\tsource: {}", action.source())?;
                writeln!(f, "\ttarget: {}", action.target())?;
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
                writeln!(f, "\tsource: {}", action.source())?;
                writeln!(f, "\ttarget: {}", action.target())?;
            },
            ValidationError::WhitespaceInDirectoryName {
                action,
                component,
                leading,
                trailing,
            } => {
                write!(
                    f,
                    "The '{component}' directory in the target path has "
                )?;
                match (leading, trailing) {
                    (true, true) => write!(f, "leading and trailing"),
                    (true, false) => write!(f, "leading"),
                    (false, true) => write!(f, "trailing"),
                    (false, false) => unreachable!(),
                }?;
                writeln!(f, " whitespace in it's name.")?;
                writeln!(f, "\tsource: {}", action.source())?;
                writeln!(f, "\ttarget: {}", action.target())?;
            },
        }

        Ok(())
    }
}

pub fn validate_rename_actions(
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_double_separators(rename_actions));
    errors.extend(validate_collisions(rename_actions));
    errors.extend(validate_existing_files(rename_actions));
    errors.extend(validate_whitespace_in_directory_name(rename_actions));

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

fn validate_existing_files(
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
    rename_actions
        .iter()
        .filter(|m| m.target().exists() && m.target() != m.source())
        .map(ValidationError::TargetExists)
        .collect()
}

fn validate_whitespace_in_directory_name(
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
    rename_actions
        .iter()
        .flat_map(|action| {
            action.target().components().filter_map(|component| {
                if let Utf8Component::Normal(component_name) = component {
                    let leading = component_name != component_name.trim_start();
                    let trailing = component_name != component_name.trim_end();

                    if leading || trailing {
                        Some(ValidationError::WhitespaceInDirectoryName {
                            action,
                            component: component_name,
                            leading,
                            trailing,
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
