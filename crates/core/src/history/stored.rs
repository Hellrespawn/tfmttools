use serde::{Deserialize, Serialize};

use super::{HistoryError, Record, Result};
use crate::action::Action;

pub const CURRENT_SCHEMA_VERSION: u64 = 1;

/// The published history document. Runtime state is never serialized here.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredHistory {
    #[serde(rename = "schema_version")]
    pub schema_version: u64,
    #[serde(rename = "records")]
    pub records: Vec<Record>,
}

impl StoredHistory {
    pub(super) fn current(records: Vec<Record>) -> Self {
        Self { schema_version: CURRENT_SCHEMA_VERSION, records }
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(HistoryError::LoadError(format!(
                "Unsupported history schema version {}",
                self.schema_version
            )));
        }
        for (record_index, record) in self.records.iter().enumerate() {
            for (action_index, action) in record.iter().enumerate() {
                Action::try_from(action).map_err(|error| {
                    HistoryError::LoadError(format!(
                        "records[{record_index}].actions[{action_index}]: {error}"
                    ))
                })?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum StoredAction {
    #[serde(rename = "move_file")]
    MoveFile {
        #[serde(rename = "source")]
        source: String,
        #[serde(rename = "target")]
        target: String,
    },
    #[serde(rename = "copy_file")]
    CopyFile {
        #[serde(rename = "source")]
        source: String,
        #[serde(rename = "target")]
        target: String,
    },
    #[serde(rename = "remove_file")]
    RemoveFile {
        #[serde(rename = "path")]
        path: String,
    },
    #[serde(rename = "make_dir")]
    MakeDir {
        #[serde(rename = "path")]
        path: String,
    },
    #[serde(rename = "remove_dir")]
    RemoveDir {
        #[serde(rename = "path")]
        path: String,
    },
    #[serde(rename = "edit_tag_values")]
    EditTagValues {
        #[serde(rename = "path")]
        path: String,
        #[serde(rename = "changes")]
        changes: Vec<StoredTagValueChange>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredTagValueChange {
    #[serde(rename = "key")]
    pub key: String,
    #[serde(rename = "kind")]
    pub kind: StoredTagValueKind,
    #[serde(rename = "old_value")]
    pub old_value: String,
    #[serde(rename = "new_value")]
    pub new_value: String,
    #[serde(rename = "old_encoding", default)]
    pub old_encoding: Option<String>,
    #[serde(rename = "new_encoding", default)]
    pub new_encoding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoredTagValueKind {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "locator")]
    Locator,
}
