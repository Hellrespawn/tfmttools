mod formatter;

use color_eyre::Result;
pub use formatter::{HistoryFormat, HistoryFormatter, HistoryPrefix};
use tfmttools_core::action::Action;
use tfmttools_core::history::ActionRecordMetadata;
use tfmttools_core::util::{Utf8File, Utf8PathExt};
use tfmttools_history_core::{History, LoadHistoryResult};
use tfmttools_history_serde::SerdeHistory;
use tracing::debug;

pub fn load_history(
    path: &Utf8File,
) -> Result<(
    // By default, return-position impl Trait tries to capture all references
    // and lifetimes, including path. In this case, the compiler can't tell
    // path isn't actually captured, so we add an empty 'use<>' bound to
    // make it explicit.
    impl History<Action, ActionRecordMetadata> + use<>,
    LoadHistoryResult,
)> {
    let mut history = SerdeHistory::new(path.as_path().to_owned());

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
