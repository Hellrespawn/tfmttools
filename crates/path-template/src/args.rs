use crate::value::Value;
use crate::{Diagnostic, Span};

/// Characters the caller rejects in argument component text.
#[derive(Clone, Debug)]
pub struct ArgumentPolicy {
    pub(crate) forbidden: Vec<char>,
}

impl ArgumentPolicy {
    #[must_use]
    pub fn new(forbidden: &[char]) -> Self {
        Self { forbidden: forbidden.to_vec() }
    }
}

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

pub(crate) fn coerce(
    spec: &ArgSpec,
    raw: &str,
    policy: &ArgumentPolicy,
) -> Result<Value, Diagnostic> {
    match spec.kind {
        ArgKind::Int => {
            raw.parse::<i64>().map(Value::Integer).map_err(|_| {
                Diagnostic::new(
                    format!("Argument '{}' requires an integer", spec.name),
                    spec.span,
                )
            })
        },
        ArgKind::String => {
            validate_text(spec, raw, policy, false)?;
            Ok(Value::Text(raw.to_owned()))
        },
        ArgKind::Path => {
            let mut components = Vec::new();
            for component in
                raw.split(['/', '\\']).filter(|part| !part.is_empty())
            {
                validate_text(spec, component, policy, true)?;
                components.push(component.to_owned());
            }
            Ok(Value::Path(components))
        },
    }
}

fn validate_text(
    spec: &ArgSpec,
    text: &str,
    policy: &ArgumentPolicy,
    path: bool,
) -> Result<(), Diagnostic> {
    if let Some(character) = text
        .chars()
        .find(|c| ['/', '\\'].contains(c) || policy.forbidden.contains(c))
    {
        let component = if path {
            format!(" in component {text:?}")
        } else {
            String::new()
        };
        return Err(Diagnostic::new(
            format!(
                "Argument '{}' contains forbidden character {character:?}{component}",
                spec.name
            ),
            spec.span,
        ));
    }
    Ok(())
}
