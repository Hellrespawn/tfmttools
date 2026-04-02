use color_eyre::Result;
use tfmttools_history::LoadHistoryResult;

use crate::history::{
    HistoryFormat, HistoryFormatter, HistoryPrefix, load_history,
};
use crate::options::TFMTOptions;

pub fn show_history(app_options: &TFMTOptions) -> Result<()> {
    let formatter = get_history_formatter(app_options.verbosity());

    let (mut history, load_history_result) =
        load_history(&app_options.history_file_path()?)?;

    match load_history_result {
        LoadHistoryResult::Loaded => {
            println!("{}", formatter.format_history(&mut history)?);
        },
        LoadHistoryResult::New => {
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
