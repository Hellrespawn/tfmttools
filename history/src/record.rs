use color_eyre::Result;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T> {
    items: Vec<T>,
    timestamp: Option<OffsetDateTime>,
}

impl<T> Record<T> {
    pub fn new(items: Vec<T>) -> Result<Self> {
        Ok(Self { items, timestamp: Some(OffsetDateTime::now_utc()) })
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.items.iter()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn timestamp(&self) -> Option<OffsetDateTime> {
        self.timestamp
    }
}
