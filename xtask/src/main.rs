use std::env;
use std::process::{Command, ExitCode};

const HELP: &str = "\
Usage: cargo xtask <task>

Tasks:
    check             cargo check --workspace
    test              cargo test --workspace --exclude tfmttools-cli
                      cargo test -p tfmttools-cli --bin tfmt
                      cargo test -p tfmttools-cli --test integration -- --nocapture
    test-core         cargo test -p tfmttools-core
    test-fs           cargo test -p tfmttools-fs
    test-cli          cargo test -p tfmttools-cli --bin tfmt
                      cargo test -p tfmttools-cli --test integration -- --nocapture
    test-integration  cargo test -p tfmttools-cli --test integration -- --nocapture
    test-container    TFMT_CONTAINER_REQUIRED=1 cargo test -p tfmttools-cli --test container -- --nocapture
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
const TEST_CONTAINER_ARGS: &[&str] = &[
    "test",
    "-p",
    "tfmttools-cli",
    "--test",
    "container",
    "--",
    "--nocapture",
];
const TEST_WORKSPACE_ARGS: &[&str] =
    &["test", "--workspace", "--exclude", "tfmttools-cli"];
const TEST_CLI_BIN_ARGS: &[&str] =
    &["test", "-p", "tfmttools-cli", "--bin", "tfmt"];
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
        "test" => {
            run_steps(&[
                TEST_WORKSPACE_ARGS,
                TEST_CLI_BIN_ARGS,
                TEST_INTEGRATION_ARGS,
            ])
        },
        "test-core" => run_cargo(&["test", "-p", "tfmttools-core"]),
        "test-fs" => run_cargo(&["test", "-p", "tfmttools-fs"]),
        "test-cli" => run_steps(&[TEST_CLI_BIN_ARGS, TEST_INTEGRATION_ARGS]),
        "test-integration" => run_cargo(TEST_INTEGRATION_ARGS),
        "test-container" => {
            run_cargo_with_env(TEST_CONTAINER_ARGS, &[(
                "TFMT_CONTAINER_REQUIRED",
                "1",
            )])
        },
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
    run_cargo_with_env(args, &[])
}

fn run_cargo_with_env(args: &[&str], env: &[(&str, &str)]) -> ExitCode {
    let mut command = Command::new("cargo");
    command.args(args);

    for (name, value) in env {
        command.env(name, value);
    }

    let status = command.status();

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
