use std::collections::{BTreeMap, BTreeSet};

use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use libtest_mimic::Arguments;
use serde::Deserialize;
use tfmttools_test_harness::FixtureDirs;

use crate::case::{ContainerCase, case_id_from_path};

const SCENARIO_SUFFIX: &str = ".scenario.json";

#[derive(Debug, Clone)]
pub struct ContainerScenario {
    id: String,
    description: String,
    mounts: BTreeMap<String, ScenarioMount>,
    workdir: String,
    input: ScenarioPathRef,
    setup: Vec<SetupOperation>,
    preconditions: Vec<Precondition>,
}

impl ContainerScenario {
    pub fn from_file(path: &Utf8Path) -> Result<Self> {
        let body = fs_err::read_to_string(path)?;
        let id = scenario_id_from_path(path)?;
        Self::from_json_str(&id, &body)
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn mounts(&self) -> &BTreeMap<String, ScenarioMount> {
        &self.mounts
    }

    pub fn workdir(&self) -> &str {
        &self.workdir
    }

    pub fn setup(&self) -> &[SetupOperation] {
        &self.setup
    }

    pub fn preconditions(&self) -> &[Precondition] {
        &self.preconditions
    }

    pub fn scenario_path(case: &ContainerCase) -> Utf8PathBuf {
        FixtureDirs::container()
            .scenario_dir()
            .join(format!("{}.scenario.json", case.scenario()))
    }

    fn validate(&self) -> Result<()> {
        if self.workdir != "/work" {
            return Err(eyre!(
                "container scenario {:?} must use /work as workdir",
                self.id
            ));
        }

        let mut seen_paths = BTreeSet::new();

        for (alias, mount) in &self.mounts {
            validate_alias(alias)?;

            if mount.kind != MountKind::Volume {
                return Err(eyre!(
                    "container scenario {:?} mount {alias:?} must use kind=volume",
                    self.id
                ));
            }

            let expected_path = expected_mount_path(alias);
            if mount.container_path != expected_path {
                return Err(eyre!(
                    "container scenario {:?} mount {alias:?} must use {expected_path:?}, got {:?}",
                    self.id,
                    mount.container_path
                ));
            }

            if !seen_paths.insert(mount.container_path.clone()) {
                return Err(eyre!(
                    "container scenario {:?} reuses mount path {:?}",
                    self.id,
                    mount.container_path
                ));
            }
        }

        validate_path_ref(self.id(), "input", &self.mounts, &self.input)?;

        for operation in &self.setup {
            match operation {
                SetupOperation::CopyFixtureDir { from, to } => {
                    fixture_source_dir(from)?;
                    validate_path_ref(
                        self.id(),
                        "setup.copy-fixture-dir.to",
                        &self.mounts,
                        to,
                    )?;
                },
            }
        }

        for precondition in &self.preconditions {
            match precondition {
                Precondition::DifferentDevices { left, right } => {
                    validate_mount_alias(self.id(), &self.mounts, left)?;
                    validate_mount_alias(self.id(), &self.mounts, right)?;
                },
            }
        }

        Ok(())
    }

    fn from_json_str(id: &str, body: &str) -> Result<Self> {
        let data: ContainerScenarioData = serde_json::from_str(body)?;
        let scenario = Self {
            id: id.to_owned(),
            description: data.description,
            mounts: data.mounts,
            workdir: data.workdir,
            input: data.input,
            setup: data.setup,
            preconditions: data.preconditions.unwrap_or_default(),
        };
        scenario.validate()?;
        Ok(scenario)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScenarioMount {
    kind: MountKind,
    container_path: String,
    #[serde(default)]
    driver: Option<String>,
    #[serde(default)]
    driver_opts: BTreeMap<String, String>,
}

impl ScenarioMount {
    pub fn container_path(&self) -> &str {
        &self.container_path
    }

    pub fn driver(&self) -> Option<&str> {
        self.driver.as_deref()
    }

    pub fn driver_opts(&self) -> &BTreeMap<String, String> {
        &self.driver_opts
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MountKind {
    Volume,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ScenarioPathRef {
    mount: String,
    path: String,
}

impl ScenarioPathRef {
    pub fn mount(&self) -> &str {
        &self.mount
    }

    pub fn path(&self) -> &str {
        &self.path
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum SetupOperation {
    CopyFixtureDir { from: String, to: ScenarioPathRef },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Precondition {
    DifferentDevices { left: String, right: String },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct ContainerScenarioData {
    description: String,
    mounts: BTreeMap<String, ScenarioMount>,
    workdir: String,
    input: ScenarioPathRef,
    setup: Vec<SetupOperation>,
    preconditions: Option<Vec<Precondition>>,
}

pub fn discover_cases(args: &Arguments) -> Result<Vec<ContainerCase>> {
    let fixture_dirs = FixtureDirs::container();
    let case_dir = fixture_dirs.case_dir();

    if !case_dir.exists() {
        return Err(eyre!("container case directory missing at {case_dir}"));
    }

    let mut case_paths = fs_err::read_dir(&case_dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter_map(|path| camino::Utf8PathBuf::from_path_buf(path).ok())
        .filter(|path| {
            path.file_name().is_some_and(|name| name.ends_with(".case.json"))
        })
        .collect::<Vec<_>>();
    case_paths.sort();

    if case_paths.is_empty() {
        return Err(eyre!("did not find any container cases at {case_dir}"));
    }

    let mut cases = Vec::new();

    for path in case_paths {
        let case_id = case_id_from_path(&path)?;
        if !matches_filters(&case_id, args) {
            continue;
        }

        cases.push(ContainerCase::from_file(&path)?);
    }

    Ok(cases)
}

pub fn fixture_source_dir(name: &str) -> Result<Utf8PathBuf> {
    let fixture_dirs = FixtureDirs::container();

    let path = match name {
        "audio" => fixture_dirs.audio_dir(),
        "extra" => fixture_dirs.extra_dir(),
        "template" => fixture_dirs.template_dir(),
        _ => {
            return Err(eyre!(
                "unsupported container fixture source {name:?}; expected audio, extra, or template"
            ));
        },
    };

    path.exists().then_some(path.clone()).ok_or_else(|| {
        eyre!("container fixture source directory missing at {path}")
    })
}

fn validate_alias(alias: &str) -> Result<()> {
    if alias.is_empty() {
        return Err(eyre!("container mount alias must not be empty"));
    }

    if alias.chars().all(|char| {
        char.is_ascii_lowercase() || char.is_ascii_digit() || char == '-'
    }) {
        Ok(())
    } else {
        Err(eyre!(
            "container mount alias {alias:?} must use lowercase letters, digits, or hyphens"
        ))
    }
}

fn expected_mount_path(alias: &str) -> String {
    if alias == "config" {
        "/work/config".to_owned()
    } else {
        format!("/mnt/{alias}")
    }
}

fn validate_path_ref(
    scenario_id: &str,
    field_name: &str,
    mounts: &BTreeMap<String, ScenarioMount>,
    path_ref: &ScenarioPathRef,
) -> Result<()> {
    validate_mount_alias(scenario_id, mounts, path_ref.mount())?;

    let path = Utf8Path::new(path_ref.path());
    if path.is_absolute() {
        return Err(eyre!(
            "container scenario {scenario_id:?} field {field_name} must use a relative path, got {:?}",
            path_ref.path()
        ));
    }

    if path.components().any(|component| component.as_str() == "..") {
        return Err(eyre!(
            "container scenario {scenario_id:?} field {field_name} must not contain '..', got {:?}",
            path_ref.path()
        ));
    }

    Ok(())
}

fn validate_mount_alias(
    scenario_id: &str,
    mounts: &BTreeMap<String, ScenarioMount>,
    alias: &str,
) -> Result<()> {
    mounts.contains_key(alias).then_some(()).ok_or_else(|| {
        eyre!("container scenario {scenario_id:?} references unknown mount {alias:?}")
    })
}

fn scenario_id_from_path(path: &Utf8Path) -> Result<String> {
    let file_name = path
        .file_name()
        .ok_or_eyre("container scenario path must include a file name")?;

    file_name.strip_suffix(SCENARIO_SUFFIX).map(str::to_owned).ok_or_else(
        || eyre!("container scenario file must end with {SCENARIO_SUFFIX}"),
    )
}

fn matches_filters(case_id: &str, args: &Arguments) -> bool {
    let filter_matches = args
        .filter
        .as_ref()
        .is_none_or(|filter| matches_filter(case_id, filter, args.exact));
    let skip_matches =
        args.skip.iter().any(|skip| matches_filter(case_id, skip, args.exact));

    filter_matches && !skip_matches
}

fn matches_filter(case_id: &str, filter: &str, exact: bool) -> bool {
    if exact { case_id == filter } else { case_id.contains(filter) }
}

#[cfg(test)]
mod tests {
    use super::{ContainerScenario, matches_filter};

    #[test]
    fn exact_filters_match_only_full_case_id() {
        assert!(matches_filter("cross-device-nemo", "cross-device-nemo", true));
        assert!(!matches_filter("cross-device-nemo", "cross-device", true));
    }

    #[test]
    fn non_exact_filters_match_substrings() {
        assert!(matches_filter("cross-device-nemo", "device", false));
        assert!(!matches_filter("cross-device-nemo", "redo-only", false));
    }

    #[test]
    fn rejects_invalid_mount_paths() {
        let body = r#"{
          "description": "bad",
          "mounts": {
            "source": { "kind": "volume", "container-path": "/tmp/source" }
          },
          "workdir": "/work",
          "input": { "mount": "source", "path": "input" },
          "setup": []
        }"#;

        let error = ContainerScenario::from_json_str("invalid", body)
            .expect_err("invalid mount path should fail");

        assert!(error.to_string().contains("must use \"/mnt/source\""));
    }
}
