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
    test-container    cargo test -p tfmttools-cli --test container -- --nocapture
    lint              cargo +nightly fmt --all --check
                      cargo +nightly clippy --workspace --all-targets
    serve-reports     python -m http.server (from tests/reports/, passes through arguments)
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
    let mut args = env::args().skip(1);
    let Some(task) = args.next() else {
        print!("{HELP}");
        return ExitCode::SUCCESS;
    };
    let trailing_args = args.collect::<Vec<_>>();

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
        "test-integration" => {
            run_cargo(&test_args_with_trailing(TEST_INTEGRATION_ARGS, &trailing_args))
        },
        "test-container" => {
            run_cargo(&test_args_with_trailing(TEST_CONTAINER_ARGS, &trailing_args))
        },
        "lint" => run_steps(&[CLIPPY_ARGS, FMT_ARGS]),
        "serve-reports" => run_reports_server(&trailing_args),
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

fn test_args_with_trailing<'a>(
    base: &'a [&'a str],
    trailing_args: &'a [String],
) -> Vec<&'a str> {
    let mut split_index = base.len();
    for (index, value) in base.iter().enumerate() {
        if *value == "--" {
            split_index = index;
            break;
        }
    }

    let mut args = base[..split_index].to_vec();
    if !trailing_args.is_empty() {
        args.extend(trailing_args.iter().map(String::as_str));
    }
    if split_index < base.len() {
        args.push("--");
        args.extend(base[(split_index + 1)..].iter().copied());
    }

    args
}

fn run_cargo_with_env(args: &[&str], env: &[(&str, &str)]) -> ExitCode {
    let mut command = Command::new("cargo");
    command.args(args);

    for (name, value) in env {
        command.env(name, value);
    }

    for (name, value) in env {
        command.env(name, value);
    }

    run_command(command, "cargo")
}

fn run_reports_server(args: &[String]) -> ExitCode {
    let mut command = Command::new("python");
    command.args(["-m", "http.server"]);
    command.args(args);
    command.current_dir("tests/reports");

    run_command(command, "python")
}

fn run_command(mut command: Command, program_name: &str) -> ExitCode {
    let status = command.status();

    match status {
        Ok(status) if status.success() => ExitCode::SUCCESS,
        Ok(status) => {
            if let Some(code) = status.code() {
                eprintln!("{program_name} exited with status code {code}");
            } else {
                eprintln!("{program_name} was terminated by a signal");
            }

            ExitCode::FAILURE
        },
        Err(error) => {
            eprintln!("failed to run {program_name}: {error}");
            ExitCode::FAILURE
        },
    }
}
