use color_eyre::Result;
use fs_err as fs;

use crate::config::{Config, DRY_RUN_PREFIX};

pub(crate) fn clear_history(config: &Config) -> Result<()> {
    let path = config.history_file();

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
