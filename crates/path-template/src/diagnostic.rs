/// UTF-8 byte offsets into the original template source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// A language error with a location in the original source.
#[derive(Clone, Debug, thiserror::Error)]
#[error("{message} (bytes {}..{})", .span.start, .span.end)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

/// A template failure or the original error returned by a metadata resolver.
#[derive(Debug, thiserror::Error)]
pub enum RenderError<E> {
    #[error(transparent)]
    Template(#[from] Diagnostic),
    #[error("Could not resolve tag '{name}' (bytes {}..{}): {source}", .span.start, .span.end)]
    Resolver { name: String, span: Span, source: E },
}

impl Diagnostic {
    pub(crate) fn new(message: impl Into<String>, span: Span) -> Self {
        Self { message: message.into(), span }
    }

    /// Return a one based line and character column, rather than byte column.
    #[must_use]
    pub fn line_column(&self, source: &str) -> (usize, usize) {
        let mut start = self.span.start.min(source.len());
        while !source.is_char_boundary(start) {
            start -= 1;
        }
        let before = &source[..start];
        let line = before.bytes().filter(|&byte| byte == b'\n').count() + 1;
        let column =
            before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        (line, column)
    }
}
