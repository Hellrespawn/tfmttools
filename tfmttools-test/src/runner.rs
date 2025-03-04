use std::error::Error;
use std::process::ExitCode;

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use libtest_mimic::{Arguments, Trial};
use tfmttools_fs::PathIterator;

use crate::context::{SourceDirs, TestContext};
use crate::data::{Expectation, TestCaseData};
use crate::outcome::{CommandOutcome, TestCaseOutcome, TestOutcome};

const TEST_DATA_DIRECTORY: &str = "../testdata";
const INITIAL_EXPECTATION_NAME: &str = "initial-state";
const TEST_RUN_ID: &str = "run_id";

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

            Trial::test(name.clone(), || {
                let outcome = run_test_case(source_dirs, name, data)?;

                if outcome.passed() { Ok(()) } else { Err(outcome.into()) }
            })
        })
        .collect::<Vec<_>>()
}

fn run_test_case(
    source_dirs: SourceDirs,
    name: String,
    test_case_data: TestCaseData,
) -> Result<TestCaseOutcome> {
    let context = TestContext::new(source_dirs)?;

    populate_files(&context)?;

    let mut test_case_outcome = TestCaseOutcome::new(
        name,
        test_case_data.description().to_owned(),
        context.work_dir().path(),
    );

    let mut previous_expectation =
        test_case_data.expectations().get(INITIAL_EXPECTATION_NAME);

    *test_case_outcome.missing_files_mut() =
        previous_expectation.map(|initial_state| {
            let (missing_files, _) =
                verify_expectations(&context, initial_state, None);

            missing_files
        });

    if test_case_outcome.passed_initial_expectation() {
        for (name, test_data) in test_case_data.tests() {
            let mut test_outcome = TestOutcome::new(name.clone());

            if let Some(command) = test_data.command() {
                *test_outcome.command_outcome_mut() =
                    Some(run_command(&context, command)?);
            }

            let expectation_name = test_data.expectation();

            let expectation =
                test_case_data.expectations().get(expectation_name).ok_or(
                    eyre!("No expectation with name '{}'", expectation_name),
                )?;

            let (missing_files, remaining_files) = verify_expectations(
                &context,
                expectation,
                previous_expectation.map(|v| &**v),
            );

            *test_outcome.missing_files_mut() = missing_files;
            *test_outcome.remaining_files_mut() = remaining_files;

            let passed = test_outcome.passed();

            test_case_outcome.test_outcomes_mut().push(test_outcome);

            if !passed {
                break;
            }

            previous_expectation = Some(expectation);
        }
    }

    context.persist_work_dir_if(!test_case_outcome.passed());

    Ok(test_case_outcome)
}

fn populate_files(context: &TestContext) -> Result<()> {
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

fn run_command(context: &TestContext, command: &str) -> Result<CommandOutcome> {
    let arguments = format!("{} {}", get_fixed_arguments(context), command);

    let mut cmd = Command::cargo_bin("tfmt").unwrap();
    cmd.current_dir(context.work_dir().path());

    for arg in arguments.split_whitespace() {
        cmd.arg(arg);
    }

    let output = cmd.output()?;

    Ok(CommandOutcome::new(arguments, output))
}

fn get_fixed_arguments(context: &TestContext) -> String {
    format!(
        "--config-directory {} --run-id {}",
        context.work_dir().config_dir(),
        TEST_RUN_ID
    )
}

fn verify_expectations(
    context: &TestContext,
    expectation: &[Expectation],
    previous_expectation: Option<&[Expectation]>,
) -> (Vec<Utf8PathBuf>, Option<Vec<Utf8PathBuf>>) {
    let remaining_files = previous_expectation.map(|previous_expectation| {
        get_still_existing_file_paths(
            &context.work_dir().path(),
            previous_expectation,
        )
    });

    let missing_files =
        get_missing_file_paths(&context.work_dir().path(), expectation);

    (missing_files, remaining_files)
}

fn get_still_existing_file_paths(
    prefix: &Utf8Path,
    expectation: &[Expectation],
) -> Vec<Utf8PathBuf> {
    expectation
        .iter()
        .filter(|expectation| !expectation.verify_no_longer_exists(prefix))
        .map(|e| e.path().to_owned())
        .collect()
}

fn get_missing_file_paths(
    prefix: &Utf8Path,
    expectation: &[Expectation],
) -> Vec<Utf8PathBuf> {
    expectation
        .iter()
        .filter(|expectation| !expectation.verify_exists(prefix))
        .map(|e| e.path().to_owned())
        .collect()
}
