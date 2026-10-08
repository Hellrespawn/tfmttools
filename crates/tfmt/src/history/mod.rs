pub(crate) mod execution;
mod formatter;

use color_eyre::Result;
pub use formatter::{HistoryFormat, HistoryFormatter, HistoryPrefix};
use tfmttools_core::history::{History, LoadHistoryResult};
use tfmttools_core::util::{Utf8File, Utf8PathExt};
use tracing::debug;

pub fn load_history(path: &Utf8File) -> Result<(History, LoadHistoryResult)> {
    let mut history = History::new(path.as_path().to_owned());

    let result = history.load()?;

    if let LoadHistoryResult::Loaded = &result {
        debug!(
            "Loaded history:\n{}",
            HistoryFormatter::new()
                .with_format(HistoryFormat::Verbose)
                .format_history(&history)?
        );
    }

    Ok((history, result))
}

pub(crate) fn load_history_for_mode(
    path: &Utf8File,
    mode: tfmttools_core::util::FSMode,
) -> Result<(History, LoadHistoryResult)> {
    if matches!(mode, tfmttools_core::util::FSMode::DryRun) {
        let result = if path.as_path().exists() {
            LoadHistoryResult::Loaded
        } else {
            LoadHistoryResult::New
        };
        Ok((History::open_read_only(path.as_path().to_owned())?, result))
    } else {
        load_history(path)
    }
}
