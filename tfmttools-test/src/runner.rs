use std::error::Error;
use std::process::ExitCode;

use libtest_mimic::{Arguments, Trial};

use crate::test_case::TestCase;

pub fn test_runner() -> Result<ExitCode, Box<dyn Error>> {
    let args = Arguments::from_args();
    let tests = collect_tests()?;
    Ok(libtest_mimic::run(&args, tests).exit_code())
}

fn collect_tests() -> Result<Vec<Trial>, Box<dyn Error>> {
    let cases = TestCase::load_all()?;

    let trials = cases
        .into_iter()
        .map(|case| {
            Trial::test(case.name.clone(), move || {
                case.run_test().map_err(|tr| tr.into())
            })
        })
        .collect::<Vec<_>>();

    Ok(trials)
}
