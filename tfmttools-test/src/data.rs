use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug)]
pub struct Expectation {
    path: Utf8PathBuf,
    no_previous: bool,
}

impl Expectation {
    fn new(string: &str) -> Result<Expectation> {
        let mut iter = string.split(':');

        let path = iter
            .next()
            .ok_or(eyre!("Expectation was None (but valid string?"))?;

        let mut expectation =
            Expectation { path: Utf8PathBuf::from(path), no_previous: false };

        for option in iter {
            match option {
                "noprevious" => expectation.no_previous = true,
                other => {
                    return Err(eyre!(
                        "Unknown expectation option '{}'",
                        other,
                    ));
                },
            }
        }

        Ok(expectation)
    }

    pub fn verify_exists(&self, prefix: &Utf8Path) -> bool {
        prefix.join(&self.path).exists()
    }

    pub fn verify_no_longer_exists(&self, prefix: &Utf8Path) -> bool {
        self.no_previous || !self.verify_exists(prefix)
    }

    pub fn path(&self) -> &Utf8Path {
        self.path.as_ref()
    }
}

impl<'de> Deserialize<'de> for Expectation {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;
        let expectation =
            Expectation::new(&string).map_err(serde::de::Error::custom)?;

        Ok(expectation)
    }
}

#[derive(Debug, Deserialize)]
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
