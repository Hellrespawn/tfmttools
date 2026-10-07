use std::path::PathBuf;

use crate::{Diagnostic, Span};

/// A rendered path with structural directory boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedPath {
    pub(crate) components: Vec<String>,
    pub(crate) rooted: bool,
}

impl RenderedPath {
    #[must_use]
    pub fn is_rooted(&self) -> bool {
        self.rooted
    }

    #[must_use]
    pub fn to_path_buf(&self) -> PathBuf {
        let mut path = if self.rooted {
            PathBuf::from(std::path::MAIN_SEPARATOR_STR)
        } else {
            PathBuf::new()
        };
        for component in &self.components {
            path.push(component);
        }
        path
    }

    #[must_use]
    pub fn components(&self) -> &[String] {
        &self.components
    }
}

#[derive(Default)]
pub(crate) struct Builder {
    current: String,
    components: Vec<String>,
    rooted: bool,
}

impl Builder {
    pub fn append(&mut self, text: &str, span: Span) -> Result<(), Diagnostic> {
        if text.contains(['/', '\\']) {
            return Err(Diagnostic::new(
                "Component text contains a path separator",
                span,
            ));
        }
        self.current.push_str(text);
        Ok(())
    }

    pub fn root(&mut self) {
        self.rooted = true;
    }

    pub fn separator(&mut self, span: Span) -> Result<(), Diagnostic> {
        if self.current.is_empty() {
            return Err(Diagnostic::new(
                "Separator creates an empty component",
                span,
            ));
        }
        self.components.push(std::mem::take(&mut self.current));
        Ok(())
    }

    pub fn insert(
        &mut self,
        components: Vec<String>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if !self.current.is_empty() {
            return Err(Diagnostic::new(
                "Path arguments require a component boundary",
                span,
            ));
        }
        self.components.extend(components);
        Ok(())
    }

    pub fn finish(mut self, span: Span) -> Result<RenderedPath, Diagnostic> {
        if self.current.is_empty() {
            return Err(Diagnostic::new(
                "Path has an empty final filename",
                span,
            ));
        }
        self.components.push(self.current);
        Ok(RenderedPath { components: self.components, rooted: self.rooted })
    }
}
