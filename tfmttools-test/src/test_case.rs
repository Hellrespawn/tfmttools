use std::collections::HashMap;
use std::fmt::Write;

use assert_cmd::Command;
use assert_fs::TempDir;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use serde::Deserialize;
use tfmttools_fs::PathIterator;

use crate::file_tree::FileTreeNode;
use crate::predicates::{PredicateInput, check_reference};
use crate::test_case_data::TestCaseData;
use crate::test_failure::{CommandOutput, TestFailure};
use crate::{
    TEST_AUDIO_FILE_DIR_NAME, TEST_CASE_DIR_NAME, TEST_CONFIG_DIR_NAME,
    TEST_DATA_DIRECTORY, TEST_TEMPLATE_DIR_NAME,
};

#[derive(Debug, Deserialize, Copy, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum TestType {
    Apply,
    Undo,
    Redo,
    PreviousData,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestType::Apply => write!(f, "apply"),
            TestType::Undo => write!(f, "undo"),
            TestType::Redo => write!(f, "redo"),
            TestType::PreviousData => write!(f, "previous-data"),
        }
    }
}

#[derive(Debug)]
pub struct TestCase {
    pub name: String,
    template: String,
    template_arguments: Vec<String>,
    global_arguments: Vec<String>,
    rename_arguments: Vec<String>,
    reference: HashMap<String, Option<String>>,
    test_type: TestType,
    temp_dir: TempDir,
}

impl TestCase {
    fn temp_dir_path(&self) -> Utf8PathBuf {
        Utf8PathBuf::from_path_buf(self.temp_dir.path().to_path_buf())
            .expect("temp_dir should be valid UTF-8")
    }

    pub fn load_all() -> Result<Vec<Self>> {
        let test_cases_dir = get_test_data_dir().join(TEST_CASE_DIR_NAME);

        let test_case_iterator =
            PathIterator::single_directory(&test_cases_dir);

        let cases = test_case_iterator.flatten().filter(|p| {
            let component = p.components().last().expect("iterator of path components should always have one element");

            component.as_str().ends_with(".case.json")
        }).map(|p|Self::load(&p))
        .try_fold(Vec::new(), |mut cases, case| {
            cases.extend(case?);
            Ok::<_, color_eyre::Report>(cases)
        })?;

        Ok(cases)
    }

    pub fn load(path: &Utf8Path) -> Result<Vec<Self>> {
        let TestCaseData {
            template,
            template_arguments,
            global_arguments,
            rename_arguments,
            reference,
            types,
            ..
        } = Self::load_data(path)?;

        let template_name = Self::validate_template(path, template)?;

        let reference = Self::validate_reference(path, reference)?;

        if types.is_none() {
            return Err(eyre!(
                "Test case data does not define test types: {path}",
            ));
        }

        let types = types.unwrap();

        let cases = types
            .into_iter()
            .map(|test_type| {
                let temp_dir = TempDir::new()?;

                let case_data_name = path
                    .file_name()
                    .expect("Path to file should always have a file name")
                    .replace(".case.json", "")
                    .to_owned();

                let name = format!("{}::{}", case_data_name, test_type);

                let case = TestCase {
                    name,
                    template: template_name.clone(),
                    reference: reference.clone(),
                    template_arguments: template_arguments
                        .clone()
                        .unwrap_or_default(),
                    global_arguments: global_arguments
                        .clone()
                        .unwrap_or_default(),
                    rename_arguments: rename_arguments
                        .clone()
                        .unwrap_or_default(),
                    test_type,
                    temp_dir,
                };

                case.populate_templates()?;
                case.populate_files()?;

                Ok(case)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(cases)
    }

    fn load_data(path: &Utf8Path) -> Result<TestCaseData> {
        let mut test_case_data = TestCaseData::from_file(path)?;

        while let Some(extends) = &test_case_data.extends {
            let ancestor_path = get_test_data_dir()
                .join(TEST_CASE_DIR_NAME)
                .join(format!("{}.case.json", extends));

            let ancestor_data = TestCaseData::from_file(&ancestor_path)?;

            test_case_data = test_case_data.inherit_from(ancestor_data);
        }

        Ok(test_case_data)
    }

    pub fn run_test_case(&self) -> Result<(), Box<TestFailure>> {
        match self.test_type {
            TestType::Apply => self.run_test(TestType::Apply),
            TestType::Undo => {
                self.run_test(TestType::Apply)?;
                self.run_test(TestType::Undo)
            },
            TestType::Redo => {
                self.run_test(TestType::Apply)?;
                self.run_test(TestType::Undo)?;
                self.run_test(TestType::Redo)
            },
            TestType::PreviousData => {
                self.run_test(TestType::Apply)?;
                self.run_test(TestType::Undo)?;
                self.run_test(TestType::PreviousData)
            },
        }
    }

    pub fn run_test(
        &self,
        test_type: TestType,
    ) -> Result<(), Box<TestFailure>> {
        let cmd = self.create_command(test_type);
        let command_output = self.run_command(cmd, test_type);

        let message = self.verify_command(&command_output, test_type);

        if let Some(message) = message {
            Err(Box::new(TestFailure {
                test_case_name: self.name.clone(),
                command_output,
                message,
            }))
        } else {
            Ok(())
        }
    }

    fn verify_command(
        &self,
        command_output: &CommandOutput,
        test_type: TestType,
    ) -> Option<String> {
        let predicate_input = PredicateInput::new(
            self.temp_dir_path(),
            self.temp_dir_path(),
            self.reference.iter(),
            test_type,
        );

        let predicate_results = check_reference(predicate_input);

        let exit_code = command_output.exit_code;

        if exit_code != 0 {
            Some(format!("Command exited with code: {}", exit_code))
        } else if predicate_results.is_error() {
            Some(predicate_results.to_string())
        } else {
            None
        }
    }

    fn get_audio_dest_dir(&self) -> Utf8PathBuf {
        self.temp_dir_path().join(TEST_AUDIO_FILE_DIR_NAME)
    }

    fn get_config_dest_dir(&self) -> Utf8PathBuf {
        self.temp_dir_path().join(TEST_CONFIG_DIR_NAME)
    }

    fn get_template_dest_dir(&self) -> Utf8PathBuf {
        self.temp_dir_path().join(TEST_TEMPLATE_DIR_NAME)
    }

    fn create_command(&self, test_type: TestType) -> Command {
        let mut cmd = Command::cargo_bin("tfmt").unwrap();

        cmd.arg("--custom-config-directory").arg(self.get_config_dest_dir());

        match test_type {
            TestType::Apply => {
                for arg in &self.global_arguments {
                    cmd.arg(arg);
                }

                cmd.arg("rename")
                    .arg("--custom-template-directory")
                    .arg(self.get_template_dest_dir())
                    .arg("--yes");

                for arg in &self.rename_arguments {
                    cmd.arg(arg);
                }

                cmd.arg(self.template.as_str());
                cmd.arg("--");

                for argument in &self.template_arguments {
                    cmd.arg(argument);
                }
            },
            TestType::Undo => {
                cmd.arg("undo").arg("--yes");
            },
            TestType::Redo => {
                cmd.arg("redo").arg("--yes");
            },
            TestType::PreviousData => {
                for arg in &self.global_arguments {
                    cmd.arg(arg);
                }

                cmd.arg("rename")
                    .arg("--custom-template-directory")
                    .arg(self.get_template_dest_dir())
                    .arg("--yes");

                for arg in &self.rename_arguments {
                    cmd.arg(arg);
                }
            },
        }

        cmd
    }

    fn get_invocation(cmd: &Command) -> String {
        let mut string = String::new();

        write!(string, "{}", cmd.get_program().to_string_lossy()).unwrap();

        for arg in cmd.get_args() {
            write!(string, " {}", arg.to_string_lossy()).unwrap();
        }

        string
    }

    fn run_command(
        &self,
        mut cmd: Command,
        test_type: TestType,
    ) -> CommandOutput {
        let file_tree_before = self.create_temp_dir_file_tree();

        let result = cmd.current_dir(self.temp_dir.path()).output();

        let file_tree_after = self.create_temp_dir_file_tree();

        match result {
            Ok(output) => {
                CommandOutput {
                    action_name: test_type.to_string(),
                    invocation: Self::get_invocation(&cmd),
                    exit_code: output.status.code().unwrap_or(-255),
                    file_tree_before,
                    file_tree_after,
                    stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                    stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                }
            },
            Err(error) => {
                CommandOutput {
                    action_name: test_type.to_string(),
                    invocation: Self::get_invocation(&cmd),
                    exit_code: i32::MIN,
                    file_tree_before,
                    file_tree_after,
                    stdout: String::new(),
                    stderr: error.to_string(),
                }
            },
        }
    }

    fn validate_template(
        path: &Utf8Path,
        template_name: Option<String>,
    ) -> Result<String> {
        if let Some(template_name) = template_name {
            let path =
                get_template_data_dir().join(format!("{template_name}.tfmt"));

            if path.is_file() {
                Ok(template_name)
            } else {
                Err(eyre!("Template {} does not exist.", template_name))
            }
        } else {
            Err(Self::template_validation_error(path, "name"))
        }
    }

    fn template_validation_error(
        path: &Utf8Path,
        property: &str,
    ) -> color_eyre::Report {
        eyre!("Test case data does not define {property}: {path}",)
    }

    fn populate_templates(&self) -> Result<()> {
        let paths = fs_err::read_dir(get_template_data_dir())?
            .flat_map(|result| {
                result.map(|entry| {
                    Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
                })
            })
            .flatten()
            .collect::<Vec<_>>();

        let template_dir = self.get_template_dest_dir();

        fs_err::create_dir(&template_dir)?;

        for template_path in &paths {
            // Templates are selected by is_file, should always have a filename
            // so path.file_name().unwrap() should be safe.
            let file_name = template_path.file_name().unwrap();

            fs_err::copy(template_path, template_dir.join(file_name))?;
        }

        Ok(())
    }

    fn populate_files(&self) -> Result<()> {
        let paths = fs_err::read_dir(get_audio_data_dir())?
            .flat_map(|result| {
                result.map(|entry| {
                    Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
                })
            })
            .flatten()
            .collect::<Vec<_>>();

        let audio_dir = self.get_audio_dest_dir();

        fs_err::create_dir(&audio_dir)?;

        for audiofile_path in &paths {
            // Audio files are selected by is_file, should always have a
            // filename so path.file_name().unwrap() should be safe.

            fs_err::copy(
                audiofile_path,
                audio_dir.join(audiofile_path.file_name().unwrap()),
            )?;
        }

        Ok(())
    }

    fn validate_reference(
        path: &Utf8Path,
        reference: Option<HashMap<String, Option<String>>>,
    ) -> Result<HashMap<String, Option<String>>> {
        if let Some(reference) = reference {
            let mut valid_reference = HashMap::new();

            for (key, value) in reference {
                let file_path = get_audio_data_dir().join(&key);

                if !file_path.is_file() {
                    return Err(eyre!(
                        "Unable to validate audio file {}",
                        file_path
                    ));
                }

                valid_reference.insert(format!("files/{key}"), value);
            }

            Ok(valid_reference)
        } else {
            Err(Self::template_validation_error(path, "reference"))
        }
    }

    fn create_temp_dir_file_tree(&self) -> FileTreeNode {
        let temp_dir_path =
            Utf8PathBuf::from_path_buf(self.temp_dir.path().to_path_buf())
                .unwrap();

        FileTreeNode::from_path(&temp_dir_path).unwrap()
    }
}

fn get_test_data_dir() -> Utf8PathBuf {
    Utf8PathBuf::from(TEST_DATA_DIRECTORY)
}

fn get_template_data_dir() -> Utf8PathBuf {
    get_test_data_dir().join("template")
}

fn get_audio_data_dir() -> Utf8PathBuf {
    get_test_data_dir().join("music")
}

// fn indent<S: AsRef<str>>(string: S) -> String {
//     const PREFIX: &str = "    ";
//     // const PREFIX: &str = "····";

//     textwrap::indent(string.as_ref(), PREFIX)
// }
