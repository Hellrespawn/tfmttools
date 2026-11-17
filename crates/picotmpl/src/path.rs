use std::path::PathBuf;

use crate::{Diagnostic, Span};

/// A rendered path with structural directory boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedPath {
    pub(crate) components: Vec<String>,
    pub(crate) anchor: PathBuf,
}

impl RenderedPath {
    #[must_use]
    pub fn is_rooted(&self) -> bool {
        self.anchor.has_root()
    }

    #[must_use]
    pub fn to_path_buf(&self) -> PathBuf {
        let mut path = self.anchor.clone();
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
    anchor: PathBuf,
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
        self.anchor.push(std::path::MAIN_SEPARATOR_STR);
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
        path: RenderedPath,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if !self.current.is_empty() {
            return Err(Diagnostic::new(
                "Path arguments require a component boundary",
                span,
            ));
        }
        if !path.anchor.as_os_str().is_empty() {
            if !self.components.is_empty()
                || !self.anchor.as_os_str().is_empty()
            {
                return Err(Diagnostic::new(
                    "Rooted or prefixed path arguments must start the output path",
                    span,
                ));
            }
            self.anchor = path.anchor;
        }
        for component in path.components {
            match component.as_str() {
                "." => {},
                ".." => {
                    if self.components.pop().is_none()
                        && !self.anchor.has_root()
                    {
                        self.anchor.push("..");
                    }
                },
                _ => self.components.push(component),
            }
        }
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
        Ok(RenderedPath { components: self.components, anchor: self.anchor })
    }
}
