use super::errors::ValidationError;
use crate::MAX_PATH_LENGTH;
use crate::action::RenameAction;

pub(super) fn validate_target_path_too_long(
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
