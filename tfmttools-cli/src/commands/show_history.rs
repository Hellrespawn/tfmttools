use color_eyre::Result;
use tfmttools_core::history::LoadActionHistoryResult;

use crate::config::paths::AppPaths;
use crate::history::{
    HistoryFormat, HistoryFormatter, HistoryPrefix, load_history,
};

pub fn show_history(app_paths: &AppPaths, verbosity: u8) -> Result<()> {
    let formatter = get_history_formatter(verbosity);

    let load_history_result = load_history(&app_paths.history_file())?;

    match load_history_result {
        LoadActionHistoryResult::Loaded(history) => {
            println!("{}", formatter.format_history(&history));
        },
        LoadActionHistoryResult::New(_) => {
            println!("There is no history.");
        },
    }

    Ok(())
}

fn get_history_formatter(verbosity: u8) -> HistoryFormatter {
    let formatter =
        HistoryFormatter::new().with_prefix(HistoryPrefix::Ordered(')'));

    if verbosity > 0 {
        formatter.with_format(HistoryFormat::Verbose)
    } else {
        formatter
    }
}
