use pest::Parser;
use pest::error::InputLocation;
use pest::iterators::Pair;
use pest_derive::Parser;

use crate::ast::{Alternative, Expression, Formatter, Reference};
use crate::template::Compiled;
use crate::{
    ArgKind, ArgSpec, ArgumentPolicy, Diagnostic, Metadata, Span, TagReference,
};

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

pub(crate) fn parse(
    source: &str,
    policy: ArgumentPolicy,
) -> Result<Compiled, Diagnostic> {
    let document = Syntax::parse(Rule::document, source)
        .map_err(|error| {
            let start = match error.location {
                InputLocation::Pos(p) | InputLocation::Span((p, _)) => p,
            };
            let end = source[start..]
                .chars()
                .next()
                .map_or(start, |c| start + c.len_utf8());
            let message = error
                .renamed_rules(|rule| {
                    match rule {
                        Rule::name => "an identifier",
                        Rule::tag => "a '$tag' reference",
                        Rule::quoted => "a quoted string",
                        Rule::separator => "'/'",
                        Rule::interpolation => "an interpolation '{...}'",
                        Rule::guard => "a guard '[reference? ...]'",
                        Rule::sequence => "a path sequence '(...)'",
                        Rule::argument => "an argument declaration",
                        Rule::definition => "a definition 'name: value'",
                        Rule::formatter => {
                            "a formatter '| year' or '| pad(width)'"
                        },
                        Rule::width | Rule::number => {
                            "a nonnegative padding width"
                        },
                        Rule::EOI => "end of template",
                        _ => "valid template syntax",
                    }
                    .to_owned()
                })
                .variant
                .message()
                .into_owned();
            let message = if source.trim_start().starts_with(['{', '+']) {
                format!(
                    "{message}; migrate Jinja/frontmatter templates manually"
                )
            } else {
                message
            };
            Diagnostic::new(message, Span { start, end })
        })?
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
                    "path" => {
                        if path.is_some() {
                            return Err(Diagnostic::new(
                                "Duplicate path definition",
                                span(&name),
                            ));
                        }
                        if value.as_rule() != Rule::sequence {
                            return Err(Diagnostic::new(
                                "Expected '(' after 'path:'",
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
        Diagnostic::new("Missing path definition", Span {
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
        policy,
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
    fn reference(&mut self, node: Node<'_>) -> Reference {
        let tag = node.as_rule() == Rule::tag;
        let name = if tag {
            node.as_str()[1..].to_owned()
        } else {
            node.as_str().to_ascii_lowercase()
        };
        let location = span(&node);
        if tag {
            self.references
                .push(TagReference { name: name.clone(), span: location });
        }
        Reference { name, tag, span: location }
    }

    fn literal(node: &Node<'_>) -> Result<String, Diagnostic> {
        let value = text(node);
        if value.contains(['/', '\\']) {
            return Err(Diagnostic::new(
                "Use a bare '/' for path separators",
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
            let location = span(&node);
            result.push(match node.as_rule() {
                Rule::quoted => {
                    Expression::Literal(Self::literal(&node)?, location)
                },
                Rule::separator if root && result.is_empty() => {
                    Expression::Root
                },
                Rule::separator => Expression::Separator(location),
                Rule::interpolation => self.interpolation(node)?,
                Rule::guard => {
                    let mut parts = node.into_inner().peekable();
                    let negative =
                        parts.peek().unwrap().as_rule() == Rule::negative;
                    if negative {
                        parts.next();
                    }
                    let reference = self.reference(parts.next().unwrap());
                    Expression::Guard {
                        reference,
                        negative,
                        contents: self.sequence(parts, false)?,
                    }
                },
                _ => unreachable!(),
            });
        }
        Ok(result)
    }

    fn interpolation(
        &mut self,
        node: Node<'_>,
    ) -> Result<Expression, Diagnostic> {
        let location = span(&node);
        let mut alternatives = Vec::new();
        let mut formatters = Vec::new();
        for node in node.into_inner() {
            match node.as_rule() {
                Rule::quoted => {
                    alternatives
                        .push(Alternative::Literal(Self::literal(&node)?))
                },
                Rule::tag | Rule::name => {
                    alternatives
                        .push(Alternative::Reference(self.reference(node)))
                },
                Rule::formatter => formatters.push(formatter(node)?),
                _ => unreachable!(),
            }
        }
        Ok(Expression::Interpolation {
            alternatives,
            formatters,
            span: location,
        })
    }
}

fn formatter(node: Node<'_>) -> Result<Formatter, Diagnostic> {
    let mut parts = node.into_inner();
    let name = parts.next().unwrap();
    let width = parts.next();
    match (name.as_str().to_ascii_lowercase().as_str(), width) {
        ("year", None) => Ok(Formatter::Year(span(&name))),
        ("pad", Some(width)) => {
            let number = width.into_inner().next().unwrap();
            let width = number.as_str().parse::<usize>().map_err(|_| {
                Diagnostic::new("Invalid padding width", span(&number))
            })?;
            if width > 1024 {
                return Err(Diagnostic::new(
                    "Padding width exceeds maximum of 1024",
                    span(&number),
                ));
            }
            Ok(Formatter::Pad(width))
        },
        ("year" | "pad", _) => {
            Err(Diagnostic::new(
                "Expected 'year' or 'pad(nonnegative width)'",
                span(&name),
            ))
        },
        _ => Err(Diagnostic::new("Unknown formatter", span(&name))),
    }
}
