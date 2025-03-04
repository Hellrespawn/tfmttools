use std::error::Error;
use std::process::ExitCode;

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use libtest_mimic::{Arguments, Trial};
use tfmttools_fs::PathIterator;

use crate::context::{SourceDirs, TestContext};
use crate::data::TestCaseData;
use crate::{TEST_DATA_DIRECTORY, TEST_RUN_ID};

const INITIAL_STATE_REFERENCE_NAME: &str = "initial-state";

pub fn test_runner() -> Result<ExitCode, Box<dyn Error>> {
    let args = Arguments::from_args();

    let source_dirs = SourceDirs::new(TEST_DATA_DIRECTORY);

    let data = load_data(&source_dirs)?;
    let tests = generate_tests(&source_dirs, data);

    Ok(libtest_mimic::run(&args, tests).exit_code())
}

fn load_data(source_dirs: &SourceDirs) -> Result<Vec<(String, TestCaseData)>> {
    let test_cases_dir = source_dirs.test_case_dir();

    let test_case_iterator = PathIterator::single_directory(&test_cases_dir);

    test_case_iterator
        .flatten()
        .filter(|p| {
            let component = p.components().last().expect(
                "iterator of path components should always have one element",
            );

            component.as_str().ends_with(".case.json")
        })
        .map(|path| {
            Ok((
                path.file_name().unwrap().to_owned(),
                TestCaseData::from_file(&path)?,
            ))
        })
        .collect()
}

fn generate_tests(
    source_dirs: &SourceDirs,
    data: Vec<(String, TestCaseData)>,
) -> Vec<Trial> {
    data.into_iter()
        .map(|(name, data)| {
            let source_dirs = source_dirs.clone();

            Trial::test(name, || {
                run_test_case(source_dirs, data).map_err(|tr| tr.into())
            })
        })
        .collect::<Vec<_>>()
}

fn run_test_case(
    source_dirs: SourceDirs,
    test_case_data: TestCaseData,
) -> Result<()> {
    let mut context = TestContext::new(source_dirs)?;

    populate_files(&mut context)?;

    let mut previous_expectation =
        test_case_data.reference().get(INITIAL_STATE_REFERENCE_NAME);

    if let Some(initial_state) = previous_expectation {
        verify_expectations(&mut context, initial_state, None)?;
    }

    println!("Verified initial state.");

    for (name, test_data) in test_case_data.tests() {
        println!("Running test {}...", name);

        if let Some(command) = test_data.command() {
            run_command(&mut context, command)?;
        }

        println!("Ran command...");

        let expectation_name = test_data.expectation();

        let expectation = test_case_data
            .reference()
            .get(expectation_name)
            .ok_or(eyre!("No reference with name '{}'", expectation_name))?;

        verify_expectations(
            &mut context,
            expectation,
            previous_expectation.map(|v| &**v),
        )?;

        println!("Verified expectation...");

        previous_expectation = Some(expectation);

        println!("Done with {}.", name)
    }

    Ok(())
}

fn populate_files(context: &mut TestContext) -> Result<()> {
    copy_files(
        context.source_dirs().template_dir(),
        context.work_dir().config_dir(),
    )?;

    copy_files(
        context.source_dirs().files_dir(),
        context.work_dir().input_dir(),
    )?;

    Ok(())
}

fn copy_files(source_dir: Utf8PathBuf, target_dir: Utf8PathBuf) -> Result<()> {
    let paths = fs_err::read_dir(source_dir)?
        .flat_map(|result| {
            result.map(|entry| {
                Utf8PathBuf::from_path_buf(entry.path().to_path_buf())
            })
        })
        .flatten()
        .collect::<Vec<_>>();

    fs_err::create_dir(&target_dir)?;

    for path in &paths {
        // Templates are selected by is_file, should always have a filename
        // so path.file_name().unwrap() should be safe.
        let file_name = path.file_name().unwrap();

        fs_err::copy(path, target_dir.join(file_name))?;
    }

    Ok(())
}

fn run_command(context: &mut TestContext, command: &str) -> Result<()> {
    let arguments = format!("{} {}", get_fixed_arguments(context), command);

    let mut cmd = Command::cargo_bin("tfmt").unwrap();
    cmd.current_dir(context.work_dir().path());

    for arg in arguments.split_whitespace() {
        cmd.arg(arg);
    }

    let result = cmd.output();

    match result {
        Ok(output) => println!("{}", String::from_utf8_lossy(&output.stdout)),
        Err(err) => println!("{}", err),
    }

    Ok(())
}

fn get_fixed_arguments(context: &TestContext) -> String {
    format!(
        "--config-directory {} --run-id {}",
        context.work_dir().config_dir(),
        TEST_RUN_ID
    )
}

fn verify_expectations(
    context: &mut TestContext,
    reference: &[String],
    previous_reference: Option<&[String]>,
) -> Result<()> {
    if let Some(previous_reference) = previous_reference {
        let remaining_files = get_still_existing_file_paths(
            &context.work_dir().path(),
            previous_reference,
        );

        if !remaining_files.is_empty() {
            return Err(eyre!(
                "Files expected to be moved are still in place:\n{}",
                remaining_files.join("\n")
            ));
        }
    }

    let missing_files =
        get_missing_file_paths(&context.work_dir().path(), reference);

    if !missing_files.is_empty() {
        return Err(eyre!(
            "Files expected to be moved are not there:\n{}",
            missing_files.join("\n")
        ));
    }

    Ok(())
}

struct Expectation {
    path: Utf8PathBuf,
    no_previous: bool,
}

impl Expectation {
    fn new(string: &str, path: &Utf8Path) -> Expectation {
        let mut iter = string.split(':');

        let suffix = iter.next().unwrap().to_owned();

        let mut expectation =
            Expectation { path: path.join(suffix), no_previous: false };

        for option in iter {
            match option {
                "noprevious" => expectation.no_previous = true,
                other => panic!("Unknown expectation option '{}'", other),
            }
        }

        expectation
    }

    fn verify_exists(&self) -> bool {
        self.path.exists()
    }

    fn verify_no_longer_exists(&self) -> bool {
        self.no_previous || !self.path.exists()
    }

    fn path_to_string(&self) -> String {
        self.path.to_string()
    }
}

fn get_still_existing_file_paths(
    prefix: &Utf8Path,
    reference: &[String],
) -> Vec<String> {
    reference
        .iter()
        .map(|string| Expectation::new(string, prefix))
        .filter(|expectation| !expectation.verify_no_longer_exists())
        .map(|expectation| expectation.path_to_string())
        .collect()
}

fn get_missing_file_paths(
    prefix: &Utf8Path,
    reference: &[String],
) -> Vec<String> {
    reference
        .iter()
        .map(|string| Expectation::new(string, prefix))
        .filter(|expectation| !expectation.verify_exists())
        .map(|expectation| expectation.path_to_string())
        .collect()
}
