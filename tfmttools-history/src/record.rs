use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum RecordState {
    Applied,
    Undone,
    Redone,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Record<A, M> {
    actions: Vec<A>,
    state: RecordState,
    timestamp: DateTime<Local>,
    metadata: M,
}

impl<A, M> Record<A, M> {
    pub fn new(items: Vec<A>, metadata: M) -> Self {
        Self {
            actions: items,
            state: RecordState::Applied,
            timestamp: Local::now(),
            metadata,
        }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &A> {
        self.actions.iter()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn actions(&self) -> &[A] {
        &self.actions
    }

    pub fn timestamp(&self) -> DateTime<Local> {
        self.timestamp
    }

    pub fn metadata(&self) -> &M {
        &self.metadata
    }

    pub fn state(&self) -> RecordState {
        self.state
    }

    pub fn set_state(&mut self, state: RecordState) {
        self.state = state;
    }
}
