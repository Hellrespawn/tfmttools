use color_eyre::Result;
use color_eyre::eyre::bail;
use tfmttools_core::history::{AttemptId, AttemptOutcome};
use tfmttools_core::util::FSMode;

use crate::cli::TFMTOptions;
use crate::history::execution::interruption_report;
use crate::history::load_history;

pub fn resolve_history(
    options: &TFMTOptions,
    attempt: i64,
    outcome: AttemptOutcome,
) -> Result<()> {
    if matches!(options.fs_mode(), FSMode::DryRun) {
        bail!("History resolution requires an explicit write; omit --dry-run");
    }
    let (mut history, _) = load_history(&options.history_file_path()?)?;
    if let Some(report) = interruption_report(&history)? {
        println!("{report}");
    }
    let record = history.resolve_attempt(AttemptId(attempt), outcome)?;
    println!(
        "Resolved attempt {attempt} as {outcome:?}. History confirms {} actions, {} currently applied. No files were inspected or changed. Remove unneeded artifacts manually.",
        record.len(),
        record.applied_count()
    );
    Ok(())
}
