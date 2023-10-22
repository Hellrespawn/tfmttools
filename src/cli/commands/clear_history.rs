use crate::cli::config::{HISTORY_NAME, PREVIEW_PREFIX};
use crate::cli::Config;
use color_eyre::Result;
use file_history::History;

pub(crate) fn clear_history(preview: bool, config: &Config) -> Result<()> {
    if preview {
        let mut history = History::load(config.path(), HISTORY_NAME)?;
        history.clear()?;
    }

    let pp = if preview { PREVIEW_PREFIX } else { "" };
    println!("{pp}Cleared history.");
    Ok(())
}
