use crate::action::Move;
use camino::Utf8Path;
use std::collections::HashMap;

pub(crate) enum ValidationError<'e> {
    DoubleSeparators(&'e Move),
    Collision(Vec<&'e Move>),
    TargetExists(&'e Move),
}

pub(crate) fn validate_move_actions(
    move_actions: &[Move],
) -> Vec<ValidationError> {
    let mut errors = Vec::new();

    errors.extend(validate_double_separators(move_actions));
    errors.extend(validate_collisions(move_actions));
    errors.extend(validate_existing_files(move_actions));

    errors
}

fn validate_double_separators(move_actions: &[Move]) -> Vec<ValidationError> {
    move_actions
        .iter()
        .filter(|move_action| {
            move_action
                .target()
                .to_string()
                .contains(&std::path::MAIN_SEPARATOR_STR.repeat(2))
        })
        .map(|p| ValidationError::DoubleSeparators(p))
        .collect()
}

fn validate_collisions(move_actions: &[Move]) -> Vec<ValidationError> {
    let mut map = HashMap::new();

    for move_action in move_actions {
        let source = move_action.source();

        map.entry(move_action.target())
            .or_insert_with(Vec::new)
            .push(move_action);
    }

    let collisions: HashMap<&Utf8Path, Vec<&Move>> =
        map.into_iter().filter(|(_, v)| v.len() > 1).collect();

    collisions.into_values().map(|v| ValidationError::Collision(v)).collect()
}

fn validate_existing_files(move_actions: &[Move]) -> Vec<ValidationError> {
    move_actions
        .iter()
        .filter(|m| m.target().exists())
        .map(|move_action| ValidationError::TargetExists(move_action))
        .collect()
}
