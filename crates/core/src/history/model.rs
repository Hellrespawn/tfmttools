use chrono::{DateTime, FixedOffset, Local};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::StoredAction;

#[derive(
    Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq,
)]
pub enum RecordState {
    #[serde(rename = "applied")]
    Applied,
    #[serde(rename = "undone")]
    Undone,
    #[serde(rename = "redone")]
    Redone,
    #[serde(rename = "superseded")]
    Superseded,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(deny_unknown_fields)]
pub struct Record {
    #[serde(rename = "id")]
    id: Option<usize>,
    #[serde(rename = "actions")]
    actions: Vec<StoredAction>,
    #[serde(rename = "state")]
    state: RecordState,
    #[serde(rename = "timestamp")]
    timestamp: DateTime<FixedOffset>,
    #[serde(rename = "metadata")]
    metadata: ActionRecordMetadata,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    applied_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    complete: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    redo_allowed: Option<bool>,
}

impl Record {
    #[must_use]
    pub fn new(
        items: Vec<StoredAction>,
        metadata: ActionRecordMetadata,
    ) -> Self {
        Self {
            id: None,
            actions: items,
            state: RecordState::Applied,
            timestamp: Local::now().fixed_offset(),
            metadata,
            applied_count: None,
            complete: None,
            redo_allowed: None,
        }
    }

    pub(super) fn from_storage(
        id: usize,
        actions: Vec<StoredAction>,
        state: RecordState,
        timestamp: DateTime<FixedOffset>,
        metadata: ActionRecordMetadata,
    ) -> Self {
        Self {
            id: Some(id),
            actions,
            state,
            timestamp,
            metadata,
            applied_count: None,
            complete: None,
            redo_allowed: None,
        }
    }

    pub(super) fn set_progress(
        &mut self,
        applied: usize,
        complete: bool,
        redo: bool,
    ) {
        self.applied_count = Some(applied);
        self.complete = Some(complete);
        self.redo_allowed = Some(redo);
    }

    #[must_use]
    pub fn applied_count(&self) -> usize {
        self.applied_count.unwrap_or_else(|| {
            if matches!(
                self.state,
                RecordState::Undone | RecordState::Superseded
            ) {
                0
            } else {
                self.len()
            }
        })
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.complete.unwrap_or(true)
    }

    #[must_use]
    pub fn redo_allowed(&self) -> bool {
        self.redo_allowed.unwrap_or(true)
    }

    #[must_use]
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &StoredAction> {
        self.actions.iter()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    #[must_use]
    pub fn id(&self) -> Option<usize> {
        self.id
    }

    #[must_use]
    pub fn actions(&self) -> &[StoredAction] {
        &self.actions
    }

    #[must_use]
    pub fn timestamp(&self) -> DateTime<FixedOffset> {
        self.timestamp
    }

    #[must_use]
    pub fn metadata(&self) -> &ActionRecordMetadata {
        &self.metadata
    }

    #[must_use]
    pub fn state(&self) -> RecordState {
        self.state
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum TemplateMetadata {
    #[serde(rename = "file_or_name")]
    FileOrName {
        #[serde(rename = "value")]
        value: String,
    },
    #[serde(rename = "inline_template")]
    InlineTemplate {
        #[serde(rename = "value")]
        value: String,
    },
    #[serde(rename = "validation")]
    Validation {
        #[serde(rename = "value")]
        value: String,
    },
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(deny_unknown_fields)]
pub struct ActionRecordMetadata {
    #[serde(rename = "template")]
    template: TemplateMetadata,
    #[serde(rename = "arguments")]
    arguments: Vec<String>,
    #[serde(rename = "run_id")]
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
