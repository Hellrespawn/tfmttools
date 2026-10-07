use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::action::Action;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RecordState {
    Applied,
    Undone,
    Redone,
    Superseded,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Record {
    id: Option<usize>,
    actions: Vec<Action>,
    state: RecordState,
    timestamp: DateTime<Local>,
    metadata: ActionRecordMetadata,
}

impl Record {
    pub fn new(items: Vec<Action>, metadata: ActionRecordMetadata) -> Self {
        Self {
            id: None,
            actions: items,
            state: RecordState::Applied,
            timestamp: Local::now(),
            metadata,
        }
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &Action> {
        self.actions.iter()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn id(&self) -> Option<usize> {
        self.id
    }

    pub fn id_mut(&mut self) -> &mut Option<usize> {
        &mut self.id
    }

    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    pub fn timestamp(&self) -> DateTime<Local> {
        self.timestamp
    }

    pub fn metadata(&self) -> &ActionRecordMetadata {
        &self.metadata
    }

    pub fn state(&self) -> RecordState {
        self.state
    }

    pub fn set_state(&mut self, state: RecordState) {
        self.state = state;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]

pub enum TemplateMetadata {
    FileOrName(String),
    // Preserve the serialized variant name in existing history files.
    #[serde(rename = "Script")]
    InlineTemplate(String),
    Validation(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionRecordMetadata {
    template: TemplateMetadata,
    arguments: Vec<String>,
    run_id: String,
}

impl ActionRecordMetadata {
    #[must_use]
    pub fn new(
        template: TemplateMetadata,
        arguments: Vec<String>,
        run_id: String,
    ) -> Self {
        Self { template, arguments, run_id }
    }

    #[must_use]
    pub fn template(&self) -> &TemplateMetadata {
        &self.template
    }

    #[must_use]
    pub fn arguments(&self) -> &[String] {
        self.arguments.as_ref()
    }

    #[must_use]
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}
