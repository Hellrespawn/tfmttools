use crate::ast::ComparisonOperator;
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

    fn into_comparable(self, span: Span) -> Result<Self, Diagnostic> {
        let value = match self {
            Self::Sequence(_) => Self::Text(self.into_text(span)?),
            Self::Path(_) | Self::Root | Self::Separator => {
                return Err(Diagnostic::new(
                    "Cannot compare a path value",
                    span,
                ));
            },
            value => value,
        };
        Ok(match value {
            Self::Text(text) if text.is_empty() => Self::Absent,
            value => value,
        })
    }

    pub(crate) fn compare(
        self,
        other: Self,
        operator: ComparisonOperator,
        span: Span,
    ) -> Result<bool, Diagnostic> {
        let left = self.into_comparable(span)?;
        let right = other.into_comparable(span)?;
        if operator.is_ordering() {
            if matches!(left, Self::Absent) || matches!(right, Self::Absent) {
                return Ok(false);
            }
            let (Self::Integer(left), Self::Integer(right)) = (left, right)
            else {
                return Err(Diagnostic::new(
                    "Ordering requires two integers",
                    span,
                ));
            };
            return Ok(match operator {
                ComparisonOperator::Less => left < right,
                ComparisonOperator::LessEqual => left <= right,
                ComparisonOperator::Greater => left > right,
                ComparisonOperator::GreaterEqual => left >= right,
                _ => unreachable!(),
            });
        }
        let equal = match (left, right) {
            (Self::Absent, Self::Absent) => true,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Integer(left), Self::Integer(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            _ => false,
        };
        Ok(match operator {
            ComparisonOperator::Equal => equal,
            ComparisonOperator::NotEqual => !equal,
            _ => unreachable!(),
        })
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
