use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum StoredTagValueKind {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "locator")]
    Locator,
}
