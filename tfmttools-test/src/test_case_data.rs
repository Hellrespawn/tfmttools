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
    pub reference: Option<HashMap<String, Option<String>>>,
    pub types: Option<Vec<TestType>>,
    pub extends: Option<String>,
}

impl TestCaseData {
    pub fn from_file(path: &Utf8Path) -> Result<Self> {
        let body = fs_err::read_to_string(path)?;

        let test_case_data: TestCaseData = serde_json::from_str(&body)?;

        Ok(test_case_data)
    }

    pub fn inherit_from(self, ancestor: TestCaseData) -> TestCaseData {
        TestCaseData {
            template: self.template.or(ancestor.template),
            template_arguments: self
                .template_arguments
                .or(ancestor.template_arguments),
            global_arguments: self
                .global_arguments
                .or(ancestor.global_arguments),
            rename_arguments: self
                .rename_arguments
                .or(ancestor.rename_arguments),
            reference: self.reference.or(ancestor.reference),
            types: self.types.or(ancestor.types),

            // Treated differently
            extends: ancestor.extends.or(None),
        }
    }
}
