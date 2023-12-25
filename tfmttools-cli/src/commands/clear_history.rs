use color_eyre::Result;
use fs_err as fs;

use super::Command;
use crate::config::{Config, DRY_RUN_PREFIX};

#[derive(Debug)]
pub struct ClearHistory;

impl Command for ClearHistory {
    fn run(&self, config: &Config) -> Result<()> {
        let path = &config.history_file();

        if path.is_file() {
            if config.dry_run() {
                print!("{DRY_RUN_PREFIX}");
            } else {
                fs::remove_file(path)?;
            }

            println!("Removed history file at: {path}");
        } else if path.exists() {
            eprintln!("History file path exists, but is not a file: {path}");
        } else {
            eprintln!("There is no history file at: {path}");
        }

        Ok(())
    }
}
