use crate::value::Value;
use crate::{Diagnostic, Span};

/// The declared argument's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgKind {
    String,
    Int,
    Path,
}

/// An argument declaration. No default means the argument is required.
#[derive(Clone, Debug)]
pub struct ArgSpec {
    pub name: String,
    pub kind: ArgKind,
    pub default: Option<String>,
    pub description: Option<String>,
    pub span: Span,
}

pub(crate) fn coerce(spec: &ArgSpec, raw: &str) -> Result<Value, Diagnostic> {
    if let Some(character) = prohibited_character(raw) {
        return Err(Diagnostic::new(
            format!(
                "Argument '{}' contains prohibited character {character:?}",
                spec.name
            ),
            spec.span,
        ));
    }
    match spec.kind {
        ArgKind::Int => {
            raw.parse::<i64>().map(Value::Integer).map_err(|_| {
                Diagnostic::new(
                    format!("Argument '{}' requires an integer", spec.name),
                    spec.span,
                )
            })
        },
        ArgKind::String => Ok(Value::Text(raw.to_owned())),
        ArgKind::Path => {
            let mut components = Vec::new();
            for component in
                raw.split(['/', '\\']).filter(|part| !part.is_empty())
            {
                components.push(component.to_owned());
            }
            Ok(Value::Path(components))
        },
    }
}

/// Controls, line separators, and bidi formatting controls are never filename text.
#[must_use]
pub fn prohibited_character(text: &str) -> Option<char> {
    text.chars().find(|c| {
        c.is_control()
            || matches!(c, '\u{2028}' | '\u{2029}' | '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
    })
}
