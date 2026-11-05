use crate::Span;

#[derive(Clone, Debug)]
pub(crate) struct Reference {
    pub name: String,
    pub tag: bool,
    pub span: Span,
}

#[derive(Clone, Debug)]
pub(crate) enum Alternative {
    Reference(Reference),
    Literal(String),
}

#[derive(Clone, Debug)]
pub(crate) enum Formatter {
    Year(Span),
    Pad(usize),
}

#[derive(Clone, Debug)]
pub(crate) enum Expression {
    Literal(String, Span),
    Root,
    Separator(Span),
    Interpolation {
        alternatives: Vec<Alternative>,
        formatters: Vec<Formatter>,
        span: Span,
    },
    Guard {
        reference: Reference,
        negative: bool,
        contents: Vec<Self>,
    },
}

pub(crate) fn dump(path: &[Expression]) -> String {
    let mut lines = vec!["Path".to_owned()];
    append_expressions(path, 1, &mut lines);
    lines.join("\n")
}

fn reference_label(reference: &Reference) -> String {
    let prefix = if reference.tag { "$" } else { "" };
    format!("{prefix}{}", reference.name)
}

fn append_expressions(
    expressions: &[Expression],
    depth: usize,
    lines: &mut Vec<String>,
) {
    for expression in expressions {
        let label = match expression {
            Expression::Literal(text, _) => format!("Literal {text:?}"),
            Expression::Root => "Root".to_owned(),
            Expression::Separator(_) => "Separator".to_owned(),
            Expression::Interpolation { alternatives, formatters, .. } => {
                let alternatives = alternatives
                    .iter()
                    .map(|alternative| {
                        match alternative {
                            Alternative::Reference(reference) => {
                                reference_label(reference)
                            },
                            Alternative::Literal(text) => format!("{text:?}"),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ?? ");
                let mut parts = vec![format!("Interpolation {alternatives}")];
                for formatter in formatters {
                    parts.push(match formatter {
                        Formatter::Year(_) => "year".to_owned(),
                        Formatter::Pad(width) => format!("pad({width})"),
                    });
                }
                parts.join(" | ")
            },
            Expression::Guard { reference, negative, .. } => {
                let prefix = if *negative { "!" } else { "" };
                format!("Guard {prefix}{}", reference_label(reference))
            },
        };
        lines.push(format!("{}{label}", "  ".repeat(depth)));
        if let Expression::Guard { contents, .. } = expression {
            append_expressions(contents, depth + 1, lines);
        }
    }
}
