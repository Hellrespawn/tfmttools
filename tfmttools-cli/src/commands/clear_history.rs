use color_eyre::Result;
use tfmttools_history_core::{History, LoadHistoryResult};

use crate::config::paths::AppPaths;
use crate::history::{HistoryFormatter, HistoryPrefix, load_history};
use crate::ui::ConfirmationPrompt;

pub fn clear_history(app_paths: &AppPaths, dry_run: bool) -> Result<()> {
    let path = &app_paths.history_file();

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
            if !dry_run {
                history.remove()?;
            }

            println!("Removed history file at: {path}");
        } else {
            println!("Aborting.");
        }
    }

    Ok(())
}
