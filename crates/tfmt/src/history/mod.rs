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
