use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum StoredAction {
    MoveFile { source: String, target: String },
    CopyFile { source: String, target: String },
    RemoveFile { path: String },
    MakeDir { path: String },
    RemoveDir { path: String },
    EditTagValues { path: String, changes: Vec<StoredTagValueChange> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct StoredTagValueChange {
    pub key: String,
    pub kind: StoredTagValueKind,
    pub old_value: String,
    pub new_value: String,
    #[serde(default)]
    pub old_encoding: Option<String>,
    #[serde(default)]
    pub new_encoding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StoredTagValueKind {
    Text,
    Locator,
}
