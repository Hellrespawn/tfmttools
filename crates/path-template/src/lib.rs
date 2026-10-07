//! Compile path templates with caller supplied metadata.

mod args;
mod ast;
mod diagnostic;
mod lexer;
mod parser;
mod script;

pub use args::{ArgKind, ArgSpec, ArgumentPolicy};
pub use diagnostic::{Diagnostic, Span};
pub use script::{Metadata, Script, TagReference};
