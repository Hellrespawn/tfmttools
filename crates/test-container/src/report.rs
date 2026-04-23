use std::collections::BTreeMap;
use std::env;

use chrono::{SecondsFormat, Utc};
use color_eyre::Result;
use tfmttools_test_harness::{
    CaseOutcome, ContainerRunDetails, FixtureDirs, ReportEnvelope,
    ReportFilters, Runner, RunnerDetails, Status, write_report,
};

pub struct ReportInput {
    pub started_at: String,
    pub duration_ms: u128,
    pub argv: Vec<String>,
    pub filters: ReportFilters,
    pub cases: Vec<CaseOutcome>,
    pub details: ContainerRunDetails,
    pub status: Option<Status>,
}

pub fn write_container_report(input: ReportInput) -> Result<()> {
    let report_dir = FixtureDirs::reports_dir();
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
