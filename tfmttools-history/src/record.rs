use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T, M> {
    items: Vec<T>,
    timestamp: DateTime<Local>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<M>,
}

impl<T, M> Record<T, M> {
    pub fn new(items: Vec<T>) -> Self {
        Self { items, timestamp: Local::now(), metadata: None }
    }

    pub fn with_metadata(items: Vec<T>, metadata: M) -> Self {
        Self { items, timestamp: Local::now(), metadata: Some(metadata) }
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

    pub fn timestamp(&self) -> DateTime<Local> {
        self.timestamp
    }

    pub fn metadata(&self) -> Option<&M> {
        self.metadata.as_ref()
    }
}
