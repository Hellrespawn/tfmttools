use std::collections::BTreeMap;
use std::env;
use std::process::Command as StdCommand;

use chrono::{SecondsFormat, Utc};
use color_eyre::Result;
use tfmttools_test_harness::{
    CaseOutcome, ContainerRunDetails, FixtureDirs, ReportEnvelope,
    ReportFilters, RunFailure, Runner, RunnerDetails, SourceMetadata, Status,
    write_report,
};

pub struct ReportInput {
    pub started_at: String,
    pub duration_ms: u128,
    pub argv: Vec<String>,
    pub filters: ReportFilters,
    pub cases: Vec<CaseOutcome>,
    pub details: ContainerRunDetails,
    pub status: Option<Status>,
    pub run_failure: Option<RunFailure>,
}

pub fn write_container_report(input: ReportInput) -> Result<()> {
    let report_dir = FixtureDirs::reports_dir().join("container");
    fs_err::create_dir_all(&report_dir)?;
    let canonical_report_dir = report_dir.canonicalize_utf8().ok();
    let mut report = ReportEnvelope::new(
        Runner::Container,
        input.started_at,
        timestamp(),
        input.duration_ms,
        input.argv,
        input.filters,
        harness_environment(),
        canonical_report_dir,
        input.cases,
        input.run_failure,
        source_metadata(),
        RunnerDetails::Container(Box::new(input.details)),
    );
    if let Some(status) = input.status {
        report = report.with_status(status);
    }

    write_report(&report_dir, report)
}

pub fn harness_environment() -> BTreeMap<String, String> {
    env::vars()
        .filter(|(name, _)| name.starts_with("TFMT_CONTAINER_"))
        .collect()
}

pub fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn source_metadata() -> SourceMetadata {
    SourceMetadata::new(
        git_output(&["rev-parse", "HEAD"]),
        git_dirty(),
        git_output(&["diff", "--stat"]),
    )
}

fn git_dirty() -> Option<bool> {
    let output = StdCommand::new("git")
        .args(["status", "--short"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    Some(!String::from_utf8_lossy(&output.stdout).trim().is_empty())
}

fn git_output(args: &[&str]) -> Option<String> {
    let output = StdCommand::new("git").args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!stdout.is_empty()).then_some(stdout)
}
