use color_eyre::Result;
use tfmttools_fs::FsHandler;

use crate::config::paths::AppPaths;
use crate::history::{HistoryFormatter, HistoryPrefix, load_history};
use crate::ui::ConfirmationPrompt;

pub fn clear_history(
    app_paths: &AppPaths,
    fs_handler: &FsHandler,
) -> Result<()> {
    let path = &app_paths.history_file();

    if path.is_file() {
        let result = load_history(path)?;

        let history = result.unwrap();

        println!("Showing history from: {path}");

        let formatter =
            HistoryFormatter::new().with_prefix(HistoryPrefix::Ordered(')'));

        println!("{}", formatter.format_history(&history));

        let confirmation =
            ConfirmationPrompt::new("Clear history?").prompt()?;

        if confirmation {
            fs_handler.remove_file(path)?;

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
