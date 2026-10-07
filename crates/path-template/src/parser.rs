use std::mem::discriminant;

use crate::ast::{Alternative, Expression, Formatter, Reference};
use crate::lexer::{Kind, Token, lex};
use crate::script::Compiled;
use crate::{ArgKind, ArgSpec, ArgumentPolicy, Diagnostic, Metadata, Span, TagReference};

pub(crate) fn parse(source: &str, policy: ArgumentPolicy) -> Result<Compiled, Diagnostic> {
    let mut parser = Parser { tokens: lex(source)?, index: 0, references: Vec::new() };
    let mut metadata = Metadata::default();
    let mut arguments = Vec::<ArgSpec>::new();
    let mut path = None;
    let mut path_span = Span::default();
    while !parser.at(&Kind::End) {
        let definition = parser.take();
        let Kind::Name(name) = &definition.kind else {
            return Err(Diagnostic::new(
                "Expected a definition; migrate Jinja/frontmatter scripts manually",
                definition.span,
            ));
        };
        if name == "arg" {
            let argument = parser.argument(definition.span.start)?;
            if arguments.iter().any(|existing| existing.name == argument.name) {
                return Err(Diagnostic::new("Duplicate argument", argument.span));
            }
            arguments.push(argument);
            continue;
        }
        parser.expect(&Kind::Colon, "Expected ':' after definition name")?;
        match name.as_str() {
            "name" | "description" => {
                let slot = if name == "name" { &mut metadata.name } else { &mut metadata.description };
                if slot.is_some() {
                    return Err(Diagnostic::new("Duplicate definition", definition.span));
                }
                *slot = Some(parser.text()?);
            },
            "path" => {
                if path.is_some() {
                    return Err(Diagnostic::new("Duplicate path definition", definition.span));
                }
                parser.expect(&Kind::OpenParen, "Expected '(' after 'path:'")?;
                path = Some(parser.sequence(&Kind::CloseParen)?);
                path_span = Span { start: definition.span.start, end: parser.previous_span().end };
            },
            _ => return Err(Diagnostic::new("Unknown definition", definition.span)),
        }
    }
    let path = path.ok_or_else(|| Diagnostic::new("Missing path definition", parser.current().span))?;
    Ok(Compiled { metadata, arguments, references: parser.references, path, span: path_span, policy })
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    references: Vec<TagReference>,
}

impl Parser {
    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn previous_span(&self) -> Span {
        self.tokens[self.index.saturating_sub(1)].span
    }

    fn at(&self, kind: &Kind) -> bool {
        discriminant(&self.current().kind) == discriminant(kind)
    }

    fn take(&mut self) -> Token {
        let token = self.current().clone();
        if !self.at(&Kind::End) {
            self.index += 1;
        }
        token
    }

    fn consume(&mut self, kind: &Kind) -> bool {
        if self.at(kind) {
            self.take();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &Kind, message: &str) -> Result<Token, Diagnostic> {
        if self.at(kind) {
            Ok(self.take())
        } else {
            Err(Diagnostic::new(message, self.current().span))
        }
    }

    fn name(&mut self) -> Result<String, Diagnostic> {
        let token = self.expect(&Kind::Name(String::new()), "Expected an identifier")?;
        let Kind::Name(name) = token.kind else { unreachable!() };
        Ok(name)
    }

    fn text(&mut self) -> Result<String, Diagnostic> {
        let token = self.expect(&Kind::Text(String::new()), "Expected a quoted string")?;
        let Kind::Text(text) = token.kind else { unreachable!() };
        Ok(text)
    }

    fn argument(&mut self, start: usize) -> Result<ArgSpec, Diagnostic> {
        let name = self.name()?;
        self.expect(&Kind::Colon, "Expected ':' after argument name")?;
        let kind_span = self.current().span;
        let kind = match self.name()?.as_str() {
            "string" => ArgKind::String,
            "int" => ArgKind::Int,
            "path" => ArgKind::Path,
            _ => return Err(Diagnostic::new("Unknown argument type", kind_span)),
        };
        let mut default = None;
        let mut description = None;
        if self.consume(&Kind::OpenParen) {
            while !self.consume(&Kind::CloseParen) {
                let setting_span = self.current().span;
                let setting = self.name()?;
                self.expect(&Kind::Colon, "Expected ':' after option name")?;
                let slot = match setting.as_str() {
                    "default" => &mut default,
                    "description" => &mut description,
                    _ => return Err(Diagnostic::new("Unknown argument option", setting_span)),
                };
                if slot.is_some() {
                    return Err(Diagnostic::new("Duplicate argument option", setting_span));
                }
                *slot = Some(self.text()?);
                if !self.consume(&Kind::Comma) {
                    self.expect(&Kind::CloseParen, "Expected ',' or ')' after option")?;
                    break;
                }
            }
        }
        Ok(ArgSpec { name, kind, default, description, span: Span { start, end: self.previous_span().end } })
    }

    fn reference(&mut self) -> Result<Reference, Diagnostic> {
        let token = self.take();
        let (name, tag) = match token.kind {
            Kind::Name(name) => (name, false),
            Kind::Tag(name) => (name, true),
            _ => return Err(Diagnostic::new("Expected an argument or '$tag' reference", token.span)),
        };
        if tag {
            self.references.push(TagReference { name: name.clone(), span: token.span });
        }
        Ok(Reference { name, tag, span: token.span })
    }

    fn literal(&mut self) -> Result<String, Diagnostic> {
        let span = self.current().span;
        let text = self.text()?;
        if text.contains(['/', '\\']) {
            return Err(Diagnostic::new("Use a bare '/' for path separators", span));
        }
        Ok(text)
    }

    fn sequence(&mut self, closing: &Kind) -> Result<Vec<Expression>, Diagnostic> {
        let mut expressions = Vec::new();
        while !self.consume(closing) {
            let span = self.current().span;
            let expression = match &self.current().kind {
                Kind::Text(_) => Expression::Literal(self.literal()?, span),
                Kind::Slash => {
                    self.take();
                    Expression::Separator(span)
                },
                Kind::OpenBrace => {
                    self.take();
                    self.interpolation(span.start)?
                },
                Kind::OpenBracket => {
                    self.take();
                    let negative = self.consume(&Kind::Bang);
                    let reference = self.reference()?;
                    self.expect(&Kind::Question, "Expected '?' after guard reference")?;
                    let contents = self.sequence(&Kind::CloseBracket)?;
                    Expression::Guard { reference, negative, contents }
                },
                _ => return Err(Diagnostic::new("Expected a quoted literal, interpolation, guard, or '/'", span)),
            };
            expressions.push(expression);
        }
        Ok(expressions)
    }

    fn interpolation(&mut self, start: usize) -> Result<Expression, Diagnostic> {
        let mut alternatives = Vec::new();
        loop {
            let alternative = if self.at(&Kind::Text(String::new())) {
                Alternative::Literal(self.literal()?)
            } else {
                Alternative::Reference(self.reference()?)
            };
            alternatives.push(alternative);
            if !self.consume(&Kind::Fallback) {
                break;
            }
        }
        let mut formatters = Vec::new();
        while self.consume(&Kind::Pipe) {
            let span = self.current().span;
            let formatter = match self.name()?.as_str() {
                "year" => Formatter::Year(span),
                "pad" => {
                    self.expect(&Kind::OpenParen, "Expected '(' after pad")?;
                    let width_token = self.expect(&Kind::Number(String::new()), "Expected a nonnegative padding width")?;
                    let Kind::Number(width) = width_token.kind else { unreachable!() };
                    let width = width.parse::<usize>().map_err(|_| Diagnostic::new("Invalid padding width", width_token.span))?;
                    let close = self.expect(&Kind::CloseParen, "Expected ')' after padding width")?;
                    Formatter::Pad(width, Span { start: span.start, end: close.span.end })
                },
                _ => return Err(Diagnostic::new("Unknown formatter", span)),
            };
            formatters.push(formatter);
        }
        let close = self.expect(&Kind::CloseBrace, "Expected '}' after interpolation")?;
        Ok(Expression::Interpolation { alternatives, formatters, span: Span { start, end: close.span.end } })
    }
}
