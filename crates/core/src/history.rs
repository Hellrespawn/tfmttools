use serde::{Deserialize, Serialize};
use tfmttools_history::Record;

use crate::action::Action;

pub type ActionRecord = Record<Action, ActionRecordMetadata>;

#[derive(Debug, Serialize, Deserialize, Clone)]

pub enum TemplateMetadata {
    FileOrName(String),
    Script(String),
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
