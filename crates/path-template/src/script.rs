use std::collections::HashMap;
use std::sync::Arc;

use crate::ast::{Alternative, Expression};
use crate::value::Value;
use crate::{ArgKind, ArgSpec, ArgumentPolicy, Diagnostic, RenderError, RenderedPath, Scalar, Span, args, parser, render};

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

/// A script with resolved argument values, ready for metadata rendering.
#[derive(Clone, Debug)]
pub struct BoundScript {
    pub(crate) script: Script,
    pub(crate) arguments: HashMap<String, Value>,
}

impl BoundScript {
    pub fn render<E>(
        &self,
        resolve: impl FnMut(&str) -> Result<Option<Scalar>, E>,
    ) -> Result<RenderedPath, RenderError<E>> {
        render::render(self, resolve)
    }
}

impl Script {
    pub fn bind(&self, supplied: &[String]) -> Result<BoundScript, Diagnostic> {
        if supplied.len() > self.inner.arguments.len() {
            return Err(Diagnostic::new("Too many supplied arguments", self.inner.span));
        }
        let mut arguments = HashMap::new();
        for (index, spec) in self.inner.arguments.iter().enumerate() {
            let raw = supplied.get(index).map(String::as_str).or(spec.default.as_deref());
            let raw = raw.ok_or_else(|| Diagnostic::new(
                format!("Missing required argument '{}'", spec.name), spec.span,
            ))?;
            arguments.insert(spec.name.clone(), args::coerce(spec, raw, &self.inner.policy)?);
        }
        Ok(BoundScript { script: self.clone(), arguments })
    }

    pub fn compile(source: &str, policy: ArgumentPolicy) -> Result<Self, Diagnostic> {
        let compiled = parser::parse(source, policy)?;
        validate_arguments(&compiled.path, &compiled.arguments)?;
        for spec in &compiled.arguments {
            if let Some(default) = &spec.default {
                args::coerce(spec, default, &compiled.policy)?;
            }
        }
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
            Expression::Interpolation { alternatives, formatters, .. } => {
                for alternative in alternatives {
                    if let Alternative::Reference(reference) = alternative {
                        validate(reference)?;
                        if !reference.tag && !formatters.is_empty()
                            && arguments.iter().any(|a| a.name == reference.name && a.kind == ArgKind::Path)
                        {
                            return Err(Diagnostic::new("Cannot format a path argument", reference.span));
                        }
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
