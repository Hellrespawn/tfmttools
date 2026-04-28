use std::error::Error;
use std::process::ExitCode;

use tfmttools_test_cli::test_runner;

fn main() -> Result<ExitCode, Box<dyn Error>> {
    test_runner()
}
