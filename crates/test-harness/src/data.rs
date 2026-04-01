use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestCaseData {
    description: String,

    expectations: IndexMap<String, Vec<Expectation>>,
    tests: IndexMap<String, TestData>,
}

impl TestCaseData {
    pub fn from_file(path: &Utf8Path) -> Result<Self> {
        let body = fs_err::read_to_string(path)?;

        let test_case_data: TestCaseData = serde_json::from_str(&body)?;

        Ok(test_case_data)
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn expectations(&self) -> &IndexMap<String, Vec<Expectation>> {
        &self.expectations
    }

    pub fn tests(&self) -> &IndexMap<String, TestData> {
        &self.tests
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExpectationOption {
    NoPrevious,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Expectation {
    path: Utf8PathBuf,
    checksum: Option<String>,
    #[serde(default)]
    options: Vec<ExpectationOption>,
}

impl Expectation {
    pub fn verify_exists(&self, prefix: &Utf8Path) -> bool {
        prefix.join(&self.path).exists()
    }

    pub fn verify_no_longer_exists(&self, prefix: &Utf8Path) -> bool {
        self.options.iter().any(|o| matches!(o, ExpectationOption::NoPrevious))
            || !self.verify_exists(prefix)
    }

    pub fn verify_checksum(&self, checksum: &str) -> bool {
        self.checksum
            .as_ref()
            .is_none_or(|self_checksum| self_checksum == checksum)
    }

    pub fn path(&self) -> &Utf8Path {
        self.path.as_ref()
    }

    pub fn checksum(&self) -> Option<&String> {
        self.checksum.as_ref()
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestData {
    command: Option<String>,
    expectations: Option<String>,
    #[serde(alias = "previous-expectations")]
    previous_expectations: Option<String>,
}

impl TestData {
    pub fn command(&self) -> Option<&String> {
        self.command.as_ref()
    }

    pub fn expectations(&self) -> Option<&String> {
        self.expectations.as_ref()
    }

    pub fn previous_expectations(&self) -> Option<&String> {
        self.previous_expectations.as_ref()
    }
}
