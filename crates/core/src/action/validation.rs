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

use crate::action::RenameAction;

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
