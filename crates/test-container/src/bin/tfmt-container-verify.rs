use std::collections::BTreeMap;
use std::env;
use std::process::ExitCode;

use camino::{Utf8Path, Utf8PathBuf};
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_fs::get_path_checksum;
use tfmttools_history::History;
use tfmttools_test_container::protocol::{
    ActionName, FilesystemExpectation, HistoryExpectation, VerifyOutcome,
    VerifyRequest, VerifyResponse,
};

fn main() -> ExitCode {
    match run() {
        Ok(response) => {
            let passed = response.outcomes.iter().all(VerifyOutcome::passed);
            print_response(&response);

            if passed { ExitCode::SUCCESS } else { ExitCode::FAILURE }
        },
        Err(error) => {
            print_response(&VerifyResponse {
                outcomes: vec![VerifyOutcome::Error {
                    message: error.to_string(),
                }],
            });
            ExitCode::FAILURE
        },
    }
}

fn run() -> color_eyre::Result<VerifyResponse> {
    let request_path =
        env::args().nth(1).unwrap_or_else(|| "/verify/request.json".to_owned());
    let request: VerifyRequest =
        serde_json::from_slice(&fs_err::read(request_path)?)?;

    let mut outcomes = Vec::new();

    for expectation in &request.filesystem {
        outcomes
            .push(verify_filesystem_expectation(&request.mounts, expectation));
    }

    for expectation in &request.history {
        outcomes
            .extend(verify_history_expectation(&request.mounts, expectation));
    }

    Ok(VerifyResponse { outcomes })
}

fn verify_filesystem_expectation(
    mounts: &BTreeMap<String, String>,
    expectation: &FilesystemExpectation,
) -> VerifyOutcome {
    let Ok(path) = resolve_path(mounts, &expectation.mount, &expectation.path)
    else {
        return VerifyOutcome::Error {
            message: format!("unknown verifier mount {:?}", expectation.mount),
        };
    };

    if !expectation.exists {
        return if path.exists() {
            VerifyOutcome::UnexpectedPresent { path }
        } else {
            VerifyOutcome::Ok { path }
        };
    }

    if !path.exists() {
        return VerifyOutcome::NotPresent { path };
    }

    if let Some(expected) = &expectation.checksum {
        match get_path_checksum(&path) {
            Ok(actual) if actual.eq_ignore_ascii_case(expected) => {
                VerifyOutcome::Ok { path }
            },
            Ok(actual) => {
                VerifyOutcome::ChecksumMismatch {
                    path,
                    expected: expected.to_ascii_uppercase(),
                    actual,
                }
            },
            Err(error) => {
                VerifyOutcome::Error {
                    message: format!("failed to checksum {path}: {error}"),
                }
            },
        }
    } else {
        VerifyOutcome::Ok { path }
    }
}

fn verify_history_expectation(
    mounts: &BTreeMap<String, String>,
    expectation: &HistoryExpectation,
) -> Vec<VerifyOutcome> {
    let Ok(path) = resolve_path(mounts, &expectation.mount, &expectation.path)
    else {
        return vec![VerifyOutcome::Error {
            message: format!("unknown verifier mount {:?}", expectation.mount),
        }];
    };

    let mut history =
        History::<Action, ActionRecordMetadata>::new(path.clone());
    if let Err(error) = history.load() {
        return vec![VerifyOutcome::HistoryRecordMissing {
            path,
            record: expectation.record,
            message: error.to_string(),
        }];
    }

    let Some(record) = history.records().get(expectation.record) else {
        return vec![VerifyOutcome::HistoryRecordMissing {
            path,
            record: expectation.record,
            message: format!(
                "history contains {} records",
                history.records().len()
            ),
        }];
    };

    let actual = record.iter().map(action_name).collect::<Vec<_>>();
    let mut outcomes = Vec::new();

    for expected in &expectation.contains_actions {
        if actual.contains(expected) {
            outcomes.push(VerifyOutcome::Ok { path: path.clone() });
        } else {
            outcomes.push(VerifyOutcome::HistoryActionMissing {
                path: path.clone(),
                record: expectation.record,
                action: *expected,
                actual: actual.clone(),
            });
        }
    }

    for unexpected in &expectation.does_not_contain_actions {
        if actual.contains(unexpected) {
            outcomes.push(VerifyOutcome::HistoryActionUnexpected {
                path: path.clone(),
                record: expectation.record,
                action: *unexpected,
                actual: actual.clone(),
            });
        } else {
            outcomes.push(VerifyOutcome::Ok { path: path.clone() });
        }
    }

    outcomes
}

fn resolve_path(
    mounts: &BTreeMap<String, String>,
    mount: &str,
    relative_path: &str,
) -> color_eyre::Result<Utf8PathBuf> {
    let base = mounts
        .get(mount)
        .ok_or_else(|| color_eyre::eyre::eyre!("unknown mount {mount:?}"))?;

    if relative_path.is_empty() {
        return Ok(Utf8PathBuf::from(base));
    }

    Ok(Utf8Path::new(base).join(relative_path))
}

fn action_name(action: &Action) -> ActionName {
    match action {
        Action::MoveFile { .. } => ActionName::MoveFile,
        Action::CopyFile { .. } => ActionName::CopyFile,
        Action::RemoveFile(_) => ActionName::RemoveFile,
        Action::MakeDir(_) => ActionName::MakeDir,
        Action::RemoveDir(_) => ActionName::RemoveDir,
    }
}

fn print_response(response: &VerifyResponse) {
    match serde_json::to_string(response) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("failed to serialize verifier response: {error}");
        },
    }
}
