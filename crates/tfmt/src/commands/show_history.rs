use color_eyre::Result;
use tfmttools_core::history::History;
use tfmttools_core::util::Utf8PathExt;

use crate::cli::TFMTOptions;
use crate::history::{HistoryFormatter, HistoryPrefix};

pub fn show_history(app_options: &TFMTOptions) -> Result<()> {
    let formatter =
        HistoryFormatter::new().with_prefix(HistoryPrefix::Ordered(')'));

    let path = app_options.history_file_path()?;
    let history = History::open_read_only(path.as_path().to_owned())?;
    if let Some(report) =
        crate::history::execution::interruption_report(&history)?
    {
        println!("{report}");
    }
    if path.as_path().exists() {
        println!("{}", formatter.format_history(&history));
    } else {
        println!("There is no history.");
    }
    Ok(())
}
