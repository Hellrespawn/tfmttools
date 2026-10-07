use crate::Span;

/// Characters the caller rejects in argument component text.
#[derive(Clone, Debug)]
pub struct ArgumentPolicy {
    pub(crate) forbidden: Vec<char>,
}

impl ArgumentPolicy {
    #[must_use]
    pub fn new(forbidden: &[char]) -> Self {
        Self { forbidden: forbidden.to_vec() }
    }
}

/// The declared argument's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArgKind {
    String,
    Int,
    Path,
}

/// An argument declaration. No default means the argument is required.
#[derive(Clone, Debug)]
pub struct ArgSpec {
    pub name: String,
    pub kind: ArgKind,
    pub default: Option<String>,
    pub description: Option<String>,
    pub span: Span,
}
