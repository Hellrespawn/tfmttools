use color_eyre::Result;
use tfmttools_core::util::FSMode;
use tfmttools_history::LoadHistoryResult;

use crate::cli::TFMTOptions;
use crate::history::{HistoryFormatter, HistoryPrefix, load_history};
use crate::ui::ConfirmationPrompt;

pub fn clear_history(app_options: &TFMTOptions) -> Result<()> {
    let path = &app_options.history_file_path()?;

    let (mut history, load_history_result) = load_history(path)?;

    if matches!(load_history_result, LoadHistoryResult::New) {
        println!("There is no history file to clear.");
    } else {
        println!("Showing history from: {path}\n");

        let formatter =
            HistoryFormatter::new().with_prefix(HistoryPrefix::Ordered(')'));

        println!("{}", formatter.format_history(&mut history)?);

        let confirmation =
            ConfirmationPrompt::new("Remove history file?").prompt()?;

        if confirmation {
            if matches!(app_options.fs_mode(), FSMode::Default) {
                history.remove()?;
            }

            println!("Removed history file at: {path}");
        } else {
            println!("Aborting.");
        }
    }

    Ok(())
}
