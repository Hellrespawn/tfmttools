use std::collections::BTreeMap;
use std::process::Output;

use camino::Utf8PathBuf;
use serde::Serialize;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Passed,
    Failed,
    Skipped,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Runner {
    Cli,
    Container,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportEnvelope {
    schema_version: u32,
    runner: Runner,
    status: Status,
    started_at: String,
    generated_at: String,
    duration_ms: u128,
    argv: Vec<String>,
    filters: ReportFilters,
    environment: BTreeMap<String, String>,
    report_dir: Option<Utf8PathBuf>,
    artifacts: ReportArtifacts,
    summary: ReportSummary,
    cases: Vec<CaseOutcome>,
    runner_details: RunnerDetails,
}

impl ReportEnvelope {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        runner: Runner,
        started_at: String,
        generated_at: String,
        duration_ms: u128,
        argv: Vec<String>,
        filters: ReportFilters,
        environment: BTreeMap<String, String>,
        report_dir: Option<Utf8PathBuf>,
        cases: Vec<CaseOutcome>,
        runner_details: RunnerDetails,
    ) -> Self {
        let summary = ReportSummary::from_cases(&cases);
        let status = summary.status();
        let artifacts = ReportArtifacts::default();

        Self {
            schema_version: SCHEMA_VERSION,
            runner,
            status,
            started_at,
            generated_at,
            duration_ms,
            argv,
            filters,
            environment,
            report_dir,
            artifacts,
            summary,
            cases,
            runner_details,
        }
    }

    pub fn status(&self) -> Status {
        self.status
    }

    #[must_use]
    pub fn with_status(mut self, status: Status) -> Self {
        self.status = status;
        self
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReportFilters {
    filter: Option<String>,
    skip: Vec<String>,
    exact: bool,
}

impl ReportFilters {
    pub fn new(filter: Option<String>, skip: Vec<String>, exact: bool) -> Self {
        Self { filter, skip, exact }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportArtifacts {
    report_html: String,
    report_json: String,
}

impl Default for ReportArtifacts {
    fn default() -> Self {
        Self {
            report_html: "report.html".to_owned(),
            report_json: "report.json".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ReportSummary {
    total: usize,
    passed: usize,
    failed: usize,
    skipped: usize,
    timed_out: usize,
}

impl ReportSummary {
    pub fn from_cases(cases: &[CaseOutcome]) -> Self {
        let mut summary = Self { total: cases.len(), ..Self::default() };

        for case in cases {
            match case.status {
                Status::Passed => summary.passed += 1,
                Status::Failed => summary.failed += 1,
                Status::Skipped => summary.skipped += 1,
                Status::TimedOut => summary.timed_out += 1,
            }
        }

        summary
    }

    pub fn status(&self) -> Status {
        if self.timed_out > 0 {
            Status::TimedOut
        } else if self.failed > 0 {
            Status::Failed
        } else if self.total == 0 || self.skipped == self.total {
            Status::Skipped
        } else {
            Status::Passed
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerDetails {
    Cli(CliRunDetails),
    Container(ContainerRunDetails),
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CliRunDetails {}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ContainerRunDetails {
    runtime: Option<String>,
    image: Option<String>,
    image_build: Option<String>,
    command_timeout_seconds: u64,
    preserve: bool,
    setup_error: Option<String>,
    skip_reason: Option<String>,
}

impl ContainerRunDetails {
    pub fn new(
        runtime: String,
        image: String,
        image_build: String,
        command_timeout_seconds: u64,
        preserve: bool,
    ) -> Self {
        Self {
            runtime: Some(runtime),
            image: Some(image),
            image_build: Some(image_build),
            command_timeout_seconds,
            preserve,
            setup_error: None,
            skip_reason: None,
        }
    }

    pub fn skipped(
        command_timeout_seconds: u64,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            command_timeout_seconds,
            skip_reason: Some(reason.into()),
            ..Self::default()
        }
    }

    pub fn failed_setup(error: impl Into<String>) -> Self {
        Self { setup_error: Some(error.into()), ..Self::default() }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseOutcome {
    name: String,
    description: String,
    status: Status,
    duration_ms: u128,
    steps: Vec<StepOutcome>,
    cli: Option<CliCaseDetails>,
}

impl CaseOutcome {
    pub fn new(
        name: String,
        description: String,
        duration_ms: u128,
        steps: Vec<StepOutcome>,
        cli: Option<CliCaseDetails>,
    ) -> Self {
        let status = if steps.iter().any(|step| step.status == Status::TimedOut)
        {
            Status::TimedOut
        } else if steps.iter().any(|step| step.status == Status::Failed) {
            Status::Failed
        } else if steps.is_empty()
            || steps.iter().all(|step| step.status == Status::Skipped)
        {
            Status::Skipped
        } else {
            Status::Passed
        };

        Self { name, description, status, duration_ms, steps, cli }
    }

    pub fn passed(&self) -> bool {
        self.status == Status::Passed
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CliCaseDetails {
    work_dir: Utf8PathBuf,
}

impl CliCaseDetails {
    pub fn new(work_dir: Utf8PathBuf) -> Self {
        Self { work_dir }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StepOutcome {
    name: String,
    status: Status,
    skip_reason: Option<String>,
    duration_ms: u128,
    command_outcome: Option<CommandOutcome>,
    expectations_outcome: Option<ExpectationsOutcome>,
}

impl StepOutcome {
    pub fn new(
        name: String,
        duration_ms: u128,
        command_outcome: Option<CommandOutcome>,
        expectations_outcome: ExpectationsOutcome,
    ) -> Self {
        let command_passed =
            command_outcome.as_ref().is_none_or(CommandOutcome::passed);
        let status = if command_passed && expectations_outcome.passed() {
            Status::Passed
        } else {
            Status::Failed
        };

        Self {
            name,
            status,
            skip_reason: None,
            duration_ms,
            command_outcome,
            expectations_outcome: Some(expectations_outcome),
        }
    }

    pub fn skipped(name: String, reason: impl Into<String>) -> Self {
        Self {
            name,
            status: Status::Skipped,
            skip_reason: Some(reason.into()),
            duration_ms: 0,
            command_outcome: None,
            expectations_outcome: None,
        }
    }

    pub fn passed(&self) -> bool {
        self.status == Status::Passed
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutcome {
    arguments: Vec<String>,
    status: Status,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl CommandOutcome {
    pub fn new(arguments: Vec<String>, output: &Output) -> Self {
        let status = if output.status.success() {
            Status::Passed
        } else {
            Status::Failed
        };
        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        Self { arguments, status, exit_code, stdout, stderr }
    }

    pub fn passed(&self) -> bool {
        self.status == Status::Passed
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExpectationsOutcome {
    files_remaining_after_previous: Option<Vec<Utf8PathBuf>>,
    outcomes: Vec<ExpectationOutcome>,
    status: Status,
}

impl ExpectationsOutcome {
    pub fn new(
        files_remaining_after_previous: Option<Vec<Utf8PathBuf>>,
        outcomes: Vec<ExpectationOutcome>,
    ) -> Self {
        let passed = files_remaining_after_previous
            .as_ref()
            .is_none_or(std::vec::Vec::is_empty)
            && outcomes.iter().all(ExpectationOutcome::passed);
        let status = if passed { Status::Passed } else { Status::Failed };

        Self { files_remaining_after_previous, outcomes, status }
    }

    pub fn passed(&self) -> bool {
        self.status == Status::Passed
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
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
