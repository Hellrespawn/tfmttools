use std::process::Output;

use camino::Utf8PathBuf;

fn mark(bool: bool) -> &'static str {
    if bool { "✅" } else { "❌" }
}

fn write_list_of_paths(
    f: &mut std::fmt::Formatter<'_>,
    paths: &[Utf8PathBuf],
    indent: usize,
) -> std::fmt::Result {
    for path in paths {
        writeln!(f, "{}- {}", " ".repeat(indent), path)?;
    }

    Ok(())
}

#[derive(Default, Debug)]
pub struct TestCaseOutcome {
    pub name: String,
    pub description: String,
    pub work_dir: Utf8PathBuf,
    pub missing_files: Option<Vec<Utf8PathBuf>>,
    pub test_outcomes: Vec<TestOutcome>,
}

impl TestCaseOutcome {
    pub fn new(
        name: String,
        description: String,
        work_dir: Utf8PathBuf,
    ) -> Self {
        Self { name, description, work_dir, ..Default::default() }
    }

    pub fn passed(&self) -> bool {
        self.passed_initial_expectation()
            && self.test_outcomes.iter().all(|outcome| outcome.passed())
    }

    pub fn passed_initial_expectation(&self) -> bool {
        self.missing_files.as_ref().is_none_or(|m| m.is_empty())
    }
}

impl std::fmt::Display for TestCaseOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Test case: '{}'", self.name)?;
        writeln!(f, "Description: {}", self.description)?;

        write!(f, "Working directory: {}", self.work_dir)?;
        if self.passed() {
            writeln!(f)?;
        } else {
            writeln!(f, " (persisted for debugging)")?;
        }

        writeln!(f)?;

        if let Some(missing_files) = &self.missing_files {
            writeln!(
                f,
                "{}: passed initial expectation",
                mark(missing_files.is_empty())
            )?;

            if !missing_files.is_empty() {
                writeln!(f, "Missing files:")?;
                write_list_of_paths(f, missing_files, 0)?;
            }
        }

        writeln!(f)?;

        for outcome in &self.test_outcomes {
            writeln!(f, "{}", outcome)?;
        }
        writeln!(f, "{}: Outcome of test case.", mark(self.passed()))?;

        Ok(())
    }
}

#[derive(Default, Debug)]
pub struct TestOutcome {
    pub name: String,
    pub command_outcome: Option<CommandOutcome>,
    pub remaining_files: Option<Vec<Utf8PathBuf>>,
    pub missing_files: Vec<Utf8PathBuf>,
}

impl TestOutcome {
    pub fn new(name: String) -> Self {
        Self { name, ..Default::default() }
    }

    pub fn passed(&self) -> bool {
        self.remaining_files
            .as_ref()
            .is_none_or(|remaining_files| remaining_files.is_empty())
            && self.missing_files.is_empty()
    }
}
impl std::fmt::Display for TestOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Outcome of test '{}'", self.name)?;

        writeln!(f)?;

        if let Some(command_outcome) = &self.command_outcome {
            writeln!(f, "{}", command_outcome)?;
        } else {
            writeln!(f, "Test failed before command outcome")?;
        }

        if let Some(remaining_files) = &self.remaining_files {
            writeln!(
                f,
                "{}: passed previous expectation",
                mark(remaining_files.is_empty())
            )?;

            if !remaining_files.is_empty() {
                writeln!(f, "Remaining files:")?;

                write_list_of_paths(f, remaining_files, 2)?;
            }

            writeln!(f)?;
        }
        writeln!(
            f,
            "{}: passed expectation",
            mark(self.missing_files.is_empty())
        )?;

        if !self.missing_files.is_empty() {
            writeln!(f, "Missing files:")?;

            write_list_of_paths(f, &self.missing_files, 2)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CommandOutcome {
    pub command: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

impl CommandOutcome {
    pub fn new(command: String, output: Output) -> Self {
        let success = output.status.success();
        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Self { command, success, exit_code, stdout, stderr }
    }
}

impl std::fmt::Display for CommandOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Command: {}", self.command)?;

        let exit_code = self.exit_code.unwrap_or(-1);

        writeln!(
            f,
            "{}: Command exited with status code {}",
            mark(self.success),
            exit_code
        )?;

        writeln!(f)?;

        if !self.stdout.trim().is_empty() {
            writeln!(f, "== stdout ==")?;
            writeln!(f, "{}", self.stdout)?;
            writeln!(f, "== end of stdout ==")?;
            writeln!(f)?;
        }

        if !self.stderr.trim().is_empty() {
            writeln!(f, "== stderr ==")?;
            writeln!(f, "{}", self.stderr)?;
            writeln!(f, "== end of stderr ==")?;
            writeln!(f)?;
        }

        Ok(())
    }
}
