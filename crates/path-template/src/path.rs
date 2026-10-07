/// A rendered path with structural directory boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedPath {
    pub(crate) components: Vec<String>,
    pub(crate) rooted: bool,
}

impl RenderedPath {
    #[must_use]
    pub fn components(&self) -> &[String] {
        &self.components
    }
}
