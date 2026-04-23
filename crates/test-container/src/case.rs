use std::time::{Instant, SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use serde::Deserialize;
use tfmttools_test_harness::{
    CaseOutcome, CommandOutcome, ExpectationsOutcome, StepOutcome,
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
}

impl ContainerCase {
    pub fn from_file(path: &Utf8Path) -> Result<Self> {
        let body = fs_err::read_to_string(path)?;
        let data: ContainerCaseData = serde_json::from_str(&body)?;
        let id = case_id_from_path(path)?;

        Ok(Self { id, scenario: data.scenario, description: data.description })
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
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContainerCaseData {
    description: String,
    scenario: String,
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

    let mut resources = CaseResources::new(case.id(), ctx.run_id);
    let step = match create_case_volumes(ctx.runtime, &scenario, &mut resources)
    {
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
            StepOutcome::failed(SETUP_STEP_NAME.to_owned(), error.to_string())
        },
    };

    let cleanup_commands = resources.cleanup_commands(ctx.runtime.command());

    if !ctx.preserve {
        for volume_name in resources.volume_names().iter().rev() {
            let _ = ctx.runtime.remove_volume(volume_name);
        }
    }

    ExecutedCase {
        outcome: CaseOutcome::new(
            case.id().to_owned(),
            format!("{} [{}]", case.description(), scenario.description()),
            case_started.elapsed().as_millis(),
            vec![step],
            None,
        ),
        volume_names: resources.volume_names().to_vec(),
        cleanup_commands,
    }
}

fn scenario_path(case: &ContainerCase) -> Utf8PathBuf {
    ContainerScenario::scenario_path(case)
}

fn create_case_volumes(
    runtime: &ContainerRuntime,
    scenario: &ContainerScenario,
    resources: &mut CaseResources,
) -> Result<()> {
    for (alias, mount) in scenario.mounts() {
        let volume_name = resources.push(alias);
        runtime.create_volume(volume_name, mount)?;
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
    let mut normalize_paths = Vec::new();

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
                normalize_paths.push(target_path);
            },
        }
    }

    for path in &normalize_paths {
        commands.push(format!("chown -R {APP_UID_GID} {}", shell_quote(path)));
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
