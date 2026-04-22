use color_eyre::Result;
use tfmttools_history::LoadHistoryResult;

use crate::cli::TFMTOptions;
use crate::history::{
    HistoryFormat, HistoryFormatter, HistoryPrefix, load_history,
};

pub fn show_history(app_options: &TFMTOptions) -> Result<()> {
    let formatter =
        HistoryFormatter::new().with_prefix(HistoryPrefix::Ordered(')'));
    let formatter = if app_options.verbosity() > 0 {
        formatter.with_format(HistoryFormat::Verbose)
    } else {
        formatter
    };

    let (history, load_history_result) =
        load_history(&app_options.history_file_path()?)?;

    match load_history_result {
        LoadHistoryResult::Loaded => {
            println!("{}", formatter.format_history(&history)?);
        },
        LoadHistoryResult::New => {
            println!("There is no history.");
        },
    }

    Ok(())
}
