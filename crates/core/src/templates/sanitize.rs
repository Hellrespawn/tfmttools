use crate::action::FORBIDDEN_CHARACTERS;

/// Replace forbidden filename characters and remove trailing dots.
/// Callers decide whether to trim surrounding whitespace.
#[must_use]
pub fn sanitize_tag_value(value: &str) -> String {
    let value = FORBIDDEN_CHARACTERS.iter().fold(
        value.to_owned(),
        |text, forbidden| {
            text.replace(
                forbidden.char(),
                forbidden.replacement().unwrap_or(""),
            )
        },
    );
    value.trim_end_matches('.').to_owned()
}
