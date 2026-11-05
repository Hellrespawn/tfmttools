use std::collections::HashMap;

use crate::ast::{Expression, ExpressionKind, Formatter, Reference};
use crate::path::Builder;
use crate::value::Value;
use crate::{
    BoundTemplate, Diagnostic, RenderError, RenderedPath, Scalar, Span, format,
};

pub(crate) fn render<E>(
    bound: &BoundTemplate,
    mut resolve: impl FnMut(&str) -> Result<Option<Scalar>, E>,
) -> Result<RenderedPath, RenderError<E>> {
    let mut builder = Builder::default();
    for expression in &bound.template.inner.path {
        let value = evaluate(expression, &bound.arguments, &mut resolve)?;
        emit(value, expression.span, &mut builder)?;
    }
    Ok(builder.finish(bound.template.inner.span)?)
}

fn reference_value<E>(
    reference: &Reference,
    arguments: &HashMap<String, Value>,
    resolve: &mut impl FnMut(&str) -> Result<Option<Scalar>, E>,
) -> Result<Value, RenderError<E>> {
    if reference.tag {
        resolve(&reference.name)
            .map(|value| value.map_or(Value::Absent, Value::from))
            .map_err(|source| {
                RenderError::Resolver {
                    name: reference.name.clone(),
                    span: reference.span,
                    source,
                }
            })
    } else {
        Ok(arguments.get(&reference.name).cloned().unwrap_or(Value::Absent))
    }
}

fn evaluate<E>(
    expression: &Expression,
    arguments: &HashMap<String, Value>,
    resolve: &mut impl FnMut(&str) -> Result<Option<Scalar>, E>,
) -> Result<Value, RenderError<E>> {
    Ok(match &expression.kind {
        ExpressionKind::Literal(text) => Value::Text(text.clone()),
        ExpressionKind::Integer(value) => Value::Integer(*value),
        ExpressionKind::Reference(reference) => {
            reference_value(reference, arguments, resolve)?
        },
        ExpressionKind::Root => Value::Root,
        ExpressionKind::Separator => Value::Separator,
        ExpressionKind::Sequence(children) => {
            if let [child] = children.as_slice() {
                return evaluate(child, arguments, resolve);
            }
            let values = children
                .iter()
                .map(|child| {
                    evaluate(child, arguments, resolve)
                        .map(|value| (value, child.span))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Value::Sequence(values)
        },
        ExpressionKind::Negate(input) => {
            Value::Boolean(!evaluate(input, arguments, resolve)?.is_present())
        },
        ExpressionKind::Fallback(alternatives) => {
            let mut selected = Value::Absent;
            for alternative in alternatives {
                selected = evaluate(alternative, arguments, resolve)?;
                if selected.is_present() {
                    break;
                }
            }
            selected
        },
        ExpressionKind::Conditional { condition, consequent, alternative } => {
            if evaluate(condition, arguments, resolve)?.is_present() {
                evaluate(consequent, arguments, resolve)?
            } else if let Some(alternative) = alternative {
                evaluate(alternative, arguments, resolve)?
            } else {
                Value::Absent
            }
        },
        ExpressionKind::Format { input, formatters } => {
            let value = evaluate(input, arguments, resolve)?;
            if !value.is_present() {
                return Ok(Value::Absent);
            }
            let mut text = value.into_text(input.span)?;
            for formatter in formatters {
                text = match formatter {
                    Formatter::Year(span) => {
                        format::year(&text).map_err(|message| {
                            Diagnostic::new(message, *span)
                        })?
                    },
                    Formatter::Pad(width) => format::pad(&text, *width),
                };
            }
            Value::Text(text)
        },
    })
}

fn emit(
    value: Value,
    span: Span,
    output: &mut Builder,
) -> Result<(), Diagnostic> {
    match value {
        Value::Absent => {},
        Value::Text(text) => output.append(&text, span)?,
        Value::Integer(value) => output.append(&value.to_string(), span)?,
        Value::Path(components) => {
            if !components.is_empty() {
                output.insert(components, span)?;
            }
        },
        Value::Sequence(values) => {
            for (value, location) in values {
                emit(value, location, output)?;
            }
        },
        Value::Root => output.root(),
        Value::Separator => output.separator(span)?,
        Value::Boolean(_) => {
            return Err(Diagnostic::new("Cannot emit a boolean value", span));
        },
    }
    Ok(())
}
