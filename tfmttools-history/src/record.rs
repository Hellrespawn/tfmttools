use chrono::{DateTime, Local};
use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Record<T> {
    items: Vec<T>,
    timestamp: Option<DateTime<Local>>,
}

impl<T> Record<T> {
    pub fn new(items: Vec<T>) -> Result<Self> {
        Ok(Self { items, timestamp: Some(Local::now()) })
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

    pub fn timestamp(&self) -> Option<DateTime<Local>> {
        self.timestamp
    }
}
