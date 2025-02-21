use color_eyre::Result;
use tfmttools_history_core::History;

use crate::config::paths::AppPaths;
use crate::history::{HistoryFormatter, HistoryPrefix, load_history};
use crate::ui::ConfirmationPrompt;

pub fn clear_history(app_paths: &AppPaths, dry_run: bool) -> Result<()> {
    let path = &app_paths.history_file();

    if path.is_file() {
        let (mut history, _) = load_history(path)?;

        println!("Showing history from: {path}");

        let formatter =
            HistoryFormatter::new().with_prefix(HistoryPrefix::Ordered(')'));

        println!("{}", formatter.format_history(&history)?);

        let confirmation =
            ConfirmationPrompt::new("Clear history?").prompt()?;

        if confirmation {
            if !dry_run {
                history.remove()?;
            }

            println!("Removed history file at: {path}");
        } else {
            println!("Aborting.");
        }
    } else if path.exists() {
        eprintln!("History file path exists, but is not a file: {path}");
    } else {
        eprintln!("There is no history file at: {path}");
    }

    Ok(())
}
