use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use assert_cmd::Command;
use camino::{Utf8Path, Utf8PathBuf};
use chrono::{SecondsFormat, Utc};
use color_eyre::Result;
use color_eyre::eyre::eyre;
use libtest_mimic::{Arguments, Trial};
use tfmttools_fs::{PathIterator, get_path_checksum};
use tfmttools_test_harness::{
    CaseOutcome, CliCaseDetails, CliRunDetails, CommandOutcome, Expectation,
    ExpectationOutcome, ExpectationsOutcome, FixtureDirs, ReportEnvelope,
    ReportFilters, Runner, RunnerDetails, StepOutcome, write_report,
};

use crate::case::{TestContext, populate_files};

const TEST_RUN_ID: &str = "run_id";
const SKIP_PREVIOUS_STEP_FAILED: &str = "previous_step_failed";

pub fn test_runner() -> Result<ExitCode, Box<dyn Error>> {
    let run_started = Instant::now();
    let started_at = timestamp();
    let args = Arguments::from_args();
    let harness_argv = env::args().collect::<Vec<_>>();
    let fixture_dirs = FixtureDirs::cli();

    let data = load_data(&fixture_dirs)?;
    let data = apply_filters(data, &args);
    let filters =
        ReportFilters::new(args.filter.clone(), args.skip.clone(), args.exact);

    let mutex = Arc::new(Mutex::new(Vec::new()));

    let tests = data
        .into_iter()
        .map(|(name, data)| {
            let mutex = mutex.clone();
            let fixture_dirs = FixtureDirs::cli();

            Trial::test(name.clone(), move || {
                let outcome =
                    run_test_case(name.clone(), &data, &fixture_dirs)?;
                let passed = outcome.passed();

                {
                    mutex.lock()?.push(outcome);
                }

                if passed {
                    Ok(())
                } else {
                    let error_message = format!("Failed test case {name}");

                    Err(error_message.into())
                }
            })
        })
        .collect::<Vec<_>>();

    let exit_code = libtest_mimic::run(&args, tests).exit_code();

    let mut test_outcomes =
        Arc::into_inner(mutex).expect("Arc dropped").into_inner()?;
    test_outcomes.sort_by(|left, right| left.name().cmp(right.name()));

    let report_dir = FixtureDirs::reports_dir();
    fs_err::create_dir_all(&report_dir)?;
    let canonical_report_dir = report_dir.canonicalize_utf8().ok();
    let report = ReportEnvelope::new(
        Runner::Cli,
        started_at,
        timestamp(),
        run_started.elapsed().as_millis(),
        harness_argv,
        filters,
        harness_environment(),
        canonical_report_dir,
        test_outcomes,
        RunnerDetails::Cli(CliRunDetails::default()),
    );

    write_report(&report_dir, report)?;

    Ok(exit_code)
}

fn load_data(
    fixture_dirs: &FixtureDirs,
) -> Result<Vec<(String, tfmttools_test_harness::TestCaseData)>> {
    let test_cases_dir = fixture_dirs.case_dir();

    let test_case_iterator = PathIterator::single_directory(&test_cases_dir);

    let mut cases = test_case_iterator
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
                tfmttools_test_harness::TestCaseData::from_file(&path)?,
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    cases.sort_by(|(left, _), (right, _)| left.cmp(right));

    if cases.is_empty() {
        Err(eyre!("Did not find any testcases at {}", test_cases_dir))
    } else {
        Ok(cases)
    }
}

fn apply_filters(
    cases: Vec<(String, tfmttools_test_harness::TestCaseData)>,
    args: &Arguments,
) -> Vec<(String, tfmttools_test_harness::TestCaseData)> {
    cases
        .into_iter()
        .filter(|(name, _)| {
            let filter_matches = args
                .filter
                .as_ref()
                .is_none_or(|filter| matches_filter(name, filter, args.exact));
            let skip_matches = args
                .skip
                .iter()
                .any(|skip| matches_filter(name, skip, args.exact));

            filter_matches && !skip_matches
        })
        .collect()
}

fn matches_filter(name: &str, filter: &str, exact: bool) -> bool {
    if exact { name == filter } else { name.contains(filter) }
}

fn run_test_case(
    test_case_name: String,
    test_case_data: &tfmttools_test_harness::TestCaseData,
    fixture_dirs: &FixtureDirs,
) -> Result<CaseOutcome> {
    let case_started = Instant::now();
    let context = TestContext::new()?;

    populate_files(fixture_dirs, &context)?;

    let mut step_outcomes = Vec::new();
    let mut previous_step_failed = false;

    for (test_name, test_data) in test_case_data.tests() {
        if previous_step_failed {
            step_outcomes.push(StepOutcome::skipped(
                test_name.clone(),
                SKIP_PREVIOUS_STEP_FAILED,
            ));
            continue;
        }

        let step_outcome = run_test_step(
            test_name.clone(),
            test_case_data,
            test_data,
            &context,
        )?;
        previous_step_failed = !step_outcome.passed();
        step_outcomes.push(step_outcome);
    }

    let test_case_outcome = CaseOutcome::new(
        test_case_name,
        test_case_data.description().to_owned(),
        case_started.elapsed().as_millis(),
        step_outcomes,
        Some(CliCaseDetails::new(context.work_dir_path())),
    );

    context.persist_work_dir_if(!test_case_outcome.passed());

    Ok(test_case_outcome)
}

fn run_test_step(
    test_name: String,
    test_case_data: &tfmttools_test_harness::TestCaseData,
    test_data: &tfmttools_test_harness::TestData,
    context: &TestContext,
) -> Result<StepOutcome> {
    let step_started = Instant::now();
    let previous_expectations = test_data
        .previous_expectations()
        .map(|expectation_name| {
            test_case_data
                .expectations()
                .get(expectation_name)
                .ok_or(eyre!("No expectation with name '{}'", expectation_name))
        })
        .transpose()?;

    let expectation = test_data
        .expectations()
        .map(|expectation_name| {
            test_case_data
                .expectations()
                .get(expectation_name)
                .ok_or(eyre!("No expectation with name '{}'", expectation_name))
        })
        .transpose()?;

    let command_outcome = test_data
        .command()
        .map(|command| run_command(context, command))
        .transpose()?;

    let expectations_outcome = verify_expectations(
        context,
        previous_expectations.map(|v| &**v),
        expectation.map(|v| &**v),
    );

    Ok(StepOutcome::new(
        test_name,
        step_started.elapsed().as_millis(),
        command_outcome,
        expectations_outcome,
    ))
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
        let checksum = get_path_checksum(&path).expect("");

        if expectation.verify_checksum(&checksum) {
            ExpectationOutcome::Ok(path)
        } else {
            ExpectationOutcome::ChecksumMismatch {
                path,
                expected: expectation.checksum().unwrap().clone(),
                actual: checksum,
            }
        }
    } else {
        ExpectationOutcome::NotPresent(path)
    }
}

fn harness_environment() -> BTreeMap<String, String> {
    env::vars()
        .filter(|(name, _)| {
            name.starts_with("TFMT_CONTAINER_")
                || name == "RUST_TEST_THREADS"
                || name == "RUST_TEST_NOCAPTURE"
        })
        .collect()
}

fn timestamp() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}
