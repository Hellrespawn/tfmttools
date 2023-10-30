use std::path::PathBuf;

use clap::Parser;
use color_eyre::Result;
use file_history::History;

use crate::cli::config::HISTORY_NAME;
use crate::cli::Config;

#[derive(Parser, Debug, PartialEq)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Sets a custom config file
    config_dir: Option<PathBuf>,
}

/// Entry point for histviewer
pub fn histviewer() -> Result<()> {
    let args = Args::parse();

    let config_dir = if let Some(path) = &args.config_dir {
        path.clone()
    } else {
        Config::default_path()?
    };

    let history = History::load(&config_dir, HISTORY_NAME)?;

    println!("{history}");

    Ok(())
}
