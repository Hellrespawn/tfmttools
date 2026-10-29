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

    /// Transform component text before converting it to a filesystem path.
    pub fn try_map_components<E>(
        mut self,
        mut map: impl FnMut(&str) -> Result<String, E>,
    ) -> Result<Self, E> {
        for component in &mut self.components {
            *component = map(component)?;
        }
        Ok(self)
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
        if let Some(character) = crate::prohibited_character(text) {
            return Err(Diagnostic::new(
                format!(
                    "Component contains prohibited character {character:?}"
                ),
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
