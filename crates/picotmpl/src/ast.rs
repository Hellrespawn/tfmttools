use crate::Span;

#[derive(Clone, Debug)]
pub(crate) struct Reference {
    pub name: String,
    pub tag: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub(crate) enum Formatter {
    Year(Span),
    Pad(usize),
}

#[derive(Clone, Debug)]
pub(crate) struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub(crate) enum ExpressionKind {
    Literal(String),
    Reference(Reference),
    Sequence(Vec<Expression>),
    Negate(Box<Expression>),
    Format {
        input: Box<Expression>,
        formatters: Vec<Formatter>,
    },
    Fallback(Vec<Expression>),
    Conditional {
        condition: Box<Expression>,
        consequent: Box<Expression>,
        alternative: Option<Box<Expression>>,
    },
    Root,
    Separator,
}

pub(crate) fn dump(path: &[Expression]) -> String {
    let mut lines = vec!["Path".to_owned()];
    for expression in path {
        append_expression(expression, 1, &mut lines);
    }
    lines.join("\n")
}

fn append_expression(
    expression: &Expression,
    depth: usize,
    lines: &mut Vec<String>,
) {
    let label = match &expression.kind {
        ExpressionKind::Literal(text) => format!("Literal {text:?}"),
        ExpressionKind::Reference(reference) => {
            let prefix = if reference.tag { "$" } else { "" };
            format!("Reference {prefix}{}", reference.name)
        },
        ExpressionKind::Sequence(_) => "Sequence".to_owned(),
        ExpressionKind::Negate(_) => "Negate".to_owned(),
        ExpressionKind::Format { formatters, .. } => {
            let names = formatters
                .iter()
                .map(|formatter| {
                    match formatter {
                        Formatter::Year(_) => "year".to_owned(),
                        Formatter::Pad(width) => format!("pad({width})"),
                    }
                })
                .collect::<Vec<_>>()
                .join(" | ");
            format!("Format {names}")
        },
        ExpressionKind::Fallback(_) => "Fallback".to_owned(),
        ExpressionKind::Conditional { .. } => "Conditional".to_owned(),
        ExpressionKind::Root => "Root".to_owned(),
        ExpressionKind::Separator => "Separator".to_owned(),
    };
    lines.push(format!("{}{label}", "  ".repeat(depth)));
    match &expression.kind {
        ExpressionKind::Sequence(children)
        | ExpressionKind::Fallback(children) => {
            for child in children {
                append_expression(child, depth + 1, lines);
            }
        },
        ExpressionKind::Negate(input)
        | ExpressionKind::Format { input, .. } => {
            append_expression(input, depth + 1, lines);
        },
        ExpressionKind::Conditional { condition, consequent, alternative } => {
            for (label, branch) in [
                ("Condition", Some(condition)),
                ("Then", Some(consequent)),
                ("Else", alternative.as_ref()),
            ] {
                if let Some(branch) = branch {
                    lines.push(format!("{}{label}", "  ".repeat(depth + 1)));
                    append_expression(branch, depth + 2, lines);
                }
            }
        },
        _ => {},
    }
}
