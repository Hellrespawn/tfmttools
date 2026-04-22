use std::sync::LazyLock;

use camino::Utf8Component;

use super::errors::ValidationError;
use crate::action::RenameAction;

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

pub(super) struct ForbiddenLeadingOrTrailingChar<'f> {
    char: &'f str,
    leading: bool,
    trailing: bool,
}

impl ForbiddenLeadingOrTrailingChar<'_> {
    fn char(&self) -> &str {
        self.char
    }

    fn leading(&self) -> bool {
        self.leading
    }

    fn trailing(&self) -> bool {
        self.trailing
    }
}

pub(super) static FORBIDDEN_LEADING_OR_TRAILING_CHARACTERS: LazyLock<
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

pub(super) fn validate_reserved_names(
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

pub(super) fn validate_forbidden_leading_or_trailing_characters_in_path_component<
    'a,
>(
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
