use crate::cli::Config;
use color_eyre::Result;
use file_history::History;

pub(crate) fn clear_history(preview: bool, config: &Config) -> Result<()> {
    if preview {
        let mut history = History::load(config.path(), Config::HISTORY_NAME)?;
        history.clear()?;
    }

    let pp = if preview { Config::PREVIEW_PREFIX } else { "" };
    println!("{pp}Cleared history.");
    Ok(())
}
