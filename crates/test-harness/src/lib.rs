#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::must_use_candidate)]

mod context;
mod data;
mod outcome;
mod report;

pub use context::{FixtureDirs, copy_files};
pub use data::{Expectation, ExpectationOption, TestCaseData, TestData};
pub use outcome::{
    CaseOutcome, CliCaseDetails, CliRunDetails, CommandOutcome,
    ContainerImageSource, ContainerRunDetails, ExpectationOutcome,
    ExpectationsOutcome, ReportEnvelope, ReportFilters, ReportSummary, Runner,
    RunnerDetails, Status, StepOutcome,
};
pub use report::write_report;
