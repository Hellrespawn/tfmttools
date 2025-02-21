use serde::{Deserialize, Serialize};
use tfmttools_history_core::{LoadHistoryResult, Record};

use crate::action::Action;

pub type LoadActionHistoryResult = LoadHistoryResult;
// pub type ActionHistoryExt = dyn HistoryExt<Action, ActionRecordMetadata>;
pub type ActionRecord = Record<Action, ActionRecordMetadata>;

#[derive(Debug, Serialize, Deserialize, Clone)]
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
