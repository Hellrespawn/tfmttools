use camino::Utf8Path;
use color_eyre::Result;
use color_eyre::eyre::eyre;

use crate::outcome::ReportArtifacts;
use crate::ReportEnvelope;

const REPORT_TEMPLATE: &str =
    include_str!("../assets/report/report-template.html");
const REPORT_CSS: &str = include_str!("../assets/report/report.css");
const REPORT_VIEWER_JS: &str = include_str!("../assets/report/viewer.js");
const REPORT_VENDOR_JS: &str =
    include_str!("../assets/report/htm-preact-standalone.mjs");

const CSS_PLACEHOLDER: &str = "{{REPORT_CSS}}";
const VENDOR_PLACEHOLDER: &str = "{{REPORT_VENDOR_MODULE_JSON}}";
const VIEWER_PLACEHOLDER: &str = "{{REPORT_VIEWER_MODULE_JSON}}";
const REPORT_JSON_FILE_PLACEHOLDER: &str = "{{REPORT_JSON_FILE_NAME_JSON}}";

pub fn write_report(
    report_dir: &Utf8Path,
    report: ReportEnvelope,
) -> Result<()> {
    fs_err::create_dir_all(report_dir)?;

    let artifacts = report_artifacts(&report);
    let report = report.with_artifacts(artifacts.clone());

    let report_json = serde_json::to_string_pretty(&report)?;
    fs_err::write(report_dir.join(artifacts.report_json()), report_json)?;

    let report_html = render_report_html(artifacts.report_json())?;
    fs_err::write(report_dir.join(artifacts.report_html()), report_html)?;

    Ok(())
}

fn render_report_html(report_json_file_name: &str) -> Result<String> {
    assert_placeholder(REPORT_TEMPLATE, CSS_PLACEHOLDER)?;
    assert_placeholder(REPORT_TEMPLATE, VENDOR_PLACEHOLDER)?;
    assert_placeholder(REPORT_TEMPLATE, VIEWER_PLACEHOLDER)?;
    assert_placeholder(REPORT_TEMPLATE, REPORT_JSON_FILE_PLACEHOLDER)?;

    let vendor_json = serde_json::to_string(REPORT_VENDOR_JS)?;
    let viewer_json = serde_json::to_string(REPORT_VIEWER_JS)?;
    let report_json_file_name_json =
        serde_json::to_string(report_json_file_name)?;

    let rendered = REPORT_TEMPLATE
        .replace(CSS_PLACEHOLDER, REPORT_CSS)
        .replace(VENDOR_PLACEHOLDER, &vendor_json)
        .replace(VIEWER_PLACEHOLDER, &viewer_json)
        .replace(REPORT_JSON_FILE_PLACEHOLDER, &report_json_file_name_json);

    if rendered.contains(CSS_PLACEHOLDER)
        || rendered.contains(VENDOR_PLACEHOLDER)
        || rendered.contains(VIEWER_PLACEHOLDER)
        || rendered.contains(REPORT_JSON_FILE_PLACEHOLDER)
    {
        return Err(eyre!("report template contains unreplaced placeholders"));
    }

    Ok(rendered)
}

fn assert_placeholder(template: &str, placeholder: &str) -> Result<()> {
    if template.contains(placeholder) {
        Ok(())
    } else {
        Err(eyre!("report template is missing placeholder {placeholder}"))
    }
}

fn report_artifacts(report: &ReportEnvelope) -> ReportArtifacts {
    let _ = report;
    ReportArtifacts::default()
}
