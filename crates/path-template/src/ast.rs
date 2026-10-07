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
    Pad(usize, Span),
}

#[derive(Clone, Debug)]
pub(crate) enum Expression {
    Literal(String, Span),
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
