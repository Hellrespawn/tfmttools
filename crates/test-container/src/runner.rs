use std::env;
use std::error::Error;
use std::process::ExitCode;
use std::time::Instant;

use color_eyre::Result;
use tfmttools_test_harness::{
    CaseOutcome, ContainerImageSource, ContainerRunDetails, Status,
};

use crate::case::run_case;
use crate::image::{
    ImageConfig, ensure_image, required_env_flag, required_env_u64,
};
use crate::protocol::DEFAULT_COMMAND_TIMEOUT_SECONDS;
use crate::report::{ReportInput, timestamp, write_container_report};
use crate::runtime::ContainerRuntime;
use crate::scenario::discover_cases;

pub fn test_runner() -> Result<ExitCode, Box<dyn Error>> {
    color_eyre::install()?;

    let run_started = Instant::now();
    let started_at = timestamp();
    let argv = env::args().collect::<Vec<_>>();

    let outcome = run_suite();
    let (cases, details, exit_code, status) = match outcome {
        Ok(SuiteOutcome { cases, details, exit_code, status }) => {
            (cases, details, exit_code, status)
        },
        Err(error) => {
            let details = ContainerRunDetails::failed_setup(error.to_string());
            (Vec::new(), details, ExitCode::FAILURE, Some(Status::Failed))
        },
    };

    write_container_report(ReportInput {
        started_at,
        duration_ms: run_started.elapsed().as_millis(),
        argv,
        cases,
        details,
        status,
    })?;

    Ok(exit_code)
}

fn run_suite() -> Result<SuiteOutcome> {
    let required = required_env_flag("TFMT_CONTAINER_REQUIRED")?;
    let preserve = required_env_flag("TFMT_CONTAINER_PRESERVE")?;
    let timeout_seconds = required_env_u64(
        "TFMT_CONTAINER_TIMEOUT_SECONDS",
        DEFAULT_COMMAND_TIMEOUT_SECONDS,
    )?;
    let runtime_configured = env::var_os("TFMT_CONTAINER_RUNTIME").is_some();

    if !required && !runtime_configured {
        let details = ContainerRunDetails::skipped(
            timeout_seconds,
            "set TFMT_CONTAINER_REQUIRED=1 or use cargo xtask test-container",
        );

        return Ok(SuiteOutcome {
            cases: Vec::new(),
            details,
            exit_code: ExitCode::SUCCESS,
            status: None,
        });
    }

    let Some(runtime) = ContainerRuntime::detect(required)? else {
        let details = ContainerRunDetails::skipped(
            timeout_seconds,
            "no docker or podman runtime found",
        );

        return Ok(SuiteOutcome {
            cases: Vec::new(),
            details,
            exit_code: ExitCode::SUCCESS,
            status: None,
        });
    };

    let image_config = ImageConfig::from_env()?;
    let image_info = ensure_image(&runtime, &image_config)?;
    let image_source = ContainerImageSource::new(
        image_info.source.build_context,
        image_info.source.containerfile,
        image_info.source.builder_base,
        image_info.source.runtime_base,
    );
    let mut cases = Vec::new();

    for case in discover_cases() {
        cases.push(run_case(&case));
    }

    Ok(SuiteOutcome {
        cases,
        details: ContainerRunDetails::new(
            runtime.command().to_owned(),
            image_config.image,
            image_info.build.as_str().to_owned(),
            image_info.id,
            image_source,
            timeout_seconds,
            preserve,
        ),
        exit_code: ExitCode::SUCCESS,
        status: None,
    })
}

struct SuiteOutcome {
    cases: Vec<CaseOutcome>,
    details: ContainerRunDetails,
    exit_code: ExitCode,
    status: Option<Status>,
}
