use std::collections::BTreeMap;
use std::env;
use std::process::ExitCode;

use camino::{Utf8Path, Utf8PathBuf};
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_fs::get_path_checksum;
use tfmttools_history::History;
use tfmttools_test_container::protocol::{
    ActionName, FilesystemExpectation, HistoryExpectation, VerifyCode,
    VerifyOutcome, VerifyRequest, VerifyResponse,
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
                mount_aliases: BTreeMap::new(),
                outcomes: vec![VerifyOutcome::failure(
                    VerifyCode::VerifierError,
                    None,
                    error.to_string(),
                )],
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

    Ok(VerifyResponse { mount_aliases: request.mounts, outcomes })
}

fn verify_filesystem_expectation(
    mounts: &BTreeMap<String, String>,
    expectation: &FilesystemExpectation,
) -> VerifyOutcome {
    let Ok(path) = resolve_path(mounts, &expectation.mount, &expectation.path)
    else {
        return VerifyOutcome::failure(
            VerifyCode::MountUnknown,
            None,
            format!("unknown verifier mount {:?}", expectation.mount),
        );
    };

    if !expectation.exists {
        return if path.exists() {
            let mut outcome = VerifyOutcome::failure(
                VerifyCode::PathUnexpected,
                Some(path),
                "path exists but was expected to be absent",
            );
            outcome.exists = Some(true);
            outcome
        } else {
            let mut outcome = VerifyOutcome::ok(path, "path is absent as expected");
            outcome.exists = Some(false);
            outcome
        };
    }

    if !path.exists() {
        let mut outcome = VerifyOutcome::failure(
            VerifyCode::PathMissing,
            Some(path),
            "path is missing",
        );
        outcome.exists = Some(false);
        return outcome;
    }

    if let Some(expected) = &expectation.checksum {
        match get_path_checksum(&path) {
            Ok(actual) if actual.eq_ignore_ascii_case(expected) => {
                let mut outcome =
                    VerifyOutcome::ok(path, "checksum matched expected value");
                outcome.expected_checksum = Some(expected.to_ascii_uppercase());
                outcome.actual_checksum = Some(actual);
                outcome
            },
            Ok(actual) => {
                let mut outcome = VerifyOutcome::failure(
                    VerifyCode::ChecksumMismatch,
                    Some(path),
                    "checksum mismatch",
                );
                outcome.expected_checksum =
                    Some(expected.to_ascii_uppercase());
                outcome.actual_checksum = Some(actual);
                outcome
            },
            Err(error) => VerifyOutcome::failure(
                VerifyCode::VerifierError,
                Some(path),
                format!("failed to checksum path: {error}"),
            ),
        }
    } else {
        let mut outcome = VerifyOutcome::ok(path, "path exists");
        outcome.exists = Some(true);
        outcome
    }
}

fn verify_history_expectation(
    mounts: &BTreeMap<String, String>,
    expectation: &HistoryExpectation,
) -> Vec<VerifyOutcome> {
    let Ok(path) = resolve_path(mounts, &expectation.mount, &expectation.path)
    else {
        return vec![VerifyOutcome::failure(
            VerifyCode::MountUnknown,
            None,
            format!("unknown verifier mount {:?}", expectation.mount),
        )];
    };

    let mut history =
        History::<Action, ActionRecordMetadata>::new(path.clone());
    if let Err(error) = history.load() {
        let mut outcome = VerifyOutcome::failure(
            VerifyCode::HistoryLoadFailed,
            Some(path),
            format!("failed to load history: {error}"),
        );
        outcome.history_record = Some(expectation.record);
        return vec![outcome];
    }

    let Some(record) = history.records().get(expectation.record) else {
        let mut outcome = VerifyOutcome::failure(
            VerifyCode::HistoryRecordMissing,
            Some(path),
            format!("history contains {} records", history.records().len()),
        );
        outcome.history_record = Some(expectation.record);
        return vec![outcome];
    };

    let actual = record.iter().map(action_name).collect::<Vec<_>>();
    let mut outcomes = Vec::new();

    for expected in &expectation.contains_actions {
        if actual.contains(expected) {
            let mut outcome = VerifyOutcome::ok(
                path.clone(),
                format!("history record contains action {expected:?}"),
            );
            outcome.history_record = Some(expectation.record);
            outcome.action = Some(*expected);
            outcome.actual_actions = actual.clone();
            outcomes.push(outcome);
        } else {
            let mut outcome = VerifyOutcome::failure(
                VerifyCode::HistoryActionMissing,
                Some(path.clone()),
                format!("history record is missing action {expected:?}"),
            );
            outcome.history_record = Some(expectation.record);
            outcome.action = Some(*expected);
            outcome.actual_actions = actual.clone();
            outcomes.push(outcome);
        }
    }

    for unexpected in &expectation.does_not_contain_actions {
        if actual.contains(unexpected) {
            let mut outcome = VerifyOutcome::failure(
                VerifyCode::HistoryActionUnexpected,
                Some(path.clone()),
                format!("history record unexpectedly contains action {unexpected:?}"),
            );
            outcome.history_record = Some(expectation.record);
            outcome.action = Some(*unexpected);
            outcome.actual_actions = actual.clone();
            outcomes.push(outcome);
        } else {
            let mut outcome = VerifyOutcome::ok(
                path.clone(),
                format!("history record does not contain action {unexpected:?}"),
            );
            outcome.history_record = Some(expectation.record);
            outcome.action = Some(*unexpected);
            outcome.actual_actions = actual.clone();
            outcomes.push(outcome);
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
