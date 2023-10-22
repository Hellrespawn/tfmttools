use crate::cli::config::{DRY_RUN_PREFIX, HISTORY_NAME};
use crate::cli::Config;
use color_eyre::Result;
use file_history::History;

pub(crate) fn clear_history(config: &Config) -> Result<()> {
    if config.dry_run() {
        let mut history = History::load(config.config_dir(), HISTORY_NAME)?;
        history.clear()?;
    }

    let prefix = if config.dry_run() { DRY_RUN_PREFIX } else { "" };

    println!("{prefix}Cleared history.");

    Ok(())
}
