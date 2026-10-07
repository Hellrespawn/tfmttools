use std::collections::HashMap;

use crate::ast::{Alternative, Expression, Formatter, Reference};
use crate::value::Value;
use crate::{BoundScript, Diagnostic, RenderError, RenderedPath, Scalar, format};

pub(crate) fn render<E>(
    bound: &BoundScript,
    mut resolve: impl FnMut(&str) -> Result<Option<Scalar>, E>,
) -> Result<RenderedPath, RenderError<E>> {
    let mut text = String::new();
    evaluate(&bound.script.inner.path, &bound.arguments, &mut resolve, &mut text)?;
    Ok(RenderedPath { components: vec![text], rooted: false })
}

fn reference_value<E>(
    reference: &Reference,
    arguments: &HashMap<String, Value>,
    resolve: &mut impl FnMut(&str) -> Result<Option<Scalar>, E>,
) -> Result<Option<Value>, RenderError<E>> {
    if reference.tag {
        resolve(&reference.name)
            .map(|value| value.map(Value::from))
            .map_err(|source| RenderError::Resolver {
                name: reference.name.clone(),
                span: reference.span,
                source,
            })
    } else {
        Ok(arguments.get(&reference.name).cloned())
    }
}

fn evaluate<E>(
    expressions: &[Expression],
    arguments: &HashMap<String, Value>,
    resolve: &mut impl FnMut(&str) -> Result<Option<Scalar>, E>,
    output: &mut String,
) -> Result<(), RenderError<E>> {
    for expression in expressions {
        match expression {
            Expression::Literal(text, _) => output.push_str(text),
            Expression::Separator(span) => {
                return Err(Diagnostic::new("Separators not implemented", *span).into());
            },
            Expression::Guard { reference, negative, contents } => {
                let present = reference_value(reference, arguments, resolve)?
                    .is_some_and(|value| value.is_present());
                if present != *negative {
                    evaluate(contents, arguments, resolve, output)?;
                }
            },
            Expression::Interpolation { alternatives, formatters, span } => {
                let mut selected = None;
                for alternative in alternatives {
                    let value = match alternative {
                        Alternative::Literal(text) => Some(Value::Text(text.clone())),
                        Alternative::Reference(reference) => reference_value(reference, arguments, resolve)?,
                    };
                    if value.as_ref().is_some_and(Value::is_present) {
                        selected = value;
                        break;
                    }
                }
                let Some(value) = selected else { continue };
                let mut text = match value {
                    Value::Text(text) => text,
                    Value::Integer(integer) => integer.to_string(),
                    Value::Path(_) => return Err(Diagnostic::new("Path insertion not implemented", *span).into()),
                };
                for formatter in formatters {
                    text = match formatter {
                        Formatter::Year(span) => format::year(&text)
                            .map_err(|message| Diagnostic::new(message, *span))?,
                        Formatter::Pad(width) => format::pad(&text, *width),
                    };
                }
                output.push_str(&text);
            },
        }
    }
    Ok(())
}
