use std::error::Error;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use libtest_mimic::{Arguments, Trial};
use minijinja::Environment;
use tfmttools_fs::PathIterator;

use crate::context::{SourceDirs, TestContext};
use crate::data::{Expectation, TestCaseData};
use crate::outcome::{CommandOutcome, TestCaseOutcome, TestOutcome};

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
                let outcome = run_test_case(name.clone(), data)?;

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

    create_test_reports(failed_test_outcomes)?;

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
    test_case_data: TestCaseData,
) -> Result<TestCaseOutcome> {
    let context = TestContext::new()?;

    populate_files(&context)?;

    let mut test_outcomes = Vec::new();

    for (test_name, test_data) in test_case_data.tests() {
        let previous_expectation = test_data
            .previous_expectation()
            .map(|expectation_name| {
                test_case_data.expectations().get(expectation_name).ok_or(
                    eyre!("No expectation with name '{}'", expectation_name),
                )
            })
            .transpose()?;

        let expectation = test_data
            .expectation()
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

        let (missing_files, remaining_files) = verify_expectations(
            &context,
            expectation.map(|v| &**v),
            previous_expectation.map(|v| &**v),
        );

        let test_outcome = TestOutcome::new(
            test_name.clone(),
            command_outcome,
            remaining_files,
            missing_files,
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
    copy_files(SourceDirs::template_dir(), context.config_work_dir())?;

    copy_files(SourceDirs::files_dir(), context.input_work_dir())?;

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
    cmd.current_dir(context.work_dir_path());

    for arg in arguments.split_whitespace() {
        cmd.arg(arg);
    }

    let output = cmd.output()?;

    Ok(CommandOutcome::new(arguments, output))
}

fn get_fixed_arguments(context: &TestContext) -> String {
    format!(
        "--config-directory {} --run-id {}",
        context.config_work_dir(),
        TEST_RUN_ID
    )
}

fn verify_expectations(
    context: &TestContext,
    expectation: Option<&[Expectation]>,
    previous_expectation: Option<&[Expectation]>,
) -> (Option<Vec<Utf8PathBuf>>, Option<Vec<Utf8PathBuf>>) {
    let remaining_files = previous_expectation.map(|previous_expectation| {
        get_still_existing_file_paths(
            &context.work_dir_path(),
            previous_expectation,
        )
    });

    let missing_files = expectation.map(|expectation| {
        get_missing_file_paths(&context.work_dir_path(), expectation)
    });

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

fn create_test_reports(
    failed_test_outcomes: Vec<TestCaseOutcome>,
) -> Result<()> {
    let template =
        fs_err::read_to_string(SourceDirs::test_report_template_path())?;

    let mut environment = Environment::new();
    environment.add_template_owned("report", template)?;

    let template = environment.get_template("report")?;

    fs_err::create_dir_all(SourceDirs::test_report_output_dir())?;

    for outcome in failed_test_outcomes {
        let rendered = template.render(&outcome)?;

        fs_err::write(
            SourceDirs::test_report_output_dir()
                .join(format!("{}.report.html", outcome.name())),
            rendered,
        )?
    }

    Ok(())
}
