use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use tfmttools_test_harness::{
    CliRunDetails, ReportEnvelope, ReportFilters, RunFailure, Runner,
    RunnerDetails, SourceMetadata, Status, write_report,
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

    let report_html =
        fs_err::read_to_string(report_dir.join("report.html")).unwrap();
    let report_json =
        fs_err::read_to_string(report_dir.join("report.json")).unwrap();

    assert!(report_html.contains("__tfmtReportJsonFileName"));
    assert!(report_html.contains("unsupported report schema version"));
    assert!(report_html.contains("await fetch(reportJsonFileName"));
    assert!(report_json.contains("\"schema_version\": 1"));
}

fn temp_report_dir() -> Utf8PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "tfmttools-test-harness-{}",
        std::process::id()
    ));
    let dir = Utf8PathBuf::from_path_buf(dir).expect("utf8 temp dir");
    if dir.exists() {
        fs_err::remove_dir_all(&dir).unwrap();
    }
    fs_err::create_dir_all(&dir).unwrap();
    dir
}
