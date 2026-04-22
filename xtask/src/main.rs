use std::env;
use std::process::{Command, ExitCode};

const HELP: &str = "\
Usage: cargo xtask <task>

Tasks:
    check             cargo check --workspace
    test              cargo test --workspace
    test-core         cargo test -p tfmttools-core
    test-fs           cargo test -p tfmttools-fs
    test-cli          cargo test -p tfmttools-cli
    test-integration  cargo test -p tfmttools-cli --test integration -- --nocapture
    lint              cargo +nightly fmt --all --check
                      cargo +nightly clippy --workspace --all-targets
";
const TEST_INTEGRATION_ARGS: &[&str] = &[
    "test",
    "-p",
    "tfmttools-cli",
    "--test",
    "integration",
    "--",
    "--nocapture",
];
const FMT_ARGS: &[&str] = &["+nightly", "fmt", "--all", "--check"];
const CLIPPY_ARGS: &[&str] =
    &["+nightly", "clippy", "--workspace", "--all-targets"];

fn main() -> ExitCode {
    let Some(task) = env::args().nth(1) else {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    };

    match task.as_str() {
        "check" => run_cargo(&["check", "--workspace"]),
        "test" => run_cargo(&["test", "--workspace"]),
        "test-core" => run_cargo(&["test", "-p", "tfmttools-core"]),
        "test-fs" => run_cargo(&["test", "-p", "tfmttools-fs"]),
        "test-cli" => run_cargo(&["test", "-p", "tfmttools-cli"]),
        "test-integration" => run_cargo(TEST_INTEGRATION_ARGS),
        "lint" => run_steps(&[CLIPPY_ARGS, FMT_ARGS]),
        "help" | "--help" | "-h" => {
            print!("{HELP}");
            ExitCode::SUCCESS
        },
        unknown => {
            eprintln!("unknown xtask task: {unknown}\n");
            eprint!("{HELP}");
            ExitCode::FAILURE
        },
    }
}

fn run_steps(steps: &[&[&str]]) -> ExitCode {
    for step in steps {
        let code = run_cargo(step);
        if code != ExitCode::SUCCESS {
            return code;
        }
    }

    ExitCode::SUCCESS
}

fn run_cargo(args: &[&str]) -> ExitCode {
    let status = Command::new("cargo").args(args).status();

    match status {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            if let Some(code) = status.code() {
                eprintln!("cargo exited with status code {code}");
            } else {
                eprintln!("cargo was terminated by a signal");
            }

            ExitCode::FAILURE
        },
        Err(error) => {
            eprintln!("failed to run cargo: {error}");
            ExitCode::FAILURE
        },
    }
}
