#![doc = include_str!("../README.md")]

mod args;
mod ast;
mod diagnostic;
mod format;
mod lexer;
mod parser;
mod path;
mod render;
mod template;
mod value;

pub use args::{ArgKind, ArgSpec, ArgumentPolicy};
pub use diagnostic::{Diagnostic, RenderError, Span};
pub use path::RenderedPath;
pub use template::{BoundTemplate, Metadata, TagReference, Template};
pub use value::Scalar;
