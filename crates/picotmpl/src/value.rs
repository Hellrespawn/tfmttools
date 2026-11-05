use crate::{Diagnostic, Span};

#[derive(Clone, Debug)]
pub(crate) enum Value {
    Absent,
    Text(String),
    Integer(i64),
    Boolean(bool),
    Path(Vec<String>),
    Sequence(Vec<(Self, Span)>),
    Root,
    Separator,
}

impl Value {
    pub(crate) fn is_present(&self) -> bool {
        match self {
            Self::Absent => false,
            Self::Text(text) => !text.is_empty(),
            Self::Integer(_) | Self::Root | Self::Separator => true,
            Self::Boolean(value) => *value,
            Self::Path(components) => !components.is_empty(),
            Self::Sequence(values) => {
                values.iter().any(|(value, _)| value.is_present())
            },
        }
    }

    pub(crate) fn into_text(self, span: Span) -> Result<String, Diagnostic> {
        match self {
            Self::Absent => Ok(String::new()),
            Self::Text(text) => Ok(text),
            Self::Integer(value) => Ok(value.to_string()),
            Self::Sequence(values) => {
                let mut text = String::new();
                for (value, location) in values {
                    text.push_str(&value.into_text(location)?);
                }
                Ok(text)
            },
            Self::Boolean(_) => {
                Err(Diagnostic::new("Cannot format a boolean value", span))
            },
            _ => Err(Diagnostic::new("Cannot format a path value", span)),
        }
    }
}

impl From<Scalar> for Value {
    fn from(value: Scalar) -> Self {
        match value {
            Scalar::Text(text) => Self::Text(text),
            Scalar::Integer(integer) => Self::Integer(integer),
        }
    }
}

/// A caller supplied metadata value. Missing values use `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scalar {
    Text(String),
    Integer(i64),
}
