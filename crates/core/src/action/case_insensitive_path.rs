use std::collections::HashSet;
use std::fmt::Display;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CaseInsensitivePathKey(String);

impl CaseInsensitivePathKey {
    #[must_use]
    pub fn new(path: impl Display) -> Self {
        Self(path.to_string().to_lowercase())
    }
}

#[derive(Debug, Default)]
pub struct CaseInsensitivePathSet {
    keys: HashSet<CaseInsensitivePathKey>,
}

impl CaseInsensitivePathSet {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: impl Display) -> bool {
        self.keys.insert(CaseInsensitivePathKey::new(path))
    }

    #[must_use]
    pub fn contains(&self, path: impl Display) -> bool {
        self.keys.contains(&CaseInsensitivePathKey::new(path))
    }
}
