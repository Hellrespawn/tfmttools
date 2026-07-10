mod collisions;
mod errors;
mod forbidden;
mod path_rules;

use collisions::{
    validate_case_insensitive_collisions, validate_collisions,
    validate_double_separators, validate_existing_files,
};
use errors::ValidationError;
pub use forbidden::FORBIDDEN_CHARACTERS;
use forbidden::{
    FORBIDDEN_LEADING_OR_TRAILING_CHARACTERS,
    validate_forbidden_leading_or_trailing_characters_in_path_component,
    validate_reserved_names,
};
use path_rules::validate_target_path_too_long;

use crate::action::{CaseInsensitivePathSet, RenameAction};

#[must_use]
pub fn validate_rename_actions<'a>(
    rename_actions: &'a [RenameAction],
    existing_targets: &CaseInsensitivePathSet,
) -> Vec<ValidationError<'a>> {
    let mut errors = Vec::new();

    errors.extend(validate_double_separators(rename_actions));
    errors.extend(validate_collisions(rename_actions));
    errors.extend(validate_case_insensitive_collisions(rename_actions));
    errors.extend(validate_existing_files(rename_actions, existing_targets));
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

#[cfg(test)]
mod test {

    use super::*;
    use crate::action::CaseInsensitivePathSet;
    use crate::util::Utf8File;

    fn assert_valid(
        rename_actions: &[RenameAction],
        existing_targets: &CaseInsensitivePathSet,
    ) {
        assert!(
            validate_rename_actions(rename_actions, existing_targets).is_empty()
        );
    }

    fn assert_single_error(
        rename_actions: &'_ [RenameAction],
    ) -> ValidationError<'_> {
        let mut errors = validate_rename_actions(
            rename_actions,
            &CaseInsensitivePathSet::new(),
        );

        assert!(errors.len() == 1);

        errors.pop().unwrap()
    }

    fn assert_n_errors(
        rename_actions: &'_ [RenameAction],
        n: usize,
    ) -> Vec<ValidationError<'_>> {
        let errors = validate_rename_actions(
            rename_actions,
            &CaseInsensitivePathSet::new(),
        );

        let len = errors.len();

        assert_eq!(len, n, "expected {n} errors, got {len}.");

        errors
    }

    #[cfg(unix)]
    fn assert_double_separator_error(rename_actions: &[RenameAction]) {
        let error = assert_single_error(rename_actions);

        assert!(matches!(error, ValidationError::DoubleSeparators(..)));
    }

    #[test]
    #[cfg(unix)]
    // TODO Test fails on Windows
    fn test_validate_double_separators() {
        let valid = [RenameAction::new(
            Utf8File::new("/a/b/c/"),
            Utf8File::new("/d/e/f/"),
        )];

        assert_valid(&valid, &CaseInsensitivePathSet::new());

        let leading = [RenameAction::new(
            Utf8File::new("/a/b/c/"),
            Utf8File::new("//d/e/f/"),
        )];

        assert_double_separator_error(&leading);

        let middle = [RenameAction::new(
            Utf8File::new("/a/b/c/"),
            Utf8File::new("/d//e/f/"),
        )];

        assert_double_separator_error(&middle);

        let trailing = [RenameAction::new(
            Utf8File::new("/a/b/c/"),
            Utf8File::new("/d/e/f//"),
        )];

        assert_double_separator_error(&trailing);
    }

    #[test]
    fn test_validate_collision() {
        let valid = [
            RenameAction::new(
                Utf8File::new("/a/b/c/"),
                Utf8File::new("/d/e/f/"),
            ),
            RenameAction::new(
                Utf8File::new("/g/h/i/"),
                Utf8File::new("/j/k/l/"),
            ),
        ];

        assert_valid(&valid, &CaseInsensitivePathSet::new());

        let colliding = [
            RenameAction::new(
                Utf8File::new("/a/b/c/"),
                Utf8File::new("/d/e/f/"),
            ),
            RenameAction::new(
                Utf8File::new("/g/h/i/"),
                Utf8File::new("/d/e/f/"),
            ),
        ];

        let error = assert_single_error(&colliding);
        assert!(matches!(error, ValidationError::Collision(..)));
    }

    #[test]
    fn test_validate_case_insensitive_collision() {
        let colliding = [
            RenameAction::new(
                Utf8File::new("input/a.mp3"),
                Utf8File::new("music/Track.mp3"),
            ),
            RenameAction::new(
                Utf8File::new("input/b.mp3"),
                Utf8File::new("music/track.mp3"),
            ),
        ];

        let error = assert_single_error(&colliding);
        assert!(matches!(error, ValidationError::CaseInsensitiveCollision(..)));
    }

    #[test]
    fn test_validate_reserved_windows_names() {
        let reserved = [
            RenameAction::new(
                Utf8File::new("input/a.mp3"),
                Utf8File::new("music/CON.mp3"),
            ),
            RenameAction::new(
                Utf8File::new("input/b.mp3"),
                Utf8File::new("music/NUL/track.mp3"),
            ),
            RenameAction::new(
                Utf8File::new("input/c.mp3"),
                Utf8File::new("music/lpt1.flac"),
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
                Utf8File::new("/a/b/c/"),
                Utf8File::new("/d/e/f/"),
            ),
            RenameAction::new(
                Utf8File::new("/a/b/c/"),
                Utf8File::new("/d/.e/f/"),
            ),
        ];

        assert_valid(&valid, &CaseInsensitivePathSet::new());

        let forbidden_leading = [
            RenameAction::new(
                Utf8File::new("/a/b/c/"),
                Utf8File::new("/d/ e/f/"),
            ),
            RenameAction::new(
                Utf8File::new("/a/b/c/"),
                Utf8File::new("/d/e /f/"),
            ),
            RenameAction::new(
                Utf8File::new("/a/b/c/"),
                Utf8File::new("/d/e./f/"),
            ),
        ];

        let errors = assert_n_errors(&forbidden_leading, 3);

        assert!(errors.into_iter().all(|e| matches!(e, ValidationError::ForbiddenCharacterLeadingOrTrailingPathComponent { .. })));
    }

    #[test]
    fn validate_path_too_long() {
        let valid = [RenameAction::new(
            Utf8File::new("/a/b/c/"),
            Utf8File::new("/d/e/f/"),
        )];

        assert_valid(&valid, &CaseInsensitivePathSet::new());

        let too_long = [RenameAction::new(
            Utf8File::new("/a/b/c/"),
            Utf8File::new(format!("/d{}/f/", "/e".repeat(128))),
        )];

        let error = assert_single_error(&too_long);

        assert!(matches!(error, ValidationError::PathTooLong { .. }));

        let exact = [RenameAction::new(
            Utf8File::new("/a/b/c/"),
            Utf8File::new(format!("/d{}/f", "/e".repeat(126))),
        )];

        let error = assert_single_error(&exact);

        assert!(matches!(error, ValidationError::PathTooLong {
            actual_length: 256,
            ..
        }));
    }

    #[test]
    fn test_validate_target_exists() {
        let actions = [RenameAction::new(
            Utf8File::new("input/a.mp3"),
            Utf8File::new("music/b.mp3"),
        )];

        let mut existing = CaseInsensitivePathSet::new();
        existing.insert("music/b.mp3");

        let errors = validate_rename_actions(&actions, &existing);

        assert_eq!(errors.len(), 1);
        assert!(matches!(errors[0], ValidationError::TargetExists(_)));
    }

    #[test]
    fn test_validate_target_exists_ignores_unrelated_paths() {
        let actions = [RenameAction::new(
            Utf8File::new("input/a.mp3"),
            Utf8File::new("music/b.mp3"),
        )];

        let existing = CaseInsensitivePathSet::new();

        assert_valid(&actions, &existing);
    }
}
