mod formatter;

use camino::Utf8Path;
use color_eyre::Result;
pub use formatter::{HistoryFormat, HistoryFormatter, HistoryPrefix};
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionRecordMetadata, LoadActionHistoryResult};
use tfmttools_history_core::History;
use tfmttools_history_serde::SerdeHistory;
use tracing::debug;

pub fn load_history(
    path: &Utf8Path,
) -> Result<(
    // TODO Learn what this means
    impl History<Action, ActionRecordMetadata> + use<>,
    LoadActionHistoryResult,
)> {
    let (mut history, result) = SerdeHistory::load(path)?;

    if let LoadActionHistoryResult::Loaded = &result {
        debug!(
            "Loaded history:\n{}",
            HistoryFormatter::new()
                .with_format(HistoryFormat::Verbose)
                .format_history(&mut history)?
        );
    }

    Ok((history, result))
}
