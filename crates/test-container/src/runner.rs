use std::env;
use std::error::Error;
use std::process::ExitCode;
use std::time::Instant;

use camino::Utf8PathBuf;
use color_eyre::Result;
use libtest_mimic::Arguments;
use tfmttools_test_harness::{
    CaseOutcome, ContainerImageSource, ContainerRunDetails, RunFailure,
    Status,
};

use crate::case::{CaseRunContext, run_case};
use crate::image::{
    ImageConfig, ImageBuildFailure, ensure_image, required_env_flag,
    required_env_u64,
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
    let test_args = Arguments::from_args();

    let outcome = run_suite(&test_args);
    let (cases, details, exit_code, status, run_failure) = match outcome {
        Ok(SuiteOutcome {
            cases,
            details,
            exit_code,
            status,
            run_failure,
        }) => (cases, details, exit_code, status, run_failure),
        Err(error) => {
            let run_failure = RunFailure::new(error.to_string());
            (
                Vec::new(),
                ContainerRunDetails::default(),
                ExitCode::FAILURE,
                Some(Status::Failed),
                Some(run_failure),
            )
        },
    };

    write_container_report(ReportInput {
        started_at,
        duration_ms: run_started.elapsed().as_millis(),
        argv,
        filters: tfmttools_test_harness::ReportFilters::new(
            test_args.filter.clone(),
            test_args.skip.clone(),
            test_args.exact,
        ),
        cases,
        details,
        status,
        run_failure,
    })?;

    Ok(exit_code)
}

fn run_suite(args: &Arguments) -> Result<SuiteOutcome> {
    let required = required_env_flag("TFMT_CONTAINER_REQUIRED")?;
    let preserve = required_env_flag("TFMT_CONTAINER_PRESERVE")?;
    let timeout_seconds = required_env_u64(
        "TFMT_CONTAINER_TIMEOUT_SECONDS",
        DEFAULT_COMMAND_TIMEOUT_SECONDS,
    )?;

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
            run_failure: None,
        });
    };

    let image_config = ImageConfig::from_env()?;
    let image_info = match ensure_image(&runtime, &image_config) {
        Ok(image_info) => image_info,
        Err(ImageBuildFailure { reason, stdout, stderr }) => {
            let details = ContainerRunDetails::new(
                runtime.command().to_owned(),
                runtime.version_output().unwrap_or_default(),
                image_config.image.clone(),
                "failed".to_owned(),
                None,
                ContainerImageSource::new(
                    ".".to_owned(),
                    "tests/container/Containerfile".to_owned(),
                    "rust:1.89-bookworm".to_owned(),
                    "debian:bookworm-slim".to_owned(),
                ),
                timeout_seconds,
                preserve,
                Vec::new(),
                Vec::new(),
                stdout,
                stderr,
            );

            return Ok(SuiteOutcome {
                cases: Vec::new(),
                details,
                exit_code: ExitCode::FAILURE,
                status: Some(Status::Failed),
                run_failure: Some(RunFailure::new(reason)),
            });
        },
    };
    let runtime_version_output = runtime.version_output()?;
    let image_source = ContainerImageSource::new(
        image_info.source.build_context,
        image_info.source.containerfile,
        image_info.source.builder_base,
        image_info.source.runtime_base,
    );
    let run_id = build_run_id(&timestamp());
    let workspace_root = workspace_root();
    let case_context = CaseRunContext {
        runtime: &runtime,
        image: &image_config.image,
        timeout_seconds,
        preserve,
        run_id: &run_id,
        workspace_root: &workspace_root,
    };
    let mut volume_names = Vec::new();
    let mut cleanup_commands = Vec::new();
    let cases = discover_cases(args)?
        .into_iter()
        .map(|case| {
            let executed = run_case(&case, &case_context);
            volume_names.extend(executed.volume_names);
            cleanup_commands.extend(executed.cleanup_commands);
            executed.outcome
        })
        .collect::<Vec<CaseOutcome>>();
    let exit_code = if cases.iter().all(CaseOutcome::passed) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    };

    Ok(SuiteOutcome {
        cases,
        details: ContainerRunDetails::new(
            runtime.command().to_owned(),
            runtime_version_output,
            image_config.image,
            image_info.build.as_str().to_owned(),
            image_info.id,
            image_source,
            timeout_seconds,
            preserve,
            volume_names,
            cleanup_commands,
            None,
            None,
        ),
        exit_code,
        status: None,
        run_failure: None,
    })
}

struct SuiteOutcome {
    cases: Vec<CaseOutcome>,
    details: ContainerRunDetails,
    exit_code: ExitCode,
    status: Option<Status>,
    run_failure: Option<RunFailure>,
}

fn build_run_id(started_at: &str) -> String {
    started_at
        .chars()
        .filter_map(|char| {
            if char.is_ascii_alphanumeric() {
                Some(char.to_ascii_lowercase())
            } else {
                None
            }
        })
        .take(24)
        .collect()
}

fn workspace_root() -> Utf8PathBuf {
    Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
