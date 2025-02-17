use crate::file_tree::FileTreeNode;

#[derive(Debug)]
pub struct CommandOutput {
    pub action_name: String,
    pub invocation: String,
    pub exit_code: i32,
    pub file_tree_before: FileTreeNode,
    pub file_tree_after: FileTreeNode,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub struct TestResult {
    pub test_case_name: String,
    pub command_output: CommandOutput,
    pub test_result_type: TestResultType,
}

impl TestResult {
    pub fn is_failure(&self) -> bool {
        matches!(self.test_result_type, TestResultType::Failure(_))
    }
}

impl std::fmt::Display for TestResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.test_result_type {
            TestResultType::Success => {
                write!(f, "✔️ {} was successful", self.test_case_name)?
            },
            TestResultType::Failure(message) => {
                let CommandOutput {
                    action_name,
                    invocation,
                    exit_code,
                    file_tree_before,
                    file_tree_after,
                    stdout,
                    stderr,
                } = &self.command_output;

                writeln!(
                    f,
                    "❌ {} has failed at action {}.\n{}",
                    self.test_case_name, action_name, message
                )?;
                writeln!(f)?;
                writeln!(
                    f,
                    "Command {} exited with {}",
                    invocation, exit_code
                )?;

                if !stdout.is_empty() {
                    writeln!(f, "[ stdout ]")?;
                    writeln!(f, "{stdout}")?;
                    writeln!(f, "[ end of stdout ]")?;
                }

                if !stderr.is_empty() {
                    writeln!(f, "[ stderr ]")?;
                    writeln!(f, "{stderr}")?;
                    writeln!(f, "[ end of stderr ]")?;
                }

                if file_tree_before != file_tree_after {
                    writeln!(f, "[ file tree before ]")?;
                    writeln!(f, "{file_tree_before}")?;
                    writeln!(f, "[ end of file tree before ]")?;
                    writeln!(f)?;

                    writeln!(f, "[ file tree after ]")?;
                    writeln!(f, "{file_tree_after}")?;
                    writeln!(f, "[ end of file tree after ]")?;
                }
            },
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum TestResultType {
    Success,
    Failure(String),
}
