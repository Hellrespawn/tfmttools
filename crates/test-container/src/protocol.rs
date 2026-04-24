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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyCode {
    FilesystemOk,
    PathMissing,
    PathUnexpected,
    ChecksumMismatch,
    HistoryRecordMissing,
    HistoryActionMissing,
    HistoryActionUnexpected,
    MountUnknown,
    HistoryLoadFailed,
    VerifierError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub mount_aliases: BTreeMap<String, String>,
    pub outcomes: Vec<VerifyOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyOutcome {
    pub status: VerifyStatus,
    pub code: VerifyCode,
    pub path: Option<Utf8PathBuf>,
    pub message: String,
    pub exists: Option<bool>,
    pub expected_checksum: Option<String>,
    pub actual_checksum: Option<String>,
    pub history_record: Option<usize>,
    pub action: Option<ActionName>,
    pub actual_actions: Vec<ActionName>,
}

impl VerifyOutcome {
    #[must_use]
    pub fn ok(path: Utf8PathBuf, message: impl Into<String>) -> Self {
        Self {
            status: VerifyStatus::Passed,
            code: VerifyCode::FilesystemOk,
            path: Some(path),
            message: message.into(),
            exists: Some(true),
            expected_checksum: None,
            actual_checksum: None,
            history_record: None,
            action: None,
            actual_actions: Vec::new(),
        }
    }

    #[must_use]
    pub fn failure(
        code: VerifyCode,
        path: Option<Utf8PathBuf>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: VerifyStatus::Failed,
            code,
            path,
            message: message.into(),
            exists: None,
            expected_checksum: None,
            actual_checksum: None,
            history_record: None,
            action: None,
            actual_actions: Vec::new(),
        }
    }

    #[must_use]
    pub fn passed(&self) -> bool {
        self.status == VerifyStatus::Passed
    }
}
