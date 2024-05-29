use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Output;

use assert_cmd::Command;
use assert_fs::TempDir;
use color_eyre::Result;
use termtree::Tree;
use tfmttools_fs::FileOrName;

use crate::template_reference::TemplateReference;
use crate::{
    TEST_AUDIO_FILE_DIR_NAME, TEST_CASE_DIR_NAME, TEST_CONFIG_DIR_NAME,
    TEST_DATA_DIRECTORY, TEST_TEMPLATE_DIR_NAME,
};

#[derive(Debug)]
pub struct TestCase {
    template: FileOrName,
    reference: HashMap<String, String>,
    arguments: Vec<String>,
    temp_dir: TempDir,
}

impl TestCase {
    pub fn load(case: &str) -> Result<Self> {
        let path = get_test_data_dir()
            .join(TEST_CASE_DIR_NAME)
            .join(format!("{case}.json"));

        let TemplateReference { template, reference, arguments } =
            TemplateReference::from_file(&path)?;

        Self::validate_template(&template);
        let reference = Self::validate_reference(reference);

        let temp_dir = TempDir::new()?;

        let case = TestCase {
            template,
            reference,
            arguments: arguments.unwrap_or_default(),
            temp_dir,
        };

        case.populate_templates()?;
        case.populate_files()?;

        Ok(case)
    }

    pub fn assert_apply(&self, report: bool) {
        let mut cmd = self.create_command();

        cmd.arg("rename")
            .arg("--custom-template-directory")
            .arg(self.get_template_dest_dir())
            .arg("--yes")
            .arg(&self.template.as_str());

        for argument in &self.arguments {
            cmd.arg(argument);
        }

        self.run_command(cmd, "apply", report);

        self.assert_files_exist(self.reference.values(), |p| {
            self.print_temp_dir_contents(&format!(
                "File was not renamed: {}",
                p.display()
            ));
        });
    }

    pub fn assert_undo(&self, report: bool) {
        let mut cmd = Command::cargo_bin("tfmt").unwrap();

        cmd.arg("--custom-config-directory")
            .arg(self.get_config_dest_dir())
            .arg("undo")
            .arg("--yes");

        self.run_command(cmd, "undo", report);

        self.assert_files_exist(self.reference.keys(), |p| {
            self.print_temp_dir_contents(&format!(
                "{} was not returned to original location.",
                p.display()
            ));
        });

        self.assert_files_dont_exist(self.reference.values(), |p| {
            self.print_temp_dir_contents(&format!(
                "{} is still in renamed location.",
                p.display()
            ));
        });
    }

    pub fn assert_redo(&self, report: bool) {
        let mut cmd = Command::cargo_bin("tfmt").unwrap();

        cmd.arg("--custom-config-directory")
            .arg(self.get_config_dest_dir())
            .arg("redo")
            .arg("--yes");

        self.run_command(cmd, "redo", report);

        self.assert_files_exist(self.reference.values(), |p| {
            self.print_temp_dir_contents(&format!(
                "{} was not returned to renamed location.",
                p.display()
            ));
        });

        self.assert_files_dont_exist(self.reference.keys(), |p| {
            self.print_temp_dir_contents(&format!(
                "{} is still in undone location.",
                p.display()
            ));
        });
    }

    fn get_audio_dest_dir(&self) -> PathBuf {
        self.temp_dir.join(TEST_AUDIO_FILE_DIR_NAME)
    }

    fn get_config_dest_dir(&self) -> PathBuf {
        self.temp_dir.join(TEST_CONFIG_DIR_NAME)
    }

    fn get_template_dest_dir(&self) -> PathBuf {
        self.temp_dir.join(TEST_TEMPLATE_DIR_NAME)
    }

    fn create_command(&self) -> Command {
        let mut cmd = Command::cargo_bin("tfmt").unwrap();

        cmd.arg("--custom-config-directory").arg(self.get_config_dest_dir());

        cmd
    }

    fn run_command(&self, mut cmd: Command, name: &str, report: bool) {
        if report {
            println!();
            self.print_temp_dir_contents(&format!("Before {name}"));
        }

        let assert = cmd.current_dir(self.temp_dir.path()).assert();

        if report {
            let output = assert.get_output();
            self.report_command_output(name, output);
        }

        assert.success();
    }

    fn report_command_output(&self, name: &str, output: &Output) {
        println!("Exit status: {}", output.status.code().unwrap_or(-1));
        println!();

        println!("```stdout");
        println!("{}```", indent(String::from_utf8_lossy(&output.stdout)));

        println!();

        println!("```stderr");
        println!("{}```", indent(String::from_utf8_lossy(&output.stderr)));
        println!();

        self.print_temp_dir_contents(&format!("After {name}"));
    }

    fn validate_template(file_or_name: &FileOrName) {
        if let FileOrName::Name(name) = file_or_name {
            let path = get_template_data_dir().join(format!("{name}.tfmt"));

            assert!(path.is_file(), "Template {} does not exist.", name);
        }
    }

    fn populate_templates(&self) -> Result<()> {
        let paths: Vec<PathBuf> = fs_err::read_dir(get_template_data_dir())?
            .flat_map(|result| result.map(|entry| entry.path()))
            .collect();

        let template_dir = self.get_template_dest_dir();

        fs_err::create_dir(&template_dir)?;

        for template_path in &paths {
            // Templates are selected by is_file, should always have a filename
            // so path.file_name().unwrap() should be safe.

            assert!(template_path.file_name().is_some());
            let file_name = template_path.file_name().unwrap();

            fs_err::copy(template_path, template_dir.join(file_name))?;
        }

        Ok(())
    }

    fn populate_files(&self) -> Result<()> {
        let paths: Vec<PathBuf> = fs_err::read_dir(get_audio_data_dir())?
            .flat_map(|result| result.map(|entry| entry.path()))
            .collect();

        let audio_dir = self.get_audio_dest_dir();

        fs_err::create_dir(&audio_dir)?;

        for audiofile_path in &paths {
            // Audio files are selected by is_file, should always have a
            // filename so path.file_name().unwrap() should be safe.

            assert!(audiofile_path.file_name().is_some());

            fs_err::copy(
                audiofile_path,
                audio_dir.join(audiofile_path.file_name().unwrap()),
            )?;
        }

        Ok(())
    }

    fn validate_reference(
        reference: HashMap<String, String>,
    ) -> HashMap<String, String> {
        let mut valid_reference = HashMap::new();

        for (key, value) in reference {
            let path = get_audio_data_dir().join(&key);

            assert!(
                path.is_file(),
                "Unable to validate audio file {}",
                path.display()
            );

            valid_reference.insert(format!("files/{key}"), value);
        }

        valid_reference
    }

    fn assert_files_exist<'a, I, F>(&self, reference: I, failure_callback: F)
    where
        I: Iterator<Item = &'a String>,
        F: Fn(&Path),
    {
        for file in reference {
            let path = self.temp_dir.join(file);

            if !path.is_file() {
                failure_callback(&path);
                panic!("File does not exist: {}", path.display());
            }
        }
    }

    fn assert_files_dont_exist<'a, I, F>(
        &self,
        reference: I,
        failure_callback: F,
    ) where
        I: Iterator<Item = &'a String>,
        F: Fn(&Path),
    {
        for file in reference {
            let path = self.temp_dir.join(file);

            if path.is_file() {
                failure_callback(&path);
                panic!("File exists: {}", path.display());
            }
        }
    }

    fn print_temp_dir_contents(&self, message: &str) {
        fn label(p: &Path) -> String {
            p.file_name().unwrap().to_str().unwrap().to_owned()
        }

        fn tree(p: &Path) -> Tree<String> {
            fs_err::read_dir(p)
                .expect("Unable to read temp_dir")
                .filter_map(|e| e.ok())
                .fold(Tree::new(label(p)), |mut root, entry| {
                    let dir = entry.metadata().unwrap();
                    if dir.is_dir() {
                        root.push(tree(&entry.path()));
                    } else {
                        root.push(Tree::new(label(&entry.path())));
                    }
                    root
                })
        }

        println!(
            "{}:\n{}",
            message,
            indent(tree(self.temp_dir.path()).to_string())
        )
    }
}

fn get_test_data_dir() -> PathBuf {
    PathBuf::from(TEST_DATA_DIRECTORY)
}

fn get_template_data_dir() -> PathBuf {
    get_test_data_dir().join("template")
}

fn get_audio_data_dir() -> PathBuf {
    get_test_data_dir().join("music")
}

fn indent<S: AsRef<str>>(string: S) -> String {
    textwrap::indent(string.as_ref(), "····")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_testcase() -> Result<()> {
        let case = TestCase::load("simple_input")?;

        dbg!(case);

        Ok(())
    }
}
