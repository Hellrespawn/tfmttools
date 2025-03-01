use serde::{Deserialize, Serialize};
use tfmttools_history_core::{LoadHistoryResult, Record};

use crate::action::Action;

pub type LoadActionHistoryResult = LoadHistoryResult;
pub type ActionRecord = Record<Action, ActionRecordMetadata>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ActionRecordMetadata {
    template: String,
    arguments: Vec<String>,
    run_id: String,
}

impl ActionRecordMetadata {
    pub fn new(
        template: String,
        arguments: Vec<String>,
        run_id: String,
    ) -> Self {
        Self { template, arguments, run_id }
    }

    pub fn template(&self) -> &str {
        self.template.as_ref()
    }

    pub fn arguments(&self) -> &[String] {
        self.arguments.as_ref()
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }
}
