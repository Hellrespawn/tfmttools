use color_eyre::Result;

use super::Command;
use crate::config::Config;
use crate::history::{load_history, HistoryFormatter, HistoryPrefix};
use crate::ui::ConfirmationPrompt;

#[derive(Debug)]
pub struct ClearHistoryCommand;

impl Command for ClearHistoryCommand {
    fn run(&self, config: &Config) -> Result<()> {
        let path = &config.history_file();

        if path.is_file() {
            let result = load_history(config)?;

            let history = result.unwrap();

            println!("Showing history from: {path}");

            let formatter = HistoryFormatter::new()
                .with_prefix(HistoryPrefix::Ordered(')'));

            println!("{}", formatter.format_history(&history));

            let confirmation =
                ConfirmationPrompt::new("Clear history?").prompt()?;

            if confirmation {
                config.fs_handler().remove_file(path)?;

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
}
