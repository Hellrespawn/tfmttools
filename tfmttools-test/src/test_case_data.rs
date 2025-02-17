use std::collections::HashMap;

use camino::Utf8Path;
use color_eyre::Result;
use serde::Deserialize;

use crate::test_case::TestType;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestCaseData {
    pub template: Option<String>,
    pub template_arguments: Option<Vec<String>>,
    pub global_arguments: Option<Vec<String>>,
    pub rename_arguments: Option<Vec<String>>,
    pub reference: Option<HashMap<String, String>>,
    pub types: Option<Vec<TestType>>,
    pub extends: Option<String>,
}

impl TestCaseData {
    pub fn from_file(path: &Utf8Path) -> Result<Self> {
        let body = fs_err::read_to_string(path)?;

        let test_case_data: TestCaseData = serde_json::from_str(&body)?;

        Ok(test_case_data)
    }

    pub fn merge(self, other: TestCaseData) -> TestCaseData {
        TestCaseData {
            template: other.template.or(self.template),
            template_arguments: other
                .template_arguments
                .or(self.template_arguments),
            global_arguments: other.global_arguments.or(self.global_arguments),
            rename_arguments: other.rename_arguments.or(self.rename_arguments),
            reference: other.reference.or(self.reference),
            types: other.types.or(self.types),
            // Treated differently
            extends: self.extends.or(None),
        }
    }
}
