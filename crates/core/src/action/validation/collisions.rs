use std::collections::HashMap;

use super::errors::ValidationError;
use crate::action::{CaseInsensitivePathKey, RenameAction};
use crate::util::Utf8PathExt;

pub(super) fn validate_double_separators(
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

pub(super) fn validate_collisions(
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

pub(super) fn validate_case_insensitive_collisions(
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

pub(super) fn validate_existing_files(
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
