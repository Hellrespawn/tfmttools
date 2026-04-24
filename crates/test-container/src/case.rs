use std::collections::BTreeMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use serde::{Deserialize, Serialize};
use tfmttools_test_harness::{
    CaseOutcome, CommandOutcome, ContainerStepDetails, ExpectationOutcome,
    ExpectationsOutcome, StepOutcome,
};

use crate::protocol::{
    ActionName, FilesystemExpectation as VerifyFilesystemExpectation,
    HistoryExpectation as VerifyHistoryExpectation, VerifyCode,
    VerifyOutcome, VerifyRequest, VerifyResponse,
};
use crate::runtime::{ContainerRuntime, RuntimeCommandResult};
use crate::scenario::{
    ContainerScenario, Precondition, ScenarioMount, ScenarioPathRef,
    SetupOperation,
};

const CASE_SUFFIX: &str = ".case.json";
const SETUP_STEP_NAME: &str = "setup";
const APP_UID_GID: &str = "1000:1000";
const ROOT_UID_GID: &str = "0:0";
const FIXTURE_MOUNT_PATH: &str = "/fixtures";
const VERIFY_MOUNT_PATH: &str = "/verify";

#[derive(Debug, Clone)]
pub struct ContainerCase {
    id: String,
    scenario: String,
    description: String,
    expectations: BTreeMap<String, Vec<FilesystemExpectation>>,
    history: BTreeMap<String, HistoryExpectation>,
    steps: Vec<CaseStep>,
}

impl ContainerCase {
    pub fn from_file(path: &Utf8Path) -> Result<Self> {
        let body = fs_err::read_to_string(path)?;
        let data: ContainerCaseData = serde_json::from_str(&body)?;
        let id = case_id_from_path(path)?;

        Ok(Self {
            id,
            scenario: data.scenario,
            description: data.description,
            expectations: data.expectations,
            history: data.history.unwrap_or_default(),
            steps: data.steps,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn scenario(&self) -> &str {
        &self.scenario
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    fn validate(&self, scenario: &ContainerScenario) -> Result<()> {
        for (name, expectations) in &self.expectations {
            for expectation in expectations {
                validate_case_path_ref(
                    self.id(),
                    name,
                    scenario,
                    &expectation.mount,
                    &expectation.path,
                )?;

                if !expectation.exists && expectation.checksum.is_some() {
                    return Err(eyre!(
                        "container case {:?} expectation {:?} cannot set checksum when exists=false",
                        self.id(),
                        name
                    ));
                }
            }
        }

        for (name, expectation) in &self.history {
            validate_case_path_ref(
                self.id(),
                name,
                scenario,
                &expectation.mount,
                &expectation.path,
            )?;
        }

        for step in &self.steps {
            if let Some(expectations) = &step.before {
                validate_named_expectations(self, step, expectations)?;
            }

            if let Some(expectations) = &step.expectations {
                validate_named_expectations(self, step, expectations)?;
            }

            if let Some(history) = &step.history
                && !self.history.contains_key(history)
            {
                return Err(eyre!(
                    "container case {:?} step {:?} references missing history expectation {:?}",
                    self.id(),
                    step.name,
                    history
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct ContainerCaseData {
    description: String,
    scenario: String,
    expectations: BTreeMap<String, Vec<FilesystemExpectation>>,
    history: Option<BTreeMap<String, HistoryExpectation>>,
    steps: Vec<CaseStep>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct FilesystemExpectation {
    mount: String,
    path: String,
    #[serde(default = "default_exists")]
    exists: bool,
    checksum: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct HistoryExpectation {
    mount: String,
    path: String,
    record: usize,
    #[serde(default)]
    contains_actions: Vec<ActionName>,
    #[serde(default)]
    does_not_contain_actions: Vec<ActionName>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct CaseStep {
    name: String,
    command: Option<Vec<String>>,
    #[serde(default = "default_exit_code")]
    exit_code: i32,
    expectations: Option<String>,
    before: Option<String>,
    history: Option<String>,
}

fn default_exists() -> bool {
    true
}

fn default_exit_code() -> i32 {
    0
}

#[derive(Debug)]
pub struct ExecutedCase {
    pub outcome: CaseOutcome,
    pub volume_names: Vec<String>,
    pub cleanup_commands: Vec<String>,
}

pub struct CaseRunContext<'a> {
    pub runtime: &'a ContainerRuntime,
    pub image: &'a str,
    pub timeout_seconds: u64,
    pub preserve: bool,
    pub run_id: &'a str,
    pub workspace_root: &'a Utf8Path,
}

pub fn case_id_from_path(path: &Utf8Path) -> Result<String> {
    let file_name = path
        .file_name()
        .ok_or_eyre("container case path must include a file name")?;

    file_name
        .strip_suffix(CASE_SUFFIX)
        .map(str::to_owned)
        .ok_or_else(|| eyre!("container case file must end with {CASE_SUFFIX}"))
}

pub fn run_case(
    case: &ContainerCase,
    ctx: &CaseRunContext<'_>,
) -> ExecutedCase {
    let case_started = Instant::now();
    let scenario = match ContainerScenario::from_file(&scenario_path(case)) {
        Ok(scenario) => scenario,
        Err(error) => {
            return ExecutedCase {
                outcome: CaseOutcome::new(
                    case.id().to_owned(),
                    case.description().to_owned(),
                    case_started.elapsed().as_millis(),
                    vec![StepOutcome::failed(
                        SETUP_STEP_NAME.to_owned(),
                        error.to_string(),
                    )],
                    None,
                ),
                volume_names: Vec::new(),
                cleanup_commands: Vec::new(),
            };
        },
    };

    if let Err(error) = case.validate(&scenario) {
        return ExecutedCase {
            outcome: CaseOutcome::new(
                case.id().to_owned(),
                case.description().to_owned(),
                case_started.elapsed().as_millis(),
                vec![StepOutcome::failed(
                    SETUP_STEP_NAME.to_owned(),
                    error.to_string(),
                )],
                None,
            ),
            volume_names: Vec::new(),
            cleanup_commands: Vec::new(),
        };
    }

    let mut resources = CaseResources::new(case.id(), ctx.run_id);
    let mut artifacts = match CaseArtifacts::new(ctx.workspace_root, case.id()) {
        Ok(artifacts) => artifacts,
        Err(error) => {
            return ExecutedCase {
                outcome: CaseOutcome::new(
                    case.id().to_owned(),
                    case.description().to_owned(),
                    case_started.elapsed().as_millis(),
                    vec![StepOutcome::failed(
                        SETUP_STEP_NAME.to_owned(),
                        error.to_string(),
                    )],
                    None,
                ),
                volume_names: Vec::new(),
                cleanup_commands: Vec::new(),
            };
        },
    };

    let setup_step =
        match create_case_volumes(ctx.runtime, &scenario, &mut resources) {
            Ok(()) => run_setup(case, &scenario, ctx, &resources, &mut artifacts)
                .unwrap_or_else(|error| {
                    StepOutcome::failed(SETUP_STEP_NAME.to_owned(), error.to_string())
                }),
            Err(error) => {
                StepOutcome::failed(SETUP_STEP_NAME.to_owned(), error.to_string())
            },
        };

    let mut steps = vec![setup_step];
    if steps.last().is_some_and(StepOutcome::passed) {
        run_case_steps(
            case,
            &scenario,
            ctx,
            &resources,
            &mut artifacts,
            &mut steps,
        );
    } else {
        for step in &case.steps {
            steps.push(StepOutcome::skipped(
                step.name.clone(),
                "previous_step_failed",
            ));
        }
    }

    let mut exported_artifacts = Vec::new();
    let case_outcome = CaseOutcome::new(
        case.id().to_owned(),
        format!("{} [{}]", case.description(), scenario.description()),
        case_started.elapsed().as_millis(),
        steps,
        None,
    );

    if !case_outcome.passed() {
        if let Ok(exports) =
            export_failure_artifacts(&scenario, ctx, &resources, &mut artifacts)
        {
            exported_artifacts = exports;
        }
    }

    let commands_artifact = artifacts
        .write_json("commands.json", &artifacts.command_records)
        .ok();
    let diagnostics_artifact = artifacts
        .write_json(
            "diagnostics.json",
            &build_case_diagnostics(
                case,
                &scenario,
                ctx,
                &resources,
                &artifacts,
                exported_artifacts.clone(),
            ),
        )
        .ok();

    let outcome = attach_case_artifacts(
        case_outcome,
        commands_artifact,
        diagnostics_artifact,
        exported_artifacts,
        artifacts.setup_created_paths.clone(),
    );

    let cleanup_commands = resources.cleanup_commands(ctx.runtime.command());

    if !ctx.preserve {
        for volume_name in resources.volume_names().iter().rev() {
            let _ = ctx.runtime.remove_volume(volume_name);
        }
        for host_dir in resources.host_dirs().iter().rev() {
            let _ = fs_err::remove_dir_all(host_dir);
        }
    }

    ExecutedCase {
        outcome,
        volume_names: resources.volume_names(),
        cleanup_commands,
    }
}

fn attach_case_artifacts(
    mut outcome: CaseOutcome,
    commands_artifact: Option<String>,
    diagnostics_artifact: Option<String>,
    exported_artifacts: Vec<String>,
    setup_created_paths: Vec<String>,
) -> CaseOutcome {
    let Some(index) = failing_step_index(&outcome) else {
        if let Some(setup_step) = outcome.steps.get_mut(0) {
            let mut details = ContainerStepDetails::new()
                .with_setup_created_paths(setup_created_paths);
            if let Some(path) = commands_artifact {
                details = details.with_commands_artifact(path);
            }
            if let Some(path) = diagnostics_artifact {
                details = details.with_diagnostics_artifact(path);
            }
            *setup_step = setup_step.clone().with_container_details(details);
        }
        return outcome;
    };

    if let Some(step) = outcome.steps.get_mut(index) {
        let mut details = step
            .clone()
            .container
            .clone()
            .unwrap_or_else(ContainerStepDetails::new)
            .with_exported_artifacts(exported_artifacts);
        if let Some(path) = commands_artifact {
            details = details.with_commands_artifact(path);
        }
        if let Some(path) = diagnostics_artifact {
            details = details.with_diagnostics_artifact(path);
        }
        if index == 0 {
            details = details.with_setup_created_paths(setup_created_paths);
        }
        *step = step.clone().with_container_details(details);
    }

    outcome
}

fn failing_step_index(outcome: &CaseOutcome) -> Option<usize> {
    outcome.steps.iter().position(|step| !step.passed() && step.status != tfmttools_test_harness::Status::Skipped)
}

fn scenario_path(case: &ContainerCase) -> Utf8PathBuf {
    ContainerScenario::scenario_path(case)
}

fn run_case_steps(
    case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &mut CaseArtifacts,
    outcomes: &mut Vec<StepOutcome>,
) {
    let mut previous_step_failed = false;

    for step in &case.steps {
        if previous_step_failed {
            outcomes.push(StepOutcome::skipped(
                step.name.clone(),
                "previous_step_failed",
            ));
            continue;
        }

        let outcome = run_case_step(case, scenario, ctx, resources, artifacts, step)
            .unwrap_or_else(|error| {
                StepOutcome::failed(step.name.clone(), error.to_string())
            });
        previous_step_failed = !outcome.passed();
        outcomes.push(outcome);
    }
}

fn run_case_step(
    case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &mut CaseArtifacts,
    step: &CaseStep,
) -> Result<StepOutcome> {
    let step_started = Instant::now();

    if let Some(before) = &step.before {
        let verification = verify_step_expectations(
            case,
            scenario,
            ctx,
            resources,
            artifacts,
            before,
            None,
            &step.name,
        )?;

        if !verification.expectations.passed() {
            return Ok(
                StepOutcome::new(
                    step.name.clone(),
                    step_started.elapsed().as_millis(),
                    None,
                    verification.expectations,
                )
                .with_container_details(verification.details),
            );
        }
    }

    let command_outcome = if let Some(command) = &step.command {
        Some(run_app_container(
            scenario,
            ctx,
            resources,
            artifacts,
            &step.name,
            command,
            step.exit_code,
        )?)
    } else {
        None
    };

    let verification = if let Some(expectations) = &step.expectations {
        verify_step_expectations(
            case,
            scenario,
            ctx,
            resources,
            artifacts,
            expectations,
            step.history.as_deref(),
            &step.name,
        )?
    } else if let Some(history) = &step.history {
        verify_step_expectations(
            case,
            scenario,
            ctx,
            resources,
            artifacts,
            "",
            Some(history),
            &step.name,
        )?
    } else {
        VerificationResult {
            expectations: ExpectationsOutcome::new(None, Vec::new()),
            details: ContainerStepDetails::new(),
        }
    };

    let step_outcome = StepOutcome::new(
        step.name.clone(),
        step_started.elapsed().as_millis(),
        command_outcome.clone(),
        verification.expectations,
    )
    .with_container_details(verification.details);

    if command_outcome
        .as_ref()
        .is_some_and(|outcome| outcome.status == tfmttools_test_harness::Status::TimedOut)
    {
        return Ok(StepOutcome::timed_out(
            step.name.clone(),
            step_started.elapsed().as_millis(),
            command_outcome,
        ));
    }

    Ok(step_outcome)
}

fn run_app_container(
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &mut CaseArtifacts,
    step_name: &str,
    command: &[String],
    expected_exit_code: i32,
) -> Result<CommandOutcome> {
    let container_name = resources.container_name("app", step_name);
    let mut args = container_run_args(
        scenario,
        resources,
        APP_UID_GID,
        scenario.workdir(),
        &container_name,
    )?;

    let command = command_with_config_directory(scenario, command);
    args.push(ctx.image.to_owned());
    args.extend(command);

    let result = ctx.runtime.run_container_with_timeout(
        &args,
        &container_name,
        ctx.timeout_seconds,
    )?;
    artifacts.command_records.push(RecordedCommand::from_result(
        "app",
        step_name,
        &container_name,
        ctx.runtime.command(),
        &result,
    ));

    Ok(command_outcome_from_result(
        ctx.runtime.command(),
        &result,
        expected_exit_code,
        format!("container command timed out after {} seconds", ctx.timeout_seconds),
    ))
}

fn command_with_config_directory(
    scenario: &ContainerScenario,
    command: &[String],
) -> Vec<String> {
    if scenario.mounts().contains_key("config")
        && !command.iter().any(|arg| arg == "--config-directory" || arg == "-c")
    {
        let mut with_config =
            vec!["--config-directory".to_owned(), "/work/config".to_owned()];
        with_config.extend(command.iter().cloned());
        with_config
    } else {
        command.to_vec()
    }
}

fn verify_step_expectations(
    case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &mut CaseArtifacts,
    expectations_name: &str,
    history_name: Option<&str>,
    step_name: &str,
) -> Result<VerificationResult> {
    let filesystem = if expectations_name.is_empty() {
        Vec::new()
    } else {
        case.expectations
            .get(expectations_name)
            .ok_or_else(|| {
                eyre!(
                    "container case {:?} step {:?} references missing expectations {:?}",
                    case.id(),
                    step_name,
                    expectations_name
                )
            })?
            .iter()
            .map(|expectation| VerifyFilesystemExpectation {
                mount: expectation.mount.clone(),
                path: expectation.path.clone(),
                exists: expectation.exists,
                checksum: expectation.checksum.clone(),
            })
            .collect()
    };

    let history = history_name
        .map(|name| {
            case.history
                .get(name)
                .ok_or_else(|| {
                    eyre!(
                        "container case {:?} step {:?} references missing history expectation {:?}",
                        case.id(),
                        step_name,
                        name
                    )
                })
                .map(|expectation| VerifyHistoryExpectation {
                    mount: expectation.mount.clone(),
                    path: expectation.path.clone(),
                    record: expectation.record,
                    contains_actions: expectation.contains_actions.clone(),
                    does_not_contain_actions: expectation
                        .does_not_contain_actions
                        .clone(),
                })
        })
        .transpose()?
        .into_iter()
        .collect();

    let request = VerifyRequest {
        mounts: scenario_mounts(scenario),
        filesystem,
        history,
    };

    let request_artifact =
        artifacts.write_json(&format!("verify-request-{step_name}.json"), &request)?;
    let (response, verify_command) = run_verifier(
        ctx,
        scenario,
        resources,
        artifacts,
        step_name,
        &request,
    )?;
    let response_artifact =
        artifacts.write_json(&format!("verify-response-{step_name}.json"), &response)?;

    if let Some(command) = verify_command {
        artifacts.command_records.push(command);
    }

    let outcomes = response
        .outcomes
        .into_iter()
        .map(verify_outcome_to_report_outcome)
        .collect();

    Ok(VerificationResult {
        expectations: ExpectationsOutcome::new(None, outcomes),
        details: ContainerStepDetails::new()
            .with_verify_request_artifact(request_artifact)
            .with_verify_response_artifact(response_artifact),
    })
}

fn run_verifier(
    ctx: &CaseRunContext<'_>,
    scenario: &ContainerScenario,
    resources: &CaseResources,
    artifacts: &CaseArtifacts,
    step_name: &str,
    request: &VerifyRequest,
) -> Result<(VerifyResponse, Option<RecordedCommand>)> {
    let (request_dir, request_rel) =
        artifacts.create_temp_dir(&format!("verify-{step_name}"))?;
    let request_path = request_dir.join("request.json");
    fs_err::write(&request_path, serde_json::to_vec_pretty(request)?)?;

    let container_name = resources.container_name("verify", step_name);
    let mut args = container_run_args(
        scenario,
        resources,
        APP_UID_GID,
        scenario.workdir(),
        &container_name,
    )?;
    args.extend([
        "--entrypoint".to_owned(),
        "/usr/local/bin/tfmt-container-verify".to_owned(),
        "--mount".to_owned(),
        format!("type=bind,src={request_dir},dst={VERIFY_MOUNT_PATH},readonly"),
    ]);
    args.push(ctx.image.to_owned());
    args.extend([
        format!("{VERIFY_MOUNT_PATH}/request.json"),
    ]);

    let result = ctx.runtime.run_container_with_timeout(
        &args,
        &container_name,
        ctx.timeout_seconds,
    )?;
    let command = Some(RecordedCommand::from_result(
        "verifier",
        step_name,
        &container_name,
        ctx.runtime.command(),
        &result,
    ));

    let response = if result.timed_out {
        VerifyResponse {
            mount_aliases: request.mounts.clone(),
            outcomes: vec![VerifyOutcome::failure(
                VerifyCode::VerifierError,
                None,
                format!("verifier timed out after {} seconds", ctx.timeout_seconds),
            )],
        }
    } else {
        let stdout = String::from_utf8_lossy(&result.output.stdout);
        serde_json::from_str(stdout.trim()).unwrap_or_else(|error| VerifyResponse {
            mount_aliases: request.mounts.clone(),
            outcomes: vec![VerifyOutcome::failure(
                VerifyCode::VerifierError,
                None,
                format!(
                    "failed to parse verifier response: {error}; request dir {request_rel}"
                ),
            )],
        })
    };

    Ok((response, command))
}

fn scenario_mounts(scenario: &ContainerScenario) -> BTreeMap<String, String> {
    scenario
        .mounts()
        .iter()
        .map(|(alias, mount)| (alias.clone(), mount.container_path().to_owned()))
        .collect()
}

fn verify_outcome_to_report_outcome(outcome: VerifyOutcome) -> ExpectationOutcome {
    if outcome.passed() {
        return outcome
            .path
            .map_or_else(
                || ExpectationOutcome::Ok(Utf8PathBuf::from(outcome.message)),
                ExpectationOutcome::Ok,
            );
    }

    match outcome.code {
        VerifyCode::PathMissing => outcome
            .path
            .map_or_else(
                || ExpectationOutcome::VerificationFailure {
                    code: "path_missing".to_owned(),
                    path: None,
                    message: outcome.message,
                },
                ExpectationOutcome::NotPresent,
            ),
        VerifyCode::PathUnexpected => outcome
            .path
            .map_or_else(
                || ExpectationOutcome::VerificationFailure {
                    code: "path_unexpected".to_owned(),
                    path: None,
                    message: outcome.message,
                },
                ExpectationOutcome::UnexpectedPresent,
            ),
        VerifyCode::ChecksumMismatch => {
            if let Some(path) = outcome.path {
                ExpectationOutcome::ChecksumMismatch {
                    path,
                    expected: outcome.expected_checksum.unwrap_or_default(),
                    actual: outcome.actual_checksum.unwrap_or_default(),
                }
            } else {
                ExpectationOutcome::VerificationFailure {
                    code: "checksum_mismatch".to_owned(),
                    path: None,
                    message: outcome.message,
                }
            }
        },
        other => ExpectationOutcome::VerificationFailure {
            code: format!("{other:?}").to_ascii_lowercase(),
            path: outcome.path,
            message: outcome.message,
        },
    }
}

fn create_case_volumes(
    runtime: &ContainerRuntime,
    scenario: &ContainerScenario,
    resources: &mut CaseResources,
) -> Result<()> {
    for (alias, mount) in scenario.mounts() {
        let volume_name = resources.push(alias);
        runtime.create_volume(volume_name, mount)?;
        if let Some(host_dir) = mount.host_bind_dir(volume_name) {
            resources.push_host_dir(host_dir);
        }
    }

    Ok(())
}

fn run_setup(
    case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &mut CaseArtifacts,
) -> Result<StepOutcome> {
    let setup_started = Instant::now();

    for (index, operation) in scenario.setup().iter().enumerate() {
        let result =
            run_setup_operation(case, scenario, ctx, resources, operation, index, artifacts)?;
        if result.timed_out || !result.output.status.success() {
            let step = if result.timed_out {
                StepOutcome::timed_out(
                    SETUP_STEP_NAME.to_owned(),
                    result.duration_ms,
                    Some(command_outcome_from_result(
                        ctx.runtime.command(),
                        &result,
                        0,
                        format!("container setup timed out after {} seconds", ctx.timeout_seconds),
                    )),
                )
            } else {
                StepOutcome::new(
                    SETUP_STEP_NAME.to_owned(),
                    result.duration_ms,
                    Some(command_outcome_from_result(
                        ctx.runtime.command(),
                        &result,
                        0,
                        format!("container setup timed out after {} seconds", ctx.timeout_seconds),
                    )),
                    failed_expectations_outcome(),
                )
            };
            return Ok(
                step.with_container_details(
                    ContainerStepDetails::new()
                        .with_setup_created_paths(artifacts.setup_created_paths.clone()),
                ),
            );
        }
    }

    let normalize_result =
        normalize_setup_paths(scenario, ctx, resources, artifacts)?;
    if normalize_result.timed_out || !normalize_result.output.status.success() {
        let step = if normalize_result.timed_out {
            StepOutcome::timed_out(
                SETUP_STEP_NAME.to_owned(),
                normalize_result.duration_ms,
                Some(command_outcome_from_result(
                    ctx.runtime.command(),
                    &normalize_result,
                    0,
                    format!(
                        "container ownership normalization timed out after {} seconds",
                        ctx.timeout_seconds
                    ),
                )),
            )
        } else {
            StepOutcome::new(
                SETUP_STEP_NAME.to_owned(),
                normalize_result.duration_ms,
                Some(command_outcome_from_result(
                    ctx.runtime.command(),
                    &normalize_result,
                    0,
                    format!(
                        "container ownership normalization timed out after {} seconds",
                        ctx.timeout_seconds
                    ),
                )),
                failed_expectations_outcome(),
            )
        };
        return Ok(
            step.with_container_details(
                ContainerStepDetails::new()
                    .with_setup_created_paths(artifacts.setup_created_paths.clone()),
            ),
        );
    }

    let precondition_result =
        run_preconditions(case, scenario, ctx, resources, artifacts)?;
    if let Some(result) = precondition_result {
        let step = if result.timed_out {
            StepOutcome::timed_out(
                SETUP_STEP_NAME.to_owned(),
                result.duration_ms,
                Some(command_outcome_from_result(
                    ctx.runtime.command(),
                    &result,
                    0,
                    format!("container precondition timed out after {} seconds", ctx.timeout_seconds),
                )),
            )
        } else if !result.output.status.success() {
            StepOutcome::new(
                SETUP_STEP_NAME.to_owned(),
                result.duration_ms,
                Some(command_outcome_from_result(
                    ctx.runtime.command(),
                    &result,
                    0,
                    format!("container precondition timed out after {} seconds", ctx.timeout_seconds),
                )),
                failed_expectations_outcome(),
            )
        } else {
            StepOutcome::new(
                SETUP_STEP_NAME.to_owned(),
                setup_started.elapsed().as_millis(),
                None,
                ExpectationsOutcome::new(None, Vec::new()),
            )
        };
        return Ok(
            step.with_container_details(
                ContainerStepDetails::new()
                    .with_setup_created_paths(artifacts.setup_created_paths.clone()),
            ),
        );
    }

    Ok(
        StepOutcome::new(
            SETUP_STEP_NAME.to_owned(),
            setup_started.elapsed().as_millis(),
            None,
            ExpectationsOutcome::new(None, Vec::new()),
        )
        .with_container_details(
            ContainerStepDetails::new()
                .with_setup_created_paths(artifacts.setup_created_paths.clone()),
        ),
    )
}

fn run_setup_operation(
    _case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    operation: &SetupOperation,
    index: usize,
    artifacts: &mut CaseArtifacts,
) -> Result<RuntimeCommandResult> {
    let fixture_root = container_fixture_root(ctx.workspace_root)?;
    let container_name =
        resources.container_name("setup", &format!("op-{index}"));
    let mut args = container_run_args(
        scenario,
        resources,
        ROOT_UID_GID,
        scenario.workdir(),
        &container_name,
    )?;
    args.extend([
        "--entrypoint".to_owned(),
        "/bin/sh".to_owned(),
        "--mount".to_owned(),
        format!("type=bind,src={fixture_root},dst={FIXTURE_MOUNT_PATH},readonly"),
    ]);
    args.push(ctx.image.to_owned());
    args.extend([
        "-lc".to_owned(),
        setup_operation_script(_case, scenario, operation, artifacts)?,
    ]);

    let result = ctx.runtime.run_container_with_timeout(
        &args,
        &container_name,
        ctx.timeout_seconds,
    )?;
    artifacts.command_records.push(RecordedCommand::from_result(
        "setup",
        SETUP_STEP_NAME,
        &container_name,
        ctx.runtime.command(),
        &result,
    ));
    Ok(result)
}

fn setup_operation_script(
    _case: &ContainerCase,
    scenario: &ContainerScenario,
    operation: &SetupOperation,
    artifacts: &mut CaseArtifacts,
) -> Result<String> {
    match operation {
        SetupOperation::Mkdir { path } => {
            let target = resolve_path_ref(scenario, path)?;
            artifacts.setup_created_paths.push(target.clone());
            Ok(format!("set -eu; mkdir -p {}", shell_quote(&target)))
        },
        SetupOperation::CopyFixtureDir { from, to } => {
            let target = resolve_path_ref(scenario, to)?;
            artifacts.setup_created_paths.push(target.clone());
            Ok(format!(
                "set -eu; mkdir -p {}; cp -a {}/{}/. {}",
                shell_quote(&target),
                shell_quote(FIXTURE_MOUNT_PATH),
                shell_quote(from),
                shell_quote(&target)
            ))
        },
    }
}

fn normalize_setup_paths(
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &mut CaseArtifacts,
) -> Result<RuntimeCommandResult> {
    let container_name = resources.container_name("setup", "normalize");
    let mut args = container_run_args(
        scenario,
        resources,
        ROOT_UID_GID,
        scenario.workdir(),
        &container_name,
    )?;
    let targets = scenario
        .mounts()
        .values()
        .map(|mount| shell_quote(mount.container_path()))
        .collect::<Vec<_>>()
        .join(" ");
    args.extend([
        "--entrypoint".to_owned(),
        "/bin/sh".to_owned(),
    ]);
    args.push(ctx.image.to_owned());
    args.extend([
        "-lc".to_owned(),
        format!("set -eu; chown -R {APP_UID_GID} {targets}"),
    ]);

    let result = ctx.runtime.run_container_with_timeout(
        &args,
        &container_name,
        ctx.timeout_seconds,
    )?;
    artifacts.command_records.push(RecordedCommand::from_result(
        "setup",
        SETUP_STEP_NAME,
        &container_name,
        ctx.runtime.command(),
        &result,
    ));
    Ok(result)
}

fn run_preconditions(
    case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &mut CaseArtifacts,
) -> Result<Option<RuntimeCommandResult>> {
    if scenario.preconditions().is_empty() {
        return Ok(None);
    }

    let container_name = resources.container_name("setup", "preconditions");
    let mut args = container_run_args(
        scenario,
        resources,
        ROOT_UID_GID,
        scenario.workdir(),
        &container_name,
    )?;
    args.extend([
        "--entrypoint".to_owned(),
        "/bin/sh".to_owned(),
    ]);
    args.push(ctx.image.to_owned());
    args.extend(["-lc".to_owned(), build_precondition_script(case, scenario)?]);

    let result = ctx.runtime.run_container_with_timeout(
        &args,
        &container_name,
        ctx.timeout_seconds,
    )?;
    artifacts.command_records.push(RecordedCommand::from_result(
        "setup",
        SETUP_STEP_NAME,
        &container_name,
        ctx.runtime.command(),
        &result,
    ));
    Ok(Some(result))
}

fn build_precondition_script(
    case: &ContainerCase,
    scenario: &ContainerScenario,
) -> Result<String> {
    let mut commands = vec!["set -eu".to_owned()];
    for precondition in scenario.preconditions() {
        match precondition {
            Precondition::DifferentDevices { left, right } => {
                let left_path = scenario
                    .mounts()
                    .get(left)
                    .map(ScenarioMount::container_path)
                    .ok_or_else(|| {
                        eyre!(
                            "container scenario {:?} references unknown mount {:?}",
                            scenario.id(),
                            left
                        )
                    })?;
                let right_path = scenario
                    .mounts()
                    .get(right)
                    .map(ScenarioMount::container_path)
                    .ok_or_else(|| {
                        eyre!(
                            "container scenario {:?} references unknown mount {:?}",
                            scenario.id(),
                            right
                        )
                    })?;
                commands.push(format!(
                    "left_dev=$(stat -c %d {}); right_dev=$(stat -c %d {}); \
                     if [ \"$left_dev\" = \"$right_dev\" ]; then \
                     echo \"precondition failed for case {}: mounts {} and {} must be on different devices\" >&2; \
                     exit 1; \
                     fi",
                    shell_quote(left_path),
                    shell_quote(right_path),
                    shell_quote(case.id()),
                    shell_quote(left),
                    shell_quote(right)
                ));
            },
        }
    }
    Ok(commands.join("; "))
}

fn failed_expectations_outcome() -> ExpectationsOutcome {
    ExpectationsOutcome::new(
        None,
        vec![ExpectationOutcome::VerificationFailure {
            code: "container_setup_failed".to_owned(),
            path: None,
            message: "container setup failed".to_owned(),
        }],
    )
}

fn validate_named_expectations(
    case: &ContainerCase,
    step: &CaseStep,
    expectations: &str,
) -> Result<()> {
    if case.expectations.contains_key(expectations) {
        Ok(())
    } else {
        Err(eyre!(
            "container case {:?} step {:?} references missing expectations {:?}",
            case.id(),
            step.name,
            expectations
        ))
    }
}

fn validate_case_path_ref(
    case_id: &str,
    name: &str,
    scenario: &ContainerScenario,
    mount: &str,
    path: &str,
) -> Result<()> {
    if !scenario.mounts().contains_key(mount) {
        return Err(eyre!(
            "container case {case_id:?} expectation {name:?} references unknown mount {mount:?}"
        ));
    }

    let path = Utf8Path::new(path);
    if path.is_absolute() || path.components().any(|part| part.as_str() == "..")
    {
        return Err(eyre!(
            "container case {case_id:?} expectation {name:?} has invalid path {path:?}"
        ));
    }

    Ok(())
}

fn container_fixture_root(workspace_root: &Utf8Path) -> Result<Utf8PathBuf> {
    let fixture_root = workspace_root.join("tests/fixtures/container");
    fixture_root.canonicalize_utf8().map_err(Into::into)
}

fn container_run_args(
    scenario: &ContainerScenario,
    resources: &CaseResources,
    user: &str,
    workdir: &str,
    container_name: &str,
) -> Result<Vec<String>> {
    let mut args = vec![
        "run".to_owned(),
        "--rm".to_owned(),
        "--name".to_owned(),
        container_name.to_owned(),
        "--user".to_owned(),
        user.to_owned(),
        "--workdir".to_owned(),
        workdir.to_owned(),
    ];

    for (alias, mount) in scenario.mounts() {
        args.push("--mount".to_owned());
        args.push(volume_mount_arg(resources.volume_name(alias)?, mount));
    }

    Ok(args)
}

fn volume_mount_arg(volume_name: &str, mount: &ScenarioMount) -> String {
    format!("type=volume,src={volume_name},dst={}", mount.container_path())
}

fn resolve_path_ref(
    scenario: &ContainerScenario,
    path_ref: &ScenarioPathRef,
) -> Result<String> {
    let mount = scenario
        .mounts()
        .get(path_ref.mount())
        .ok_or_else(|| eyre!("container scenario missing mount {:?}", path_ref.mount()))?;
    join_container_path(mount.container_path(), path_ref.path())
}

fn join_container_path(base: &str, relative: &str) -> Result<String> {
    if relative.is_empty() {
        return Ok(base.to_owned());
    }

    let relative = Utf8Path::new(relative);
    if relative.is_absolute() {
        return Err(eyre!("container path {:?} must be relative", relative));
    }

    Ok(Utf8Path::new(base).join(relative).to_string())
}

fn runtime_argv(command: &str, args: &[String]) -> Vec<String> {
    std::iter::once(command.to_owned()).chain(args.iter().cloned()).collect()
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn command_outcome_from_result(
    runtime_command: &str,
    result: &RuntimeCommandResult,
    expected_exit_code: i32,
    timed_out_message: String,
) -> CommandOutcome {
    if result.timed_out {
        CommandOutcome::timed_out(
            runtime_argv(runtime_command, &result.arguments),
            String::from_utf8_lossy(&result.output.stdout).trim().to_owned(),
            format!(
                "{timed_out_message}{}{}",
                if result.output.stderr.is_empty() { "" } else { "\nstderr:\n" },
                String::from_utf8_lossy(&result.output.stderr).trim()
            ),
        )
        .with_duration_ms(result.duration_ms)
    } else {
        CommandOutcome::with_expected_exit_code(
            runtime_argv(runtime_command, &result.arguments),
            &result.output,
            expected_exit_code,
        )
        .with_duration_ms(result.duration_ms)
    }
}

fn export_failure_artifacts(
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &mut CaseArtifacts,
) -> Result<Vec<String>> {
    let mut exported = Vec::new();

    for (alias, _mount) in scenario.mounts() {
        let volume_name = resources.volume_name(alias)?;
        let (export_dir, export_rel) = artifacts.ensure_dir(alias)?;
        let container_name = resources.container_name("export", alias);
        let args = vec![
            "run".to_owned(),
            "--rm".to_owned(),
            "--name".to_owned(),
            container_name.clone(),
            "--user".to_owned(),
            ROOT_UID_GID.to_owned(),
            "--mount".to_owned(),
            format!("type=volume,src={volume_name},dst=/from,readonly"),
            "--mount".to_owned(),
            format!("type=bind,src={export_dir},dst=/to"),
            "--entrypoint".to_owned(),
            "/bin/sh".to_owned(),
            ctx.image.to_owned(),
            "-lc".to_owned(),
            "set -eu; if find /from -mindepth 1 -print -quit | grep -q .; then cp -a /from/. /to/; else : > /to/.empty-volume; fi".to_owned(),
        ];
        let result = ctx.runtime.run_container_with_timeout(
            &args,
            &container_name,
            ctx.timeout_seconds,
        )?;
        artifacts.command_records.push(RecordedCommand::from_result(
            "export",
            alias,
            &container_name,
            ctx.runtime.command(),
            &result,
        ));
        if result.output.status.success() && !result.timed_out {
            exported.push(export_rel);
        }
    }

    let container_name = resources.container_name("diagnostics", "mountinfo");
    let mut args = container_run_args(
        scenario,
        resources,
        APP_UID_GID,
        scenario.workdir(),
        &container_name,
    )?;
    args.extend([
        "--entrypoint".to_owned(),
        "/bin/sh".to_owned(),
    ]);
    args.push(ctx.image.to_owned());
    args.extend(["-lc".to_owned(), "cat /proc/self/mountinfo".to_owned()]);
    let result = ctx.runtime.run_container_with_timeout(
        &args,
        &container_name,
        ctx.timeout_seconds,
    )?;
    artifacts.mountinfo = Some(String::from_utf8_lossy(&result.output.stdout).to_string());
    artifacts.command_records.push(RecordedCommand::from_result(
        "diagnostics",
        "mountinfo",
        &container_name,
        ctx.runtime.command(),
        &result,
    ));

    Ok(exported)
}

fn build_case_diagnostics(
    case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    artifacts: &CaseArtifacts,
    exported_artifacts: Vec<String>,
) -> CaseDiagnostics {
    CaseDiagnostics {
        case_id: case.id().to_owned(),
        scenario_id: scenario.id().to_owned(),
        runtime: ctx.runtime.command().to_owned(),
        image: ctx.image.to_owned(),
        volume_names: resources.volume_names(),
        mount_aliases: scenario_mounts(scenario),
        setup_created_paths: artifacts.setup_created_paths.clone(),
        mountinfo: artifacts.mountinfo.clone(),
        exported_artifacts,
    }
}

#[derive(Debug)]
struct CaseResources {
    prefix: String,
    case_slug: String,
    random_suffix: String,
    volumes: Vec<(String, String)>,
    host_dirs: Vec<Utf8PathBuf>,
}

impl CaseResources {
    fn new(case_id: &str, run_id: &str) -> Self {
        Self {
            prefix: format!("tfmttools-{}", sanitize_volume_component(run_id, 24)),
            case_slug: sanitize_volume_component(case_id, 24),
            random_suffix: random_suffix(),
            volumes: Vec::new(),
            host_dirs: Vec::new(),
        }
    }

    fn push(&mut self, alias: &str) -> &str {
        let volume_name = format!(
            "{}-{}-{}-{}",
            self.prefix,
            self.case_slug,
            sanitize_volume_component(alias, 16),
            self.random_suffix
        );
        self.volumes.push((alias.to_owned(), volume_name));
        self.volumes.last().map(|(_, name)| name.as_str()).expect("just pushed")
    }

    fn volume_name(&self, alias: &str) -> Result<&str> {
        self.volumes
            .iter()
            .find_map(|(candidate, name)| (candidate == alias).then_some(name.as_str()))
            .ok_or_else(|| eyre!("container case is missing volume for mount {alias:?}"))
    }

    fn volume_names(&self) -> Vec<String> {
        self.volumes.iter().map(|(_, name)| name.clone()).collect()
    }

    fn push_host_dir(&mut self, path: Utf8PathBuf) {
        self.host_dirs.push(path);
    }

    fn host_dirs(&self) -> &[Utf8PathBuf] {
        &self.host_dirs
    }

    fn cleanup_commands(&self, runtime_command: &str) -> Vec<String> {
        self.volumes
            .iter()
            .map(|(_, volume_name)| format!("{runtime_command} volume rm -f {volume_name}"))
            .collect()
    }

    fn container_name(&self, phase: &str, step: &str) -> String {
        format!(
            "{}-{}-{}-{}",
            self.prefix,
            self.case_slug,
            sanitize_volume_component(phase, 12),
            sanitize_volume_component(step, 16)
        )
    }
}

#[derive(Debug)]
struct CaseArtifacts {
    report_dir: Utf8PathBuf,
    case_dir: Utf8PathBuf,
    case_rel: String,
    command_records: Vec<RecordedCommand>,
    setup_created_paths: Vec<String>,
    mountinfo: Option<String>,
}

impl CaseArtifacts {
    fn new(workspace_root: &Utf8Path, case_id: &str) -> Result<Self> {
        let report_dir = workspace_root.join("tests/reports/container");
        let case_rel = format!(
            "artifacts/{}-{}",
            sanitize_volume_component(case_id, 40),
            random_suffix()
        );
        let case_dir = report_dir.join(&case_rel);
        fs_err::create_dir_all(&case_dir)?;

        Ok(Self {
            report_dir,
            case_dir,
            case_rel,
            command_records: Vec::new(),
            setup_created_paths: Vec::new(),
            mountinfo: None,
        })
    }

    fn write_json<T: Serialize>(&self, file_name: &str, value: &T) -> Result<String> {
        let path = self.case_dir.join(file_name);
        fs_err::write(&path, serde_json::to_vec_pretty(value)?)?;
        Ok(format!("{}/{}", self.case_rel, file_name))
    }

    fn create_temp_dir(&self, name: &str) -> Result<(Utf8PathBuf, String)> {
        let dir = self.case_dir.join(format!(
            "{}-{}",
            sanitize_volume_component(name, 24),
            random_suffix()
        ));
        fs_err::create_dir_all(&dir)?;
        let rel = dir
            .strip_prefix(&self.report_dir)
            .map_err(|_| eyre!("failed to relativize artifact path"))?
            .to_string();
        Ok((dir, rel))
    }

    fn ensure_dir(&self, name: &str) -> Result<(Utf8PathBuf, String)> {
        let dir = self.case_dir.join(name);
        fs_err::create_dir_all(&dir)?;
        Ok((dir.clone(), format!("{}/{}", self.case_rel, name)))
    }
}

#[derive(Debug, Clone, Serialize)]
struct RecordedCommand {
    phase: String,
    step: String,
    container_name: String,
    arguments: Vec<String>,
    exit_code: Option<i32>,
    timed_out: bool,
    duration_ms: u128,
    stdout: String,
    stderr: String,
}

impl RecordedCommand {
    fn from_result(
        phase: &str,
        step: &str,
        container_name: &str,
        runtime_command: &str,
        result: &RuntimeCommandResult,
    ) -> Self {
        Self {
            phase: phase.to_owned(),
            step: step.to_owned(),
            container_name: container_name.to_owned(),
            arguments: runtime_argv(runtime_command, &result.arguments),
            exit_code: result.output.status.code(),
            timed_out: result.timed_out,
            duration_ms: result.duration_ms,
            stdout: String::from_utf8_lossy(&result.output.stdout).trim().to_owned(),
            stderr: String::from_utf8_lossy(&result.output.stderr).trim().to_owned(),
        }
    }
}

#[derive(Debug, Serialize)]
struct CaseDiagnostics {
    case_id: String,
    scenario_id: String,
    runtime: String,
    image: String,
    volume_names: Vec<String>,
    mount_aliases: BTreeMap<String, String>,
    setup_created_paths: Vec<String>,
    mountinfo: Option<String>,
    exported_artifacts: Vec<String>,
}

struct VerificationResult {
    expectations: ExpectationsOutcome,
    details: ContainerStepDetails,
}

fn sanitize_volume_component(value: &str, max_len: usize) -> String {
    let sanitized = value
        .chars()
        .map(|char| {
            if char.is_ascii_lowercase() || char.is_ascii_digit() {
                char
            } else if char.is_ascii_uppercase() {
                char.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();

    sanitized.trim_matches('-').chars().take(max_len).collect::<String>()
}

fn random_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    format!("{:08x}", (nanos & 0xffff_ffff) as u64)
}

#[cfg(test)]
mod tests {
    use camino::Utf8Path;

    use super::{case_id_from_path, sanitize_volume_component};

    #[test]
    fn strips_case_suffix_for_case_id() {
        let id =
            case_id_from_path(Utf8Path::new("cases/cross-device.case.json"))
                .expect("case id should parse");

        assert_eq!(id, "cross-device");
    }

    #[test]
    fn rejects_non_case_json_files() {
        let error = case_id_from_path(Utf8Path::new("cases/not-a-case.json"))
            .expect_err("non case files should fail");

        assert!(
            error
                .to_string()
                .contains("container case file must end with .case.json")
        );
    }

    #[test]
    fn sanitizes_volume_name_components() {
        assert_eq!(
            sanitize_volume_component("Cross_Device-Case", 32),
            "cross-device-case"
        );
    }
}
