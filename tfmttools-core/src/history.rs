use serde::{Deserialize, Serialize};
use tfmttools_history::{History, Record};

use crate::action::Action;

pub type ActionHistory = History<Action, ActionRecordMetadata>;
pub type ActionRecord = Record<Action, ActionRecordMetadata>;

#[derive(Debug, Serialize, Deserialize)]
pub struct ActionRecordMetadata {
    template: String,
    arguments: Vec<String>,
}

impl ActionRecordMetadata {
    pub fn new(template: String, arguments: Vec<String>) -> Self {
        Self { template, arguments }
    }

    pub fn template(&self) -> &str {
        self.template.as_ref()
    }

    pub fn arguments(&self) -> &[String] {
        self.arguments.as_ref()
    }
}
