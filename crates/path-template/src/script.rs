use std::sync::Arc;

use crate::ast::{Alternative, Expression};
use crate::{ArgSpec, ArgumentPolicy, Diagnostic, Span, parser};

/// Optional listing metadata, independent of argument binding.
#[derive(Clone, Debug, Default)]
pub struct Metadata {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// A metadata reference the caller can check against its schema.
#[derive(Clone, Debug)]
pub struct TagReference {
    pub name: String,
    pub span: Span,
}

#[derive(Debug)]
pub(crate) struct Compiled {
    pub metadata: Metadata,
    pub arguments: Vec<ArgSpec>,
    pub references: Vec<TagReference>,
    pub path: Vec<Expression>,
    pub span: Span,
    pub policy: ArgumentPolicy,
}

/// An owned, compiled script. Cloning shares its immutable representation.
#[derive(Clone, Debug)]
pub struct Script {
    pub(crate) inner: Arc<Compiled>,
}

impl Script {
    pub fn compile(source: &str, policy: ArgumentPolicy) -> Result<Self, Diagnostic> {
        let compiled = parser::parse(source, policy)?;
        validate_arguments(&compiled.path, &compiled.arguments)?;
        Ok(Self { inner: Arc::new(compiled) })
    }

    #[must_use]
    pub fn metadata(&self) -> &Metadata {
        &self.inner.metadata
    }

    #[must_use]
    pub fn arguments(&self) -> &[ArgSpec] {
        &self.inner.arguments
    }

    #[must_use]
    pub fn tag_references(&self) -> &[TagReference] {
        &self.inner.references
    }
}

fn validate_arguments(path: &[Expression], arguments: &[ArgSpec]) -> Result<(), Diagnostic> {
    let validate = |reference: &crate::ast::Reference| {
        if !reference.tag && !arguments.iter().any(|a| a.name == reference.name) {
            Err(Diagnostic::new(
                format!("Undeclared argument '{}'", reference.name),
                reference.span,
            ))
        } else {
            Ok(())
        }
    };
    for expression in path {
        match expression {
            Expression::Interpolation { alternatives, .. } => {
                for alternative in alternatives {
                    if let Alternative::Reference(reference) = alternative {
                        validate(reference)?;
                    }
                }
            },
            Expression::Guard { reference, contents, .. } => {
                validate(reference)?;
                validate_arguments(contents, arguments)?;
            },
            Expression::Literal(..) | Expression::Separator(_) => {},
        }
    }
    Ok(())
}
