use crate::ast::{Expression, ExpressionKind};
use crate::{ArgKind, ArgSpec, Diagnostic};

#[derive(Clone, Copy, Default)]
struct Capabilities {
    boolean: bool,
    integer: bool,
    text: bool,
    structural: bool,
}

impl Capabilities {
    fn include(&mut self, other: Self) {
        self.boolean |= other.boolean;
        self.integer |= other.integer;
        self.text |= other.text;
        self.structural |= other.structural;
    }
}

pub(crate) fn validate(
    path: &[Expression],
    arguments: &[ArgSpec],
) -> Result<(), Diagnostic> {
    for expression in path {
        if analyze(expression, arguments)?.boolean {
            return Err(Diagnostic::new(
                "Cannot emit a boolean value",
                expression.span,
            ));
        }
    }
    Ok(())
}

fn analyze(
    expression: &Expression,
    arguments: &[ArgSpec],
) -> Result<Capabilities, Diagnostic> {
    let mut result = Capabilities::default();
    match &expression.kind {
        ExpressionKind::Reference(reference) => {
            if !reference.tag {
                let argument = arguments
                    .iter()
                    .find(|a| a.name == reference.name)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            format!("Undeclared argument '{}'", reference.name),
                            reference.span,
                        )
                    })?;
                result.structural = argument.kind == ArgKind::Path;
                result.integer = argument.kind == ArgKind::Int;
                result.text = argument.kind == ArgKind::String;
            } else {
                result.integer = true;
                result.text = true;
            }
        },
        ExpressionKind::Sequence(children)
        | ExpressionKind::Fallback(children) => {
            for child in children {
                let capabilities = analyze(child, arguments)?;
                if matches!(expression.kind, ExpressionKind::Sequence(_))
                    && children.len() > 1
                    && capabilities.boolean
                {
                    return Err(Diagnostic::new(
                        "Cannot concatenate a boolean value",
                        child.span,
                    ));
                }
                result.include(capabilities);
            }
            if matches!(expression.kind, ExpressionKind::Sequence(_))
                && children.len() > 1
            {
                result.text |= result.integer;
                result.integer = false;
            }
        },
        ExpressionKind::Compare { left, operator, right, operator_span } => {
            for operand in [left, right] {
                let capabilities = analyze(operand, arguments)?;
                if capabilities.structural {
                    return Err(Diagnostic::new(
                        "Cannot compare a path value",
                        operand.span,
                    ));
                }
                if operator.is_ordering()
                    && !capabilities.integer
                    && (capabilities.text || capabilities.boolean)
                {
                    return Err(Diagnostic::new(
                        "Ordering requires two integers",
                        *operator_span,
                    ));
                }
            }
            result.boolean = true;
        },
        ExpressionKind::Negate(input) => {
            analyze(input, arguments)?;
            result.boolean = true;
        },
        ExpressionKind::Format { input, .. } => {
            let capabilities = analyze(input, arguments)?;
            if capabilities.boolean {
                return Err(Diagnostic::new(
                    "Cannot format a boolean value",
                    input.span,
                ));
            }
            if capabilities.structural {
                return Err(Diagnostic::new(
                    "Cannot format a path value",
                    input.span,
                ));
            }
            result.text = true;
        },
        ExpressionKind::Conditional { condition, consequent, alternative } => {
            analyze(condition, arguments)?;
            result.include(analyze(consequent, arguments)?);
            if let Some(alternative) = alternative {
                result.include(analyze(alternative, arguments)?);
            }
        },
        ExpressionKind::Root | ExpressionKind::Separator => {
            result.structural = true;
        },
        ExpressionKind::Literal(text) => result.text = !text.is_empty(),
        ExpressionKind::Integer(_) => result.integer = true,
    }
    Ok(result)
}
