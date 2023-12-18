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
        Ok(Self { items, timestamp: Some(OffsetDateTime::now_local()?) })
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &T> {
        self.items.iter()
    }
}
