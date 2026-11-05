use pest::Parser;
use pest::error::InputLocation;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::{
    ComparisonOperator, Expression, ExpressionKind, Formatter, LogicalOperator,
    Reference,
};
use crate::template::Compiled;
use crate::{ArgKind, ArgSpec, Diagnostic, Metadata, Span, TagReference};

#[derive(Parser)]
#[grammar = "grammar.pest"]
struct Syntax;

type Node<'a> = Pair<'a, Rule>;

fn span(node: &Node<'_>) -> Span {
    Span { start: node.as_span().start(), end: node.as_span().end() }
}

fn text(node: &Node<'_>) -> String {
    let raw = node.as_str();
    let mut chars = raw[1..raw.len() - 1].chars();
    let mut result = String::new();
    while let Some(character) = chars.next() {
        result.push(if character == '\\' {
            chars.next().unwrap()
        } else {
            character
        });
    }
    result
}

fn syntax_error(source: &str, error: pest::error::Error<Rule>) -> Diagnostic {
    let start = match error.location {
        InputLocation::Pos(p) | InputLocation::Span((p, _)) => p,
    };
    let end =
        source[start..].chars().next().map_or(start, |c| start + c.len_utf8());
    let message = error
        .renamed_rules(|rule| {
            match rule {
                Rule::name => "an identifier",
                Rule::integer => "a signed decimal integer",
                Rule::tag => "a '$tag' reference",
                Rule::quoted => "a quoted string",
                Rule::separator => "'/'",
                Rule::conditional => "an expression",
                Rule::unary => "a value or unary negation",
                Rule::sequence => "a template expression '(...)'",
                Rule::argument => "an argument declaration",
                Rule::definition => "a definition 'name: value'",
                Rule::formatter => "a formatter after '|'",
                Rule::number => "an integer formatter parameter",
                Rule::EOI => "end of template",
                _ => "valid template syntax",
            }
            .to_owned()
        })
        .variant
        .message()
        .into_owned();
    let message = if source.trim_start().starts_with(['{', '+']) {
        format!("{message}; migrate Jinja/frontmatter templates manually")
    } else {
        let hint = match source[start..].chars().next() {
            Some('{') => "use bare references or parentheses instead of braces",
            Some('[') => {
                "use condition ? (contents) instead of square brackets"
            },
            Some('?')
                if source[start..].starts_with("??")
                    || source[..start].ends_with('?') =>
            {
                "use ?: instead of ??"
            },
            _ => "",
        };
        if hint.is_empty() { message } else { format!("{message}; {hint}") }
    };
    Diagnostic::new(message, Span { start, end })
}

pub(crate) fn parse(source: &str) -> Result<Compiled, Diagnostic> {
    let document = Syntax::parse(Rule::document, source)
        .map_err(|error| syntax_error(source, error))?
        .next()
        .unwrap();
    let mut metadata = Metadata::default();
    let mut arguments: Vec<ArgSpec> = Vec::new();
    let mut path = None;
    let mut path_span = Span::default();
    let mut builder = Builder { references: Vec::new() };
    for node in document.into_inner() {
        let location = span(&node);
        match node.as_rule() {
            Rule::argument => {
                let argument = argument(node)?;
                if arguments.iter().any(|a| a.name == argument.name) {
                    return Err(Diagnostic::new(
                        "Duplicate argument",
                        argument.span,
                    ));
                }
                arguments.push(argument);
            },
            Rule::definition => {
                let mut parts = node.into_inner();
                let name = parts.next().unwrap();
                let value = parts.next().unwrap();
                match name.as_str().to_ascii_lowercase().as_str() {
                    "template" => {
                        if path.is_some() {
                            return Err(Diagnostic::new(
                                "Duplicate template definition",
                                span(&name),
                            ));
                        }
                        if value.as_rule() != Rule::sequence {
                            return Err(Diagnostic::new(
                                "Expected '(' after 'template:'",
                                span(&value),
                            ));
                        }
                        path_span = location;
                        path =
                            Some(builder.sequence(value.into_inner(), true)?);
                    },
                    "name" | "description" => {
                        let slot = if name.as_str().eq_ignore_ascii_case("name")
                        {
                            &mut metadata.name
                        } else {
                            &mut metadata.description
                        };
                        if slot.is_some() {
                            return Err(Diagnostic::new(
                                "Duplicate definition",
                                span(&name),
                            ));
                        }
                        if value.as_rule() != Rule::quoted {
                            return Err(Diagnostic::new(
                                "Expected a quoted string",
                                span(&value),
                            ));
                        }
                        *slot = Some(text(&value));
                    },
                    _ => {
                        return Err(Diagnostic::new(
                            "Unknown definition",
                            span(&name),
                        ));
                    },
                }
            },
            Rule::EOI => {},
            _ => unreachable!(),
        }
    }
    let path = path.ok_or_else(|| {
        Diagnostic::new("Missing template definition", Span {
            start: source.len(),
            end: source.len(),
        })
    })?;
    Ok(Compiled {
        source: source.to_owned(),
        metadata,
        arguments,
        references: builder.references,
        path,
        span: path_span,
    })
}

fn argument(node: Node<'_>) -> Result<ArgSpec, Diagnostic> {
    let mut location = span(&node);
    let mut parts = node.into_inner();
    parts.next(); // arg keyword
    let name = parts.next().unwrap().as_str().to_ascii_lowercase();
    let kind = parts.next().unwrap();
    location.end = kind.as_span().end();
    let kind = match kind.as_str().to_ascii_lowercase().as_str() {
        "string" => ArgKind::String,
        "int" => ArgKind::Int,
        "path" => ArgKind::Path,
        _ => return Err(Diagnostic::new("Unknown argument type", span(&kind))),
    };
    let mut default = None;
    let mut description = None;
    if let Some(settings) = parts.next() {
        location.end = settings.as_span().end();
        for setting in settings.into_inner() {
            let mut parts = setting.into_inner();
            let name = parts.next().unwrap();
            let slot = match name.as_str().to_ascii_lowercase().as_str() {
                "default" => &mut default,
                "description" => &mut description,
                _ => {
                    return Err(Diagnostic::new(
                        "Unknown argument option",
                        span(&name),
                    ));
                },
            };
            if slot.is_some() {
                return Err(Diagnostic::new(
                    "Duplicate argument option",
                    span(&name),
                ));
            }
            *slot = Some(text(&parts.next().unwrap()));
        }
    }
    Ok(ArgSpec { name, kind, default, description, span: location })
}

struct Builder {
    references: Vec<TagReference>,
}

impl Builder {
    fn reference(&mut self, node: &Node<'_>) -> Reference {
        let tag = node.as_rule() == Rule::tag;
        let name = if tag {
            node.as_str()[1..].to_owned()
        } else {
            node.as_str().to_ascii_lowercase()
        };
        let location = span(node);
        if tag {
            self.references
                .push(TagReference { name: name.clone(), span: location });
        }
        Reference { name, tag, span: location }
    }

    fn literal(node: &Node<'_>) -> Result<String, Diagnostic> {
        let value = text(node);
        if let Some(character) = crate::prohibited_character(&value) {
            return Err(Diagnostic::new(
                format!("Literal contains prohibited character {character:?}"),
                span(node),
            ));
        }
        Ok(value)
    }

    fn sequence<'a>(
        &mut self,
        nodes: impl Iterator<Item = Node<'a>>,
        root: bool,
    ) -> Result<Vec<Expression>, Diagnostic> {
        let mut result = Vec::new();
        for node in nodes {
            let mut expression = self.expression(node)?;
            if root
                && result.is_empty()
                && matches!(expression.kind, ExpressionKind::Separator)
            {
                expression.kind = ExpressionKind::Root;
            }
            result.push(expression);
        }
        Ok(result)
    }

    fn comparison(&mut self, node: Node<'_>) -> Result<Expression, Diagnostic> {
        let location = span(&node);
        let mut parts = node.into_inner();
        let left = self.expression(parts.next().unwrap())?;
        let Some(operator) = parts.next() else {
            return Ok(left);
        };
        let operator_span = span(&operator);
        let operator = match operator.as_str() {
            "==" => ComparisonOperator::Equal,
            "!=" => ComparisonOperator::NotEqual,
            "<" => ComparisonOperator::Less,
            "<=" => ComparisonOperator::LessEqual,
            ">" => ComparisonOperator::Greater,
            ">=" => ComparisonOperator::GreaterEqual,
            _ => unreachable!(),
        };
        Ok(Expression {
            kind: ExpressionKind::Compare {
                left: Box::new(left),
                operator,
                right: Box::new(self.expression(parts.next().unwrap())?),
                operator_span,
            },
            span: location,
        })
    }

    fn expression(&mut self, node: Node<'_>) -> Result<Expression, Diagnostic> {
        let location = span(&node);
        let kind = match node.as_rule() {
            Rule::quoted => ExpressionKind::Literal(Self::literal(&node)?),
            Rule::integer => {
                ExpressionKind::Integer(node.as_str().parse::<i64>().map_err(
                    |_| {
                        Diagnostic::new(
                            "Integer literal exceeds signed 64-bit range",
                            location,
                        )
                    },
                )?)
            },
            Rule::tag | Rule::name => {
                ExpressionKind::Reference(self.reference(&node))
            },
            Rule::separator => ExpressionKind::Separator,
            Rule::sequence => {
                ExpressionKind::Sequence(
                    self.sequence(node.into_inner(), false)?,
                )
            },
            Rule::conditional => {
                let mut parts = node.into_inner();
                let condition = self.expression(parts.next().unwrap())?;
                if let Some(consequent) = parts.next() {
                    ExpressionKind::Conditional {
                        condition: Box::new(condition),
                        consequent: Box::new(self.expression(consequent)?),
                        alternative: parts
                            .next()
                            .map(|part| self.expression(part).map(Box::new))
                            .transpose()?,
                    }
                } else {
                    return Ok(condition);
                }
            },
            Rule::logical_or | Rule::logical_and => {
                let operator = if node.as_rule() == Rule::logical_and {
                    LogicalOperator::And
                } else {
                    LogicalOperator::Or
                };
                let mut parts = node.into_inner();
                let mut left = self.expression(parts.next().unwrap())?;
                while parts.next().is_some() {
                    let right = self.expression(parts.next().unwrap())?;
                    let span =
                        Span { start: left.span.start, end: right.span.end };
                    left = Expression {
                        kind: ExpressionKind::Logical {
                            left: Box::new(left),
                            operator,
                            right: Box::new(right),
                        },
                        span,
                    };
                }
                return Ok(left);
            },
            Rule::comparison => return self.comparison(node),
            Rule::fallback => {
                let mut alternatives = node
                    .into_inner()
                    .map(|part| self.expression(part))
                    .collect::<Result<Vec<_>, _>>()?;
                if alternatives.len() == 1 {
                    return Ok(alternatives.pop().unwrap());
                }
                ExpressionKind::Fallback(alternatives)
            },
            Rule::formatted => {
                let mut parts = node.into_inner();
                let input = self.expression(parts.next().unwrap())?;
                let formatters =
                    parts.map(formatter).collect::<Result<Vec<_>, _>>()?;
                if formatters.is_empty() {
                    return Ok(input);
                }
                ExpressionKind::Format { input: Box::new(input), formatters }
            },
            Rule::unary => {
                let mut parts = node.into_inner();
                let first = parts.next().unwrap();
                if first.as_rule() == Rule::negation {
                    ExpressionKind::Negate(Box::new(
                        self.expression(parts.next().unwrap())?,
                    ))
                } else {
                    return self.expression(first);
                }
            },
            _ => unreachable!(),
        };
        Ok(Expression { kind, span: location })
    }
}

fn formatter(node: Node<'_>) -> Result<Formatter, Diagnostic> {
    let mut parts = node.into_inner();
    let name = parts.next().unwrap();
    let parameters = parts.next();
    let arguments: Vec<_> = parameters
        .as_ref()
        .map(|parameters| parameters.clone().into_inner().collect())
        .unwrap_or_default();
    match (
        name.as_str().to_ascii_lowercase().as_str(),
        parameters.is_some(),
        arguments.as_slice(),
    ) {
        ("year", false, []) => Ok(Formatter::Year(span(&name))),
        ("lower", false, []) => Ok(Formatter::Lower),
        ("upper", false, []) => Ok(Formatter::Upper),
        ("trim", false, []) => Ok(Formatter::Trim),
        ("pad", true, [number]) if number.as_rule() == Rule::number => {
            let width = number.as_str().parse::<usize>().map_err(|_| {
                Diagnostic::new("Invalid padding width", span(number))
            })?;
            if width > 1024 {
                return Err(Diagnostic::new(
                    "Padding width exceeds maximum of 1024",
                    span(number),
                ));
            }
            Ok(Formatter::Pad(width))
        },
        ("replace", true, [from, to])
            if from.as_rule() == Rule::quoted
                && to.as_rule() == Rule::quoted =>
        {
            let value = Builder::literal(from)?;
            if value.is_empty() {
                return Err(Diagnostic::new(
                    "Replacement search string must not be empty",
                    span(from),
                ));
            }
            Ok(Formatter::Replace { from: value, to: Builder::literal(to)? })
        },
        ("year" | "lower" | "upper" | "trim", _, _) => {
            Err(Diagnostic::new(
                "This formatter takes no parameters or parentheses",
                span(&name),
            ))
        },
        ("pad", _, _) => {
            Err(Diagnostic::new(
                "Expected 'pad(nonnegative width)'",
                span(&name),
            ))
        },
        ("replace", _, _) => {
            Err(Diagnostic::new(
                "Expected 'replace(\"from\", \"to\")'",
                span(&name),
            ))
        },
        _ => Err(Diagnostic::new("Unknown formatter", span(&name))),
    }
}
