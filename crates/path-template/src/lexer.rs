use crate::{Diagnostic, Span};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Name(String),
    Tag(String),
    Text(String),
    Number(String),
    Colon,
    Question,
    Fallback,
    Bang,
    Pipe,
    Slash,
    OpenParen,
    CloseParen,
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    Comma,
    End,
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub kind: Kind,
    pub span: Span,
}

pub(crate) fn lex(source: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut lexer = Lexer { source, offset: 0 };
    let mut tokens = Vec::new();
    while let Some(character) = lexer.peek() {
        if character.is_whitespace() {
            lexer.advance();
            continue;
        }
        if character == '#' {
            while lexer.peek().is_some_and(|c| c != '\n') {
                lexer.advance();
            }
            continue;
        }
        let start = lexer.offset;
        lexer.advance();
        let kind = match character {
            ':' => Kind::Colon,
            '?' if lexer.peek() == Some('?') => {
                lexer.advance();
                Kind::Fallback
            },
            '?' => Kind::Question,
            '!' => Kind::Bang,
            '|' => Kind::Pipe,
            '/' => Kind::Slash,
            '(' => Kind::OpenParen,
            ')' => Kind::CloseParen,
            '{' => Kind::OpenBrace,
            '}' => Kind::CloseBrace,
            '[' => Kind::OpenBracket,
            ']' => Kind::CloseBracket,
            ',' => Kind::Comma,
            '"' => Kind::Text(lexer.string(start)?),
            '$' => {
                if !lexer.peek().is_some_and(is_name_start) {
                    return Err(lexer.error("Expected a tag name after '$'", start));
                }
                let name_start = lexer.offset;
                lexer.name_tail(true);
                Kind::Tag(source[name_start..lexer.offset].to_ascii_lowercase())
            },
            c if is_name_start(c) => {
                lexer.name_tail(false);
                Kind::Name(source[start..lexer.offset].to_ascii_lowercase())
            },
            c if c.is_ascii_digit() || c == '-' => {
                while lexer.peek().is_some_and(|c| c.is_ascii_digit()) {
                    lexer.advance();
                }
                Kind::Number(source[start..lexer.offset].to_owned())
            },
            _ => return Err(lexer.error("Unexpected character", start)),
        };
        tokens.push(Token { kind, span: Span { start, end: lexer.offset } });
    }
    tokens.push(Token {
        kind: Kind::End,
        span: Span { start: source.len(), end: source.len() },
    });
    Ok(tokens)
}

fn is_name_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

struct Lexer<'a> {
    source: &'a str,
    offset: usize,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.offset += c.len_utf8();
        }
    }

    fn name_tail(&mut self, tag: bool) {
        while self.peek().is_some_and(|c| {
            c.is_ascii_alphanumeric() || c == '_' || (tag && c == '-')
        }) {
            self.advance();
        }
    }

    fn error(&self, message: &str, start: usize) -> Diagnostic {
        Diagnostic::new(message, Span { start, end: self.offset })
    }

    fn string(&mut self, start: usize) -> Result<String, Diagnostic> {
        let mut result = String::new();
        while let Some(c) = self.peek() {
            self.advance();
            match c {
                '"' => return Ok(result),
                '\\' => match self.peek() {
                    Some(escaped @ ('"' | '\\')) => {
                        self.advance();
                        result.push(escaped);
                    },
                    _ => return Err(self.error("Invalid string escape", start)),
                },
                _ => result.push(c),
            }
        }
        Err(self.error("Unterminated quoted string", start))
    }
}
