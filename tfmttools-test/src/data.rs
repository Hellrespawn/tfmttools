use camino::Utf8Path;
use color_eyre::Result;
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TestCaseData {
    description: String,
    reference: IndexMap<String, Vec<String>>,
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

    pub fn reference(&self) -> &IndexMap<String, Vec<String>> {
        &self.reference
    }

    pub fn tests(&self) -> &IndexMap<String, TestData> {
        &self.tests
    }
}

#[derive(Debug, Deserialize)]
pub struct TestData {
    command: Option<String>,
    expectation: String,
}

impl TestData {
    pub fn command(&self) -> Option<&String> {
        self.command.as_ref()
    }

    pub fn expectation(&self) -> &str {
        &self.expectation
    }
}
