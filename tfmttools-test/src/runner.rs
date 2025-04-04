use std::error::Error;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use libtest_mimic::{Arguments, Trial};
use minijinja::{Environment, context};
use tfmttools_fs::{PathIterator, get_file_checksum};

use crate::context::{SourceDirs, TestContext};
use crate::data::{Expectation, TestCaseData};
use crate::outcome::{
    CommandOutcome, ExpectationOutcome, ExpectationsOutcome, TestCaseOutcome,
    TestOutcome,
};

const TEST_RUN_ID: &str = "run_id";

pub fn test_runner() -> Result<ExitCode, Box<dyn Error>> {
    let args = Arguments::from_args();

    let data = load_data()?;

    let arc = Arc::new(Mutex::new(Vec::new()));

    let tests = data
        .into_iter()
        .map(|(name, data)| {
            let mutex = arc.clone();

            Trial::test(name.clone(), move || {
                let outcome = run_test_case(name.clone(), &data)?;

                if outcome.passed() {
                    Ok(())
                } else {
                    let error_message = format!("Failed test case {name}");

                    mutex.lock()?.push(outcome);

                    Err(error_message.into())
                }
            })
        })
        .collect::<Vec<_>>();

    let exit_code = libtest_mimic::run(&args, tests).exit_code();

    let failed_test_outcomes =
        Arc::into_inner(arc).expect("Arc dropped").into_inner()?;

    create_test_report(&failed_test_outcomes)?;

    Ok(exit_code)
}

fn load_data() -> Result<Vec<(String, TestCaseData)>> {
    let test_cases_dir = SourceDirs::test_case_dir();

    let test_case_iterator = PathIterator::single_directory(&test_cases_dir);

    test_case_iterator
        .flatten()
        .filter(|p| {
            let file_name = p.file_name().expect(
                "iterator of path components should always have one element",
            );

            file_name.ends_with(".case.json")
        })
        .map(|path| {
            Ok((
                path.file_name().unwrap().to_owned(),
                TestCaseData::from_file(&path)?,
            ))
        })
        .collect()
}
fn run_test_case(
    test_case_name: String,
    test_case_data: &TestCaseData,
) -> Result<TestCaseOutcome> {
    let context = TestContext::new()?;

    populate_files(&context)?;

    let mut test_outcomes = Vec::new();

    for (test_name, test_data) in test_case_data.tests() {
        let previous_expectations = test_data
            .previous_expectations()
            .map(|expectation_name| {
                test_case_data.expectations().get(expectation_name).ok_or(
                    eyre!("No expectation with name '{}'", expectation_name),
                )
            })
            .transpose()?;

        let expectation = test_data
            .expectations()
            .map(|expectation_name| {
                test_case_data.expectations().get(expectation_name).ok_or(
                    eyre!("No expectation with name '{}'", expectation_name),
                )
            })
            .transpose()?;

        let command_outcome = test_data
            .command()
            .map(|command| run_command(&context, command))
            .transpose()?;

        let expectations_outcome = verify_expectations(
            &context,
            previous_expectations.map(|v| &**v),
            expectation.map(|v| &**v),
        );

        let test_outcome = TestOutcome::new(
            test_name.clone(),
            command_outcome,
            expectations_outcome,
        );

        let passed = test_outcome.passed();

        test_outcomes.push(test_outcome);

        if !passed {
            break;
        }
    }

    let test_case_outcome = TestCaseOutcome::new(
        test_case_name,
        test_case_data.description().to_owned(),
        context.work_dir_path(),
        test_outcomes,
    );

    context.persist_work_dir_if(!test_case_outcome.passed());

    Ok(test_case_outcome)
}

fn populate_files(context: &TestContext) -> Result<()> {
    copy_files(SourceDirs::template_dir(), &context.config_work_dir())?;

    copy_files(SourceDirs::audio_dir(), &context.input_audio_dir())?;

    copy_files(SourceDirs::extra_dir(), &context.input_extra_dir())?;

    Ok(())
}

fn copy_files(source_dir: Utf8PathBuf, target_dir: &Utf8Path) -> Result<()> {
    let paths = fs_err::read_dir(source_dir)?
        .flat_map(|result| {
            result.map(|entry| Utf8PathBuf::from_path_buf(entry.path().clone()))
        })
        .flatten()
        .collect::<Vec<_>>();

    fs_err::create_dir(target_dir)?;

    for path in &paths {
        // Templates are selected by is_file, should always have a filename
        // so path.file_name().unwrap() should be safe.
        let file_name = path.file_name().unwrap();

        fs_err::copy(path, target_dir.join(file_name))?;
    }

    Ok(())
}

fn run_command(context: &TestContext, command: &str) -> Result<CommandOutcome> {
    let mut cmd = Command::cargo_bin("tfmt").unwrap();
    cmd.current_dir(context.work_dir_path());

    cmd.arg("--config-directory");
    cmd.arg(context.config_work_dir());
    cmd.arg("--run-id");
    cmd.arg(TEST_RUN_ID);

    for arg in command.split_whitespace() {
        cmd.arg(arg);
    }

    let output = cmd.output()?;
    let arguments =
        cmd.get_args().map(|arg| arg.to_string_lossy().to_string()).collect();

    Ok(CommandOutcome::new(arguments, &output))
}

fn verify_expectations(
    context: &TestContext,
    previous_expectations: Option<&[Expectation]>,
    expectations: Option<&[Expectation]>,
) -> ExpectationsOutcome {
    let prefix = context.work_dir_path();

    let remaining_files = previous_expectations.map(|previous_expectation| {
        get_still_existing_file_paths(&prefix, previous_expectation)
    });

    let outcomes = expectations
        .map(|expectations| {
            expectations
                .iter()
                .map(|expectation| verify_expectation(expectation, &prefix))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    ExpectationsOutcome::new(remaining_files, outcomes)
}

fn get_still_existing_file_paths(
    prefix: &Utf8Path,
    expectations: &[Expectation],
) -> Vec<Utf8PathBuf> {
    expectations
        .iter()
        .filter(|expectation| !expectation.verify_no_longer_exists(prefix))
        .map(|e| e.path().to_owned())
        .collect()
}
fn verify_expectation(
    expectation: &Expectation,
    prefix: &Utf8Path,
) -> ExpectationOutcome {
    let path = prefix.join(expectation.path());

    if path.exists() {
        let checksum = get_file_checksum(&path).expect("");

        if expectation.verify_checksum(&checksum) {
            ExpectationOutcome::Ok(path)
        } else {
            ExpectationOutcome::ChecksumMismatch {
                path,
                expected: expectation.checksum().unwrap().to_string(),
                actual: checksum,
            }
        }
    } else {
        ExpectationOutcome::NotPresent(path)
    }
}

fn create_test_report(failed_test_outcomes: &[TestCaseOutcome]) -> Result<()> {
    let _ = fs_err::remove_dir_all(SourceDirs::test_report_output_dir());

    if !failed_test_outcomes.is_empty() {
        let template =
            fs_err::read_to_string(SourceDirs::test_report_template_path())?;

        let mut environment = Environment::new();
        environment.add_template_owned("report", template)?;

        let template = environment.get_template("report")?;

        fs_err::create_dir_all(SourceDirs::test_report_output_dir())?;

        let rendered =
            template.render(context!(test_cases => failed_test_outcomes))?;

        let file_path =
            SourceDirs::test_report_output_dir().join("test-report.html");

        fs_err::write(&file_path, rendered)?;
    }

    Ok(())
}
