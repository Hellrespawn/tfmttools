use std::collections::BTreeMap;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use serde::Deserialize;
use tfmttools_test_harness::{
    CaseOutcome, CommandOutcome, ExpectationOutcome, ExpectationsOutcome,
    StepOutcome,
};

use crate::protocol::{
    ActionName, FilesystemExpectation as VerifyFilesystemExpectation,
    HistoryExpectation as VerifyHistoryExpectation, VerifyOutcome,
    VerifyRequest, VerifyResponse,
};
use crate::runtime::ContainerRuntime;
use crate::scenario::{
    ContainerScenario, Precondition, ScenarioMount, SetupOperation,
};

const CASE_SUFFIX: &str = ".case.json";
const SETUP_STEP_NAME: &str = "setup";
const APP_UID_GID: &str = "1000:1000";
const FIXTURE_MOUNT_PATH: &str = "/fixtures";

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
    let setup_step =
        match create_case_volumes(ctx.runtime, &scenario, &mut resources) {
            Ok(()) => {
                run_setup_container(case, &scenario, ctx, &resources)
                    .unwrap_or_else(|error| {
                        StepOutcome::failed(
                            SETUP_STEP_NAME.to_owned(),
                            error.to_string(),
                        )
                    })
            },
            Err(error) => {
                StepOutcome::failed(
                    SETUP_STEP_NAME.to_owned(),
                    error.to_string(),
                )
            },
        };
    let mut steps = vec![setup_step];

    if steps.last().is_some_and(StepOutcome::passed) {
        run_case_steps(case, &scenario, ctx, &resources, &mut steps);
    } else {
        for step in &case.steps {
            steps.push(StepOutcome::skipped(
                step.name.clone(),
                "previous_step_failed",
            ));
        }
    }

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
        outcome: CaseOutcome::new(
            case.id().to_owned(),
            format!("{} [{}]", case.description(), scenario.description()),
            case_started.elapsed().as_millis(),
            steps,
            None,
        ),
        volume_names: resources.volume_names(),
        cleanup_commands,
    }
}

fn scenario_path(case: &ContainerCase) -> Utf8PathBuf {
    ContainerScenario::scenario_path(case)
}

fn run_case_steps(
    case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
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

        let outcome = run_case_step(case, scenario, ctx, resources, step)
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
    step: &CaseStep,
) -> Result<StepOutcome> {
    let step_started = Instant::now();

    if let Some(before) = &step.before {
        let expectations = verify_step_expectations(
            case, scenario, ctx, resources, before, None, &step.name,
        )?;

        if !expectations.passed() {
            return Ok(StepOutcome::new(
                step.name.clone(),
                step_started.elapsed().as_millis(),
                None,
                expectations,
            ));
        }
    }

    let command_outcome = step
        .command
        .as_ref()
        .map(|command| {
            run_app_container(scenario, ctx, resources, command, step.exit_code)
        })
        .transpose()?;

    let expectations_outcome = if let Some(expectations) = &step.expectations {
        verify_step_expectations(
            case,
            scenario,
            ctx,
            resources,
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
            "",
            Some(history),
            &step.name,
        )?
    } else {
        ExpectationsOutcome::new(None, Vec::new())
    };

    Ok(StepOutcome::new(
        step.name.clone(),
        step_started.elapsed().as_millis(),
        command_outcome,
        expectations_outcome,
    ))
}

fn run_app_container(
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
    command: &[String],
    expected_exit_code: i32,
) -> Result<CommandOutcome> {
    let mut args = vec![
        "run".to_owned(),
        "--rm".to_owned(),
        "--user".to_owned(),
        APP_UID_GID.to_owned(),
        "--workdir".to_owned(),
        scenario.workdir().to_owned(),
    ];

    for (alias, mount) in scenario.mounts() {
        args.push("--mount".to_owned());
        args.push(volume_mount_arg(resources.volume_name(alias)?, mount));
    }

    args.push(ctx.image.to_owned());

    let command = command_with_config_directory(scenario, command);
    args.extend(command);

    let result = ctx.runtime.run_with_timeout(&args, ctx.timeout_seconds)?;

    if result.timed_out {
        Ok(CommandOutcome::timed_out(
            runtime_argv(ctx.runtime.command(), &result.arguments),
            String::from_utf8_lossy(&result.output.stdout).trim().to_owned(),
            format!(
                "container command timed out after {} seconds{}{}",
                ctx.timeout_seconds,
                if result.output.stderr.is_empty() {
                    ""
                } else {
                    "\nstderr:\n"
                },
                String::from_utf8_lossy(&result.output.stderr).trim()
            ),
        ))
    } else {
        Ok(CommandOutcome::with_expected_exit_code(
            runtime_argv(ctx.runtime.command(), &result.arguments),
            &result.output,
            expected_exit_code,
        ))
    }
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
    expectations_name: &str,
    history_name: Option<&str>,
    step_name: &str,
) -> Result<ExpectationsOutcome> {
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
    let response =
        run_verifier(ctx, scenario, resources, case, step_name, &request)?;
    let outcomes = response
        .outcomes
        .into_iter()
        .map(verify_outcome_to_report_outcome)
        .collect();

    Ok(ExpectationsOutcome::new(None, outcomes))
}

fn run_verifier(
    ctx: &CaseRunContext<'_>,
    scenario: &ContainerScenario,
    resources: &CaseResources,
    case: &ContainerCase,
    step_name: &str,
    request: &VerifyRequest,
) -> Result<VerifyResponse> {
    let request_dir = create_verify_request_dir(case.id(), step_name)?;
    let request_path = request_dir.join("request.json");
    fs_err::write(&request_path, serde_json::to_vec_pretty(request)?)?;

    let mut args = vec![
        "run".to_owned(),
        "--rm".to_owned(),
        "--user".to_owned(),
        APP_UID_GID.to_owned(),
        "--workdir".to_owned(),
        scenario.workdir().to_owned(),
        "--entrypoint".to_owned(),
        "/usr/local/bin/tfmt-container-verify".to_owned(),
        "--mount".to_owned(),
        format!("type=bind,src={request_dir},dst=/verify,readonly"),
    ];

    for (alias, mount) in scenario.mounts() {
        args.push("--mount".to_owned());
        args.push(volume_mount_arg(resources.volume_name(alias)?, mount));
    }

    args.push(ctx.image.to_owned());
    args.push("/verify/request.json".to_owned());

    let result = ctx.runtime.run_with_timeout(&args, ctx.timeout_seconds)?;

    if !ctx.preserve {
        let _ = fs_err::remove_dir_all(&request_dir);
    }

    if result.timed_out {
        return Ok(VerifyResponse {
            outcomes: vec![VerifyOutcome::Error {
                message: format!(
                    "verifier timed out after {} seconds",
                    ctx.timeout_seconds
                ),
            }],
        });
    }

    let stdout = String::from_utf8_lossy(&result.output.stdout);
    match serde_json::from_str(stdout.trim()) {
        Ok(response) => Ok(response),
        Err(error) => {
            Ok(VerifyResponse {
                outcomes: vec![VerifyOutcome::Error {
                    message: format!(
                        "failed to parse verifier response: {error}\nstdout:\n{}\nstderr:\n{}",
                        stdout.trim(),
                        String::from_utf8_lossy(&result.output.stderr).trim()
                    ),
                }],
            })
        },
    }
}

fn scenario_mounts(scenario: &ContainerScenario) -> BTreeMap<String, String> {
    scenario
        .mounts()
        .iter()
        .map(|(alias, mount)| {
            (alias.clone(), mount.container_path().to_owned())
        })
        .collect()
}

fn verify_outcome_to_report_outcome(
    outcome: VerifyOutcome,
) -> ExpectationOutcome {
    match outcome {
        VerifyOutcome::Ok { path } => ExpectationOutcome::Ok(path),
        VerifyOutcome::NotPresent { path } => {
            ExpectationOutcome::NotPresent(path)
        },
        VerifyOutcome::UnexpectedPresent { path } => {
            ExpectationOutcome::UnexpectedPresent(path)
        },
        VerifyOutcome::ChecksumMismatch { path, expected, actual } => {
            ExpectationOutcome::ChecksumMismatch { path, expected, actual }
        },
        VerifyOutcome::HistoryRecordMissing { path, message, .. } => {
            ExpectationOutcome::NotPresent(path.join(message))
        },
        VerifyOutcome::HistoryActionMissing {
            path, action, actual, ..
        } => {
            ExpectationOutcome::NotPresent(path.join(format!(
                "missing action {action:?}; actual actions: {actual:?}"
            )))
        },
        VerifyOutcome::HistoryActionUnexpected {
            path, action, actual, ..
        } => {
            ExpectationOutcome::UnexpectedPresent(path.join(format!(
                "unexpected action {action:?}; actual actions: {actual:?}"
            )))
        },
        VerifyOutcome::Error { message } => {
            ExpectationOutcome::NotPresent(Utf8PathBuf::from(message))
        },
    }
}

fn create_verify_request_dir(
    case_id: &str,
    step_name: &str,
) -> Result<Utf8PathBuf> {
    let dir = Utf8PathBuf::from_path_buf(std::env::temp_dir())
        .map_err(|_| eyre!("temporary directory path is not valid UTF-8"))?;
    let path = dir.join(format!(
        "tfmttools-verify-{}-{}-{}",
        sanitize_volume_component(case_id, 24),
        sanitize_volume_component(step_name, 24),
        random_suffix()
    ));

    fs_err::create_dir_all(&path)?;
    Ok(path)
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

fn run_setup_container(
    case: &ContainerCase,
    scenario: &ContainerScenario,
    ctx: &CaseRunContext<'_>,
    resources: &CaseResources,
) -> Result<StepOutcome> {
    let fixture_root = container_fixture_root(ctx.workspace_root)?;
    let script = build_setup_script(case.id(), scenario)?;

    let mut args = vec![
        "run".to_owned(),
        "--rm".to_owned(),
        "--user".to_owned(),
        "0:0".to_owned(),
        "--workdir".to_owned(),
        scenario.workdir().to_owned(),
        "--entrypoint".to_owned(),
        "/bin/sh".to_owned(),
        "--mount".to_owned(),
        format!(
            "type=bind,src={},dst={FIXTURE_MOUNT_PATH},readonly",
            fixture_root
        ),
    ];

    for (alias, mount) in scenario.mounts() {
        args.push("--mount".to_owned());
        args.push(volume_mount_arg(resources.volume_name(alias)?, mount));
    }

    args.push(ctx.image.to_owned());
    args.push("-lc".to_owned());
    args.push(script);

    let result = ctx.runtime.run_with_timeout(&args, ctx.timeout_seconds)?;
    let command_outcome = if result.timed_out {
        CommandOutcome::timed_out(
            runtime_argv(ctx.runtime.command(), &result.arguments),
            String::from_utf8_lossy(&result.output.stdout).trim().to_owned(),
            format!(
                "container setup timed out after {} seconds{}{}",
                ctx.timeout_seconds,
                if result.output.stderr.is_empty() {
                    ""
                } else {
                    "\nstderr:\n"
                },
                String::from_utf8_lossy(&result.output.stderr).trim()
            ),
        )
    } else {
        CommandOutcome::new(
            runtime_argv(ctx.runtime.command(), &result.arguments),
            &result.output,
        )
    };

    if result.timed_out {
        Ok(StepOutcome::timed_out(
            SETUP_STEP_NAME.to_owned(),
            result.duration_ms,
            Some(command_outcome),
        ))
    } else if result.output.status.success() {
        Ok(StepOutcome::new(
            SETUP_STEP_NAME.to_owned(),
            result.duration_ms,
            Some(command_outcome),
            ExpectationsOutcome::new(None, Vec::new()),
        ))
    } else {
        Ok(StepOutcome::new(
            SETUP_STEP_NAME.to_owned(),
            result.duration_ms,
            Some(command_outcome),
            failed_expectations_outcome(),
        ))
    }
}

fn build_setup_script(
    case_id: &str,
    scenario: &ContainerScenario,
) -> Result<String> {
    let mut commands = vec!["set -eu".to_owned()];

    for operation in scenario.setup() {
        match operation {
            SetupOperation::CopyFixtureDir { from, to } => {
                let mount = scenario
                    .mounts()
                    .get(to.mount())
                    .ok_or_else(|| {
                        eyre!(
                            "container scenario {:?} references unknown mount {:?}",
                            scenario.id(),
                            to.mount()
                        )
                    })?;
                let target_path =
                    join_container_path(mount.container_path(), to.path())?;

                commands
                    .push(format!("mkdir -p {}", shell_quote(&target_path)));
                commands.push(format!(
                    "cp -a {}/{}/. {}",
                    shell_quote(FIXTURE_MOUNT_PATH),
                    shell_quote(from),
                    shell_quote(&target_path)
                ));
            },
        }
    }

    for mount in scenario.mounts().values() {
        commands.push(format!(
            "chown -R {APP_UID_GID} {}",
            shell_quote(mount.container_path())
        ));
    }

    for precondition in scenario.preconditions() {
        match precondition {
            Precondition::DifferentDevices { left, right } => {
                let left_path = scenario
                    .mounts()
                    .get(left)
                    .map(ScenarioMount::container_path);
                let right_path = scenario
                    .mounts()
                    .get(right)
                    .map(ScenarioMount::container_path);
                let Some(left_path) = left_path else {
                    return Err(eyre!(
                        "container scenario {:?} references unknown mount {:?}",
                        scenario.id(),
                        left
                    ));
                };
                let Some(right_path) = right_path else {
                    return Err(eyre!(
                        "container scenario {:?} references unknown mount {:?}",
                        scenario.id(),
                        right
                    ));
                };

                commands.push(format!(
                    "left_dev=$(stat -c %d {}); right_dev=$(stat -c %d {}); \
                     if [ \"$left_dev\" = \"$right_dev\" ]; then \
                     echo \"precondition failed for case {}: mounts {} and {} must be on different devices\" >&2; \
                     exit 1; \
                     fi",
                    shell_quote(left_path),
                    shell_quote(right_path),
                    shell_quote(case_id),
                    shell_quote(left),
                    shell_quote(right)
                ));
            },
        }
    }

    Ok(commands.join("; "))
}

fn failed_expectations_outcome() -> ExpectationsOutcome {
    ExpectationsOutcome::new(None, vec![
        tfmttools_test_harness::ExpectationOutcome::NotPresent(
            Utf8PathBuf::from("container-setup"),
        ),
    ])
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

fn volume_mount_arg(volume_name: &str, mount: &ScenarioMount) -> String {
    format!("type=volume,src={volume_name},dst={}", mount.container_path())
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
            prefix: format!(
                "tfmttools-{}",
                sanitize_volume_component(run_id, 24)
            ),
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
            .find_map(|(candidate, name)| {
                (candidate == alias).then_some(name.as_str())
            })
            .ok_or_else(|| {
                eyre!("container case is missing volume for mount {alias:?}")
            })
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
            .map(|(_, volume_name)| {
                format!("{runtime_command} volume rm -f {volume_name}")
            })
            .collect()
    }
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
