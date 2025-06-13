use tfmttools_core::error::{TFMTError, TFMTResult};

use crate::action::Action;

pub struct TransactionError {
    execution_error: TFMTError,
    rollback_error: Option<TFMTError>,
}

pub struct Transaction {
    actions: Vec<Box<dyn Action>>,
    execution_count: usize,
}

impl Transaction {
    pub fn new() -> Self {
        Self { actions: Vec::new(), execution_count: 0 }
    }

    pub fn add_action(&mut self, action: Box<dyn Action>) {
        self.actions.push(action);
    }

    pub fn add_actions(&mut self, actions: Vec<Box<dyn Action>>) {
        self.actions.extend(actions);
    }

    pub fn run(&mut self) -> TFMTResult<(), TransactionError> {
        let result = self.execute();

        if let Err(execution_error) = result {
            let rollback_error = self.rollback().err();

            Err(TransactionError { execution_error, rollback_error })
        } else {
            Ok(())
        }
    }

    fn execute(&mut self) -> TFMTResult {
        for action in &self.actions {
            action.apply()?;
            self.execution_count += 1;
        }

        Ok(())
    }

    fn rollback(&mut self) -> TFMTResult {
        for i in (0..self.execution_count).rev() {
            self.actions[i].undo()?;
            self.execution_count -= 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use assert_fs::TempDir;
    use assert_fs::fixture::ChildPath;
    use assert_fs::prelude::*;
    use camino::Utf8PathBuf;
    use color_eyre::Result;
    use predicates::prelude::*;

    use super::*;
    use crate::action::CopyFile;

    struct TestDir {
        temp_dir: TempDir,
    }

    impl TestDir {
        const FILE_NAMES: [&str; 3] =
            ["file_a.txt", "file_b.txt", "file_c.txt"];

        fn new() -> Result<Self> {
            Ok(Self { temp_dir: TempDir::new()? })
        }

        fn files_dir(&self) -> ChildPath {
            self.temp_dir.child("files")
        }

        fn target_dir(&self) -> ChildPath {
            self.temp_dir.child("target")
        }

        fn backup_dir(&self) -> ChildPath {
            self.temp_dir.child("backup")
        }

        fn test_files(&self) -> Vec<ChildPath> {
            Self::FILE_NAMES
                .iter()
                .map(|name| self.files_dir().child(name))
                .collect()
        }

        fn create_source_and_target_for_filename(
            &self,
            filename: &str,
        ) -> (ChildPath, ChildPath) {
            let source_file = self.files_dir().child(filename);

            let target_file = self.target_dir().child(filename);

            (source_file, target_file)
        }
    }

    fn setup_test() -> Result<TestDir> {
        let test_dir = TestDir::new()?;

        fs_err::create_dir(test_dir.files_dir().path())?;
        fs_err::create_dir(test_dir.backup_dir().path())?;
        fs_err::create_dir(test_dir.target_dir().path())?;

        for file in test_dir.test_files() {
            file.touch()?;
        }

        Ok(test_dir)
    }

    #[test]
    fn test_copy() -> Result<()> {
        let test_dir = setup_test()?;

        let mut tr = Transaction::new();

        let (source_file_a, target_file_a) = test_dir
            .create_source_and_target_for_filename(TestDir::FILE_NAMES[0]);

        source_file_a.assert(predicate::path::exists());
        target_file_a.assert(predicate::path::missing());

        let (source_file_b, target_file_b) = test_dir
            .create_source_and_target_for_filename(TestDir::FILE_NAMES[1]);

        source_file_b.assert(predicate::path::exists());
        target_file_b.assert(predicate::path::missing());

        let action_a = CopyFile::new(
            Utf8PathBuf::try_from(source_file_a.path().to_owned())?,
            Utf8PathBuf::try_from(target_file_a.path().to_owned())?,
        );

        let action_b = CopyFile::new(
            Utf8PathBuf::try_from(source_file_b.path().to_owned())?,
            Utf8PathBuf::try_from(target_file_b.path().to_owned())?,
        );

        tr.add_action(Box::new(action_a));
        tr.add_action(Box::new(action_b));

        tr.execute()?;

        source_file_a.assert(predicate::path::exists());
        target_file_a.assert(predicate::path::exists());
        source_file_b.assert(predicate::path::exists());
        target_file_b.assert(predicate::path::exists());

        tr.rollback()?;

        source_file_a.assert(predicate::path::exists());
        target_file_a.assert(predicate::path::missing());
        source_file_b.assert(predicate::path::exists());
        target_file_b.assert(predicate::path::missing());

        Ok(())
    }

    #[test]
    fn test_copy_with_error() -> Result<()> {
        let test_dir = setup_test()?;

        let mut tr = Transaction::new();

        let (source_file_a, target_file_a) = test_dir
            .create_source_and_target_for_filename(TestDir::FILE_NAMES[0]);

        source_file_a.assert(predicate::path::exists());
        target_file_a.assert(predicate::path::missing());

        let (source_file_b, _) = test_dir
            .create_source_and_target_for_filename(TestDir::FILE_NAMES[1]);
        let target_file_b = test_dir
            .target_dir()
            .child("missing")
            .child(TestDir::FILE_NAMES[1]);

        source_file_b.assert(predicate::path::exists());
        target_file_b.assert(predicate::path::missing());

        let action_a = CopyFile::new(
            Utf8PathBuf::try_from(source_file_a.path().to_owned())?,
            Utf8PathBuf::try_from(target_file_a.path().to_owned())?,
        );

        let action_b = CopyFile::new(
            Utf8PathBuf::try_from(source_file_b.path().to_owned())?,
            Utf8PathBuf::try_from(target_file_b.path().to_owned())?,
        );

        tr.add_action(Box::new(action_a));
        tr.add_action(Box::new(action_b));

        let result = tr.run();

        assert!(result.is_err());

        let TransactionError { execution_error, rollback_error } =
            result.unwrap_err();

        if let Some(error) = &rollback_error {
            eprintln!("Rollback error:\n{error}");
        }

        assert!(rollback_error.is_none());

        source_file_a.assert(predicate::path::exists());
        target_file_a.assert(predicate::path::missing());
        source_file_b.assert(predicate::path::exists());
        target_file_b.assert(predicate::path::missing());

        Ok(())
    }
}
