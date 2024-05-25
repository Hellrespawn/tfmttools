use std::collections::HashMap;

use camino::Utf8Path;

use crate::action::RenameAction;

pub enum ValidationError<'e> {
    DoubleSeparators(&'e RenameAction),
    Collision(Vec<&'e RenameAction>),
    TargetExists(&'e RenameAction),
}

pub fn validate_rename_actions(
    rename_actions: &[RenameAction],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_double_separators(rename_actions));
    errors.extend(validate_collisions(rename_actions));
    errors.extend(validate_existing_files(rename_actions));

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
        .filter(|m| m.target().exists())
        .map(ValidationError::TargetExists)
        .collect()
}
