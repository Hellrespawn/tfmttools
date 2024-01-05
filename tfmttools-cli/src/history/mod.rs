mod formatter;

use color_eyre::Result;
use tfmttools_core::history::{ActionHistory, LoadActionHistoryResult};
use tracing::debug;

pub use self::formatter::{HistoryFormat, HistoryFormatter};
use crate::config::Config;

pub fn load_history(config: &Config) -> Result<LoadActionHistoryResult> {
    let result = ActionHistory::load(&config.history_file())?;

    if let LoadActionHistoryResult::Loaded(history) = &result {
        debug!(
            "Loaded history:\n{}",
            HistoryFormatter::verbose().format(history)
        );
    }

    Ok(result)
}
