mod formatter;

use camino::Utf8Path;
use color_eyre::Result;
pub use formatter::{HistoryFormat, HistoryFormatter, HistoryPrefix};
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_history_core::{History, LoadHistoryResult};
use tfmttools_history_serde::SerdeHistory;
use tracing::debug;

pub fn load_history(
    path: &Utf8Path,
) -> Result<(
    // TODO Learn what this + use<> means exactly
    impl History<Action, ActionRecordMetadata> + use<>,
    LoadHistoryResult,
)> {
    let mut history = SerdeHistory::new(path.to_owned());

    let result = history.load()?;

    if let LoadHistoryResult::Loaded = &result {
        debug!(
            "Loaded history:\n{}",
            HistoryFormatter::new()
                .with_format(HistoryFormat::Verbose)
                .format_history(&mut history)?
        );
    }

    Ok((history, result))
}
