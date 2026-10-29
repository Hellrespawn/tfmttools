#![doc = include_str!("../README.md")]

mod args;
mod ast;
mod diagnostic;
mod format;
mod parser;
mod path;
mod render;
mod template;
mod value;

pub use args::{ArgKind, ArgSpec, prohibited_character};
pub use diagnostic::{Diagnostic, RenderError, Span};
pub use path::RenderedPath;
pub use template::{BoundTemplate, Metadata, TagReference, Template};
pub use value::Scalar;
