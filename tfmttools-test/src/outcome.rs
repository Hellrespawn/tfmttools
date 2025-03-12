use std::process::Output;

use camino::Utf8PathBuf;
use serde::Serialize;

#[derive(Default, Debug, Clone, Serialize)]
pub struct TestCaseOutcome {
    name: String,
    description: String,
    work_dir: Utf8PathBuf,
    test_outcomes: Vec<TestOutcome>,
}

impl TestCaseOutcome {
    pub fn new(
        name: String,
        description: String,
        work_dir: Utf8PathBuf,
        test_outcomes: Vec<TestOutcome>,
    ) -> Self {
        Self { name, description, work_dir, test_outcomes }
    }

    pub fn passed(&self) -> bool {
        self.test_outcomes.iter().all(|outcome| outcome.passed())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TestOutcome {
    name: String,
    command_outcome: Option<CommandOutcome>,
    expectations_outcome: ExpectationsOutcome,
    passed: bool,
}

impl TestOutcome {
    pub fn new(
        name: String,
        command_outcome: Option<CommandOutcome>,
        expectations_outcome: ExpectationsOutcome,
    ) -> Self {
        Self {
            name,
            command_outcome,
            passed: expectations_outcome.passed,
            expectations_outcome,
        }
    }

    pub fn passed(&self) -> bool {
        self.passed
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutcome {
    arguments: Vec<String>,
    success: bool,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl CommandOutcome {
    pub fn new(arguments: Vec<String>, output: Output) -> Self {
        let success = output.status.success();
        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        Self { arguments, success, exit_code, stdout, stderr }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpectationsOutcome {
    files_remaining_after_previous: Option<Vec<Utf8PathBuf>>,
    outcomes: Vec<ExpectationOutcome>,
    passed: bool,
}

impl ExpectationsOutcome {
    pub fn new(
        files_remaining_after_previous: Option<Vec<Utf8PathBuf>>,
        outcomes: Vec<ExpectationOutcome>,
    ) -> Self {
        let passed = files_remaining_after_previous
            .as_ref()
            .is_none_or(|v| v.is_empty())
            && outcomes.iter().all(|outcome| outcome.passed());

        Self { files_remaining_after_previous, outcomes, passed }
    }
}
#[derive(Debug, Clone, Serialize)]
pub enum ExpectationOutcome {
    Ok(Utf8PathBuf),
    NotPresent(Utf8PathBuf),
    ChecksumMismatch { path: Utf8PathBuf, expected: String, actual: String },
}

impl ExpectationOutcome {
    pub fn passed(&self) -> bool {
        matches!(self, ExpectationOutcome::Ok(..))
    }
}
