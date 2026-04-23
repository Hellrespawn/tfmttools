use camino::Utf8Path;
use color_eyre::Result;
use color_eyre::eyre::{OptionExt, eyre};
use serde::Deserialize;
use tfmttools_test_harness::CaseOutcome;

const CASE_SUFFIX: &str = ".case.json";

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

pub fn case_id_from_path(path: &Utf8Path) -> Result<String> {
    let file_name = path
        .file_name()
        .ok_or_eyre("container case path must include a file name")?;

    file_name
        .strip_suffix(CASE_SUFFIX)
        .map(str::to_owned)
        .ok_or_else(|| eyre!("container case file must end with {CASE_SUFFIX}"))
}

pub fn run_case(case: &ContainerCase) -> CaseOutcome {
    let _ = case.scenario();

    CaseOutcome::new(
        case.id().to_owned(),
        case.description().to_owned(),
        0,
        Vec::new(),
        None,
    )
}

#[cfg(test)]
mod tests {
    use camino::Utf8Path;

    use super::case_id_from_path;

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
}
