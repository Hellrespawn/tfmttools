mod formatter;

use camino::Utf8Path;
use color_eyre::Result;
use tfmttools_core::history::{ActionHistory, LoadActionHistoryResult};
use tracing::debug;

pub use self::formatter::{HistoryFormat, HistoryFormatter, HistoryPrefix};

pub fn load_history(path: &Utf8Path) -> Result<LoadActionHistoryResult> {
    let result = ActionHistory::load(path)?;

    if let LoadActionHistoryResult::Loaded(history) = &result {
        debug!(
            "Loaded history:\n{}",
            HistoryFormatter::new()
                .with_format(HistoryFormat::Verbose)
                .format_history(history)
        );
    }

    Ok(result)
}
