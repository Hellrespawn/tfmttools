// Crate-specific exception; shared Clippy policy lives in Cargo.toml.
#![allow(clippy::must_use_candidate)]

mod checksum;
mod context;
mod data;
mod outcome;
mod report;

pub use checksum::get_path_checksum;
pub use context::{FixtureDirs, copy_files};
pub use data::{Expectation, ExpectationOption, TestCaseData, TestData};
pub use outcome::{
    CaseOutcome, CliCaseDetails, CliRunDetails, CommandOutcome,
    ExpectationOutcome, ExpectationVerification, ExpectationsOutcome,
    ReportEnvelope, ReportFilters, ReportSummary, RunFailure, Runner,
    RunnerDetails, SourceMetadata, Status, StepOutcome,
};
pub use report::write_report;
