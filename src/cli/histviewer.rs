use crate::cli::config::HISTORY_NAME;
use crate::cli::Config;
use clap::Parser;
use color_eyre::Result;
use file_history::History;
use std::path::PathBuf;

#[derive(Parser, Debug, PartialEq)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    config_dir: Option<PathBuf>,
}

/// Entry point for histviewer
pub fn histviewer() -> Result<()> {
    let args = Args::parse();

    let config = if let Some(config_dir) = args.config_dir {
        Config::new(&config_dir)?
    } else {
        Config::default()?
    };

    let history = History::load(config.path(), HISTORY_NAME)?;

    println!("{history}");

    Ok(())
}
