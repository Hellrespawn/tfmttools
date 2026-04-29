use std::collections::BTreeMap;
use std::process::{ExitStatus, Output};

use camino::Utf8PathBuf;
use serde_json::Value;
use tfmttools_test_harness::{
    CaseOutcome, CliCaseDetails, CliRunDetails, CommandOutcome,
    ExpectationOutcome, ExpectationVerification, ExpectationsOutcome,
    ReportEnvelope, ReportFilters, RunFailure, Runner, RunnerDetails,
    SourceMetadata, Status, StepOutcome, write_report,
};

#[test]
fn writes_static_report_viewer_with_json_loader() {
    let fixture = fs_err::read_to_string(
        Utf8PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/report/sample-report.json"),
    )
    .expect("fixture should load");
    assert!(fixture.contains("\"schema_version\": 1"));

    let report_dir = temp_report_dir();
    let report = ReportEnvelope::new(
        Runner::Cli,
        "2026-04-24T12:00:00.000Z".to_owned(),
        "2026-04-24T12:00:01.000Z".to_owned(),
        1,
        vec!["cargo".to_owned(), "test".to_owned()],
        ReportFilters::new(None, Vec::new(), false),
        BTreeMap::new(),
        Some(report_dir.clone()),
        Vec::new(),
        Some(RunFailure::new("fixture")),
        SourceMetadata::default(),
        RunnerDetails::Cli(CliRunDetails::default()),
    )
    .with_status(Status::Skipped);

    write_report(&report_dir, report).expect("report should write");

    let report_html_path = report_dir.join("cli-2026-04-24T12-00-00.000Z.html");
    let report_json_path = report_dir.join("cli-2026-04-24T12-00-00.000Z.json");
    let report_html = fs_err::read_to_string(report_html_path).unwrap();
    let report_json = fs_err::read_to_string(report_json_path).unwrap();
    let report_json: Value = serde_json::from_str(&report_json).unwrap();

    assert!(report_html.contains("__tfmtReportJsonFileName"));
    assert!(
        report_html.contains("cli-2026-04-24T12-00-00.000Z.json"),
        "html should load the timestamped json report"
    );
    assert!(report_html.contains("unsupported report schema version"));
    assert!(report_html.contains("await fetch(reportJsonFileName"));
    assert_eq!(report_json["schema_version"], 1);
    assert_eq!(
        report_json["artifacts"]["report_html"],
        "cli-2026-04-24T12-00-00.000Z.html"
    );
    assert_eq!(
        report_json["artifacts"]["report_json"],
        "cli-2026-04-24T12-00-00.000Z.json"
    );
}

#[test]
fn writes_tag_expectation_verifications_to_report_json() {
    let report_dir = temp_report_dir();
    let audio_path = Utf8PathBuf::from("/tmp/input/song.mp3");
    let report = ReportEnvelope::new(
        Runner::Cli,
        "2026-04-24T12:00:00.000Z".to_owned(),
        "2026-04-24T12:00:01.000Z".to_owned(),
        1,
        vec!["cargo".to_owned(), "test".to_owned()],
        ReportFilters::new(None, Vec::new(), false),
        BTreeMap::new(),
        Some(report_dir.clone()),
        vec![CaseOutcome::new(
            "tag.case.json".to_owned(),
            "tag verification".to_owned(),
            1,
            vec![StepOutcome::new(
                "initial".to_owned(),
                1,
                Some(CommandOutcome::with_expected_exit_code(
                    vec!["validate".to_owned(), "check".to_owned()],
                    &successful_output(),
                    0,
                )),
                ExpectationsOutcome::new(None, vec![ExpectationOutcome::Ok {
                    path: audio_path,
                    verifications: vec![ExpectationVerification::TagValue {
                        key: "TrackTitle".to_owned(),
                        expected: "Nemo".to_owned(),
                        actual: "Nemo".to_owned(),
                    }],
                }]),
            )],
            Some(CliCaseDetails::new(Utf8PathBuf::from("/tmp"))),
        )],
        None,
        SourceMetadata::default(),
        RunnerDetails::Cli(CliRunDetails::default()),
    );

    write_report(&report_dir, report).expect("report should write");

    let report_json_path = report_dir.join("cli-2026-04-24T12-00-00.000Z.json");
    let report_json = fs_err::read_to_string(report_json_path).unwrap();
    let report_json: Value = serde_json::from_str(&report_json).unwrap();
    let step = &report_json["cases"][0]["steps"][0];
    let tag_value = &step["expectations_outcome"]["outcomes"][0]["ok"]["verifications"]
        [0]["tag_value"];

    assert_eq!(step["command_outcome"]["expected_exit_code"], 0);
    assert_eq!(tag_value["key"], "TrackTitle");
    assert_eq!(tag_value["expected"], "Nemo");
    assert_eq!(tag_value["actual"], "Nemo");
}

#[cfg(unix)]
fn successful_output() -> Output {
    use std::os::unix::process::ExitStatusExt;

    Output {
        status: ExitStatus::from_raw(0),
        stdout: Vec::new(),
        stderr: Vec::new(),
    }
}

fn temp_report_dir() -> Utf8PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "tfmttools-test-harness-{}-{unique}",
        std::process::id()
    ));
    let dir = Utf8PathBuf::from_path_buf(dir).expect("utf8 temp dir");
    if dir.exists() {
        fs_err::remove_dir_all(&dir).unwrap();
    }
    fs_err::create_dir_all(&dir).unwrap();
    dir
}
