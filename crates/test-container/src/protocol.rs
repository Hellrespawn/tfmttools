use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

pub const DEFAULT_COMMAND_TIMEOUT_SECONDS: u64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyRequest {
    pub mounts: BTreeMap<String, String>,
    pub filesystem: Vec<FilesystemExpectation>,
    pub history: Vec<HistoryExpectation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemExpectation {
    pub mount: String,
    pub path: String,
    pub exists: bool,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryExpectation {
    pub mount: String,
    pub path: String,
    pub record: usize,
    pub contains_actions: Vec<ActionName>,
    pub does_not_contain_actions: Vec<ActionName>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionName {
    MoveFile,
    CopyFile,
    RemoveFile,
    MakeDir,
    RemoveDir,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub outcomes: Vec<VerifyOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum VerifyOutcome {
    Ok {
        path: Utf8PathBuf,
    },
    NotPresent {
        path: Utf8PathBuf,
    },
    UnexpectedPresent {
        path: Utf8PathBuf,
    },
    ChecksumMismatch {
        path: Utf8PathBuf,
        expected: String,
        actual: String,
    },
    HistoryRecordMissing {
        path: Utf8PathBuf,
        record: usize,
        message: String,
    },
    HistoryActionMissing {
        path: Utf8PathBuf,
        record: usize,
        action: ActionName,
        actual: Vec<ActionName>,
    },
    HistoryActionUnexpected {
        path: Utf8PathBuf,
        record: usize,
        action: ActionName,
        actual: Vec<ActionName>,
    },
    Error {
        message: String,
    },
}

impl VerifyOutcome {
    #[must_use]
    pub fn passed(&self) -> bool {
        matches!(self, Self::Ok { .. })
    }
}
